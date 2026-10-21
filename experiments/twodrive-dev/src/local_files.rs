//! Shared-engine private-cache bridge. No CFAPI, GUI, provider switch or root
//! registration. Native capabilities stay disabled until the adapter owns the
//! corresponding OS locks and recovery path.
use crate::model::{CloudBackend, MetadataEntry};
use crate::provider::{Progress, Provider};
use anyhow::{Context, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
    sync::Arc,
};
use twodrive_core::{CacheJournal, CacheSnapshot, CacheWriteLease, Database, FileState};

#[derive(Clone)]
pub struct LocalFileBridge {
    journal: CacheJournal,
}
impl LocalFileBridge {
    pub fn open(db: Database, private_cache: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            journal: CacheJournal::open(db, private_cache)?,
        })
    }
    pub fn journal(&self) -> &CacheJournal {
        &self.journal
    }
    pub fn status(&self, id: &str) -> anyhow::Result<CacheSnapshot> {
        self.journal.snapshot(id)
    }
    pub fn edit(&self, id: &str) -> anyhow::Result<CacheWriteLease> {
        self.journal.open_write(id)
    }

    /// Download by opaque object identity; recheck provider version after EOF.
    pub fn hydrate(&self, backend: &impl CloudBackend, remote_id: &str) -> anyhow::Result<String> {
        let remote = backend
            .get_metadata(remote_id)?
            .context("provider object missing")?;
        ensure!(
            !remote.is_dir && !remote.etag.is_empty(),
            "versioned file required"
        );
        let db = self.journal.database();
        db.upsert_metadata(&core_metadata(&remote))?;
        let record = db
            .get_by_cloud_remote_id(remote_id)?
            .context("local identity missing")?;
        ensure!(
            record.state == FileState::OnlineOnly && record.cache_path.is_none(),
            "existing cache must be recovered, not replaced"
        );
        let id = record.metadata.remote_id;
        let generation = db.download_generation(&id)?;
        ensure!(
            db.begin_hydration(&id, generation)?,
            "download claim refused"
        );
        let result = (|| {
            let mut temporary = tempfile::NamedTempFile::new_in(self.journal.cache_root())?;
            let size = backend.download_to(remote_id, temporary.as_file_mut(), &mut |_| Ok(()))?;
            ensure!(size == remote.size, "incomplete provider download");
            temporary.as_file().sync_all()?;
            let after = backend
                .get_metadata(remote_id)?
                .context("provider object disappeared")?;
            ensure!(
                after.remote_id == remote.remote_id
                    && after.path == remote.path
                    && after.etag == remote.etag
                    && after.size == remote.size,
                "provider version changed during download"
            );
            let digest = digest_file(temporary.as_file_mut())?;
            let (file, path) = temporary.keep()?;
            drop(file);
            self.journal
                .publish_download(&id, generation, &path, &digest, &remote.etag)?;
            Ok(id.clone())
        })();
        if result.is_err() {
            db.fail_hydration(&id, generation)?;
        }
        result
    }

    /// Provider's explicit conditional-write capability is mandatory; legacy
    /// CloudBackend default methods alone do not establish safe overwrites.
    pub fn upload(
        &self,
        provider: &impl Provider,
        id: &str,
        progress: Progress,
    ) -> anyhow::Result<bool> {
        ensure!(
            provider.capabilities().upload && provider.capabilities().conditional_upload,
            "conditional provider upload unavailable"
        );
        let mut lease = self.journal.begin_upload(id, digest_file)?;
        let record = lease.record().clone();
        match provider.upload_controlled(
            &record.metadata.path,
            record.cache_path.as_deref().context("missing cache")?,
            record.cloud_remote_id.as_deref(),
            Some(&record.metadata.etag),
            progress,
        ) {
            Ok(remote) => {
                let digest = digest_file(lease.file())?;
                lease.acknowledge(&core_metadata(&remote), &digest)
            }
            Err(error) => {
                if provider.is_conflict_error(&error) {
                    lease.reject_conflict()?;
                }
                // All other errors can be an accepted write with a lost reply.
                // Drop retains the durable intent; no blind retry or release.
                Err(error)
            }
        }
    }

    pub fn reconcile(&self, backend: &impl CloudBackend, id: &str) -> anyhow::Result<bool> {
        let record = self
            .journal
            .database()
            .get_by_remote_id(id)?
            .context("unknown local item")?;
        let remote_id = record
            .cloud_remote_id
            .as_deref()
            .context("unbound local item")?;
        let before = backend
            .get_metadata(remote_id)?
            .context("provider object missing")?;
        ensure!(
            !before.is_dir && !before.etag.is_empty(),
            "versioned provider file required"
        );
        let mut download = tempfile::tempfile_in(self.journal.cache_root())?;
        let size = backend.download_to(remote_id, &mut download, &mut |_| Ok(()))?;
        ensure!(size == before.size, "incomplete reconciliation download");
        let after = backend
            .get_metadata(remote_id)?
            .context("provider object disappeared")?;
        ensure!(
            after.remote_id == before.remote_id
                && !after.is_dir
                && after.path == before.path
                && after.etag == before.etag
                && after.size == before.size,
            "provider version changed during reconciliation"
        );
        let remote_digest = digest_file(&mut download)?;
        let mut local = self.journal.open_read(id)?;
        let local_digest = digest_reader(&mut local)?;
        drop(local);
        self.journal
            .reconcile_upload(id, &core_metadata(&after), &remote_digest, &local_digest)
    }

    /// Diagnostic engine eligibility; this never dehydrates or removes a file.
    /// The native adapter still reports release unsupported and requires fresh
    /// CFAPI evidence under its own exclusive oplock before any future action.
    pub fn engine_release_ready(&self, id: &str, generation: i64) -> anyhow::Result<bool> {
        let verified = self
            .journal
            .with_clean_cache(id, generation, |file, expected| {
                Ok(digest_file(file)? == expected)
            })?;
        if verified == Some(false) {
            self.journal.record_external_edit(id, generation)?;
        }
        Ok(verified.unwrap_or(false))
    }
}

fn digest_reader(reader: &mut impl Read) -> anyhow::Result<String> {
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65536];
    loop {
        let size = reader.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        hash.update(&buffer[..size]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
pub fn digest_file(file: &mut File) -> anyhow::Result<String> {
    file.seek(SeekFrom::Start(0))?;
    digest_reader(file)
}
fn core_metadata(entry: &MetadataEntry) -> twodrive_core::MetadataEntry {
    twodrive_core::MetadataEntry {
        remote_id: entry.remote_id.clone(),
        path: entry.path.clone(),
        parent_path: entry.parent_path.clone(),
        name: entry.name.clone(),
        is_dir: entry.is_dir,
        size: entry.size,
        modified_unix: entry.modified_unix,
        etag: entry.etag.clone(),
    }
}

/// Used by deterministic mock fixtures; not connected to desktop IPC.
pub fn no_progress() -> Progress {
    Arc::new(|_, _| Ok(()))
}

#[cfg(test)]
#[path = "local_files_tests.rs"]
mod tests;
