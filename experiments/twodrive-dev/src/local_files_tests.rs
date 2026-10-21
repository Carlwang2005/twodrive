use super::*;
use crate::model::DeltaResult;
use crate::provider::Capabilities;
use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
};

const OBJECT: &str = "opaque-object-not-a-path";
const REMOTE_PATH: &str = "/generated-中文.txt";
const INITIAL: &[u8] = b"generated initial provider content";
const EDITED: &[u8] = b"generated unsynchronized content survives complete-byte checks";
struct Remote {
    bytes: Vec<u8>,
    version: u64,
}
struct MockProvider {
    remote: Mutex<Remote>,
    mode: AtomicU8,
    conditional: bool,
}
impl MockProvider {
    fn new() -> Self {
        Self {
            remote: Mutex::new(Remote {
                bytes: INITIAL.to_vec(),
                version: 1,
            }),
            mode: AtomicU8::new(0),
            conditional: true,
        }
    }
    fn metadata(remote: &Remote) -> MetadataEntry {
        MetadataEntry::new_file(
            OBJECT,
            REMOTE_PATH,
            remote.bytes.len() as u64,
            remote.version as i64,
            format!("version-{}", remote.version),
        )
    }
    fn change(&self, bytes: &[u8]) {
        let mut r = self.remote.lock().unwrap();
        r.bytes = bytes.to_vec();
        r.version += 1;
    }
}
impl CloudBackend for MockProvider {
    fn is_conflict_error(&self, error: &anyhow::Error) -> bool {
        error.to_string().contains("fixture conflict")
    }
    fn list_all(&self) -> anyhow::Result<Vec<MetadataEntry>> {
        Ok(vec![Self::metadata(&self.remote.lock().unwrap())])
    }
    fn list_delta(&self, _: Option<&str>) -> anyhow::Result<DeltaResult> {
        Ok(DeltaResult {
            entries: self.list_all()?,
            deleted_remote_ids: vec![],
            delta_link: None,
        })
    }
    fn get_metadata(&self, id: &str) -> anyhow::Result<Option<MetadataEntry>> {
        ensure!(id == OBJECT, "opaque ID required, never a path");
        Ok(Some(Self::metadata(&self.remote.lock().unwrap())))
    }
    fn get_metadata_by_path(&self, path: &str) -> anyhow::Result<Option<MetadataEntry>> {
        ensure!(path == REMOTE_PATH, "unexpected fixture path");
        self.get_metadata(OBJECT)
    }
    fn download(&self, id: &str) -> anyhow::Result<Vec<u8>> {
        ensure!(id == OBJECT, "opaque download ID required");
        Ok(self.remote.lock().unwrap().bytes.clone())
    }
    fn download_to(
        &self,
        id: &str,
        writer: &mut dyn Write,
        progress: &mut dyn FnMut(u64) -> anyhow::Result<()>,
    ) -> anyhow::Result<u64> {
        let bytes = self.download(id)?;
        writer.write_all(&bytes)?;
        progress(bytes.len() as u64)?;
        if self.mode.load(Ordering::SeqCst) == 3 {
            self.mode.store(0, Ordering::SeqCst);
            self.change(INITIAL);
        }
        Ok(bytes.len() as u64)
    }
    fn upload(&self, _: &str, _: Vec<u8>) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("unconditional upload forbidden")
    }
    fn upload_with_etag(
        &self,
        _: &str,
        _: Vec<u8>,
        _: Option<&str>,
    ) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("use controlled conditional transfer")
    }
    fn upload_file_with_version(
        &self,
        path: &str,
        source: &Path,
        id: Option<&str>,
        version: Option<&str>,
        progress: &mut dyn FnMut(u64, u64) -> anyhow::Result<()>,
    ) -> anyhow::Result<MetadataEntry> {
        let result = self.upload_controlled(path, source, id, version, no_progress())?;
        progress(result.size, result.size)?;
        Ok(result)
    }
    fn create_folder(&self, _: &str) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("fixture mkdir disabled")
    }
    fn rename(&self, _: &str, _: &str) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("fixture rename disabled")
    }
    fn delete(&self, _: &str) -> anyhow::Result<()> {
        anyhow::bail!("fixture delete disabled")
    }
}
impl Provider for MockProvider {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            upload: true,
            conditional_upload: self.conditional,
            ..Default::default()
        }
    }
    fn list_directory(&self, _: &str) -> anyhow::Result<Vec<MetadataEntry>> {
        self.list_all()
    }
    fn upload_controlled(
        &self,
        path: &str,
        source: &Path,
        id: Option<&str>,
        version: Option<&str>,
        progress: Progress,
    ) -> anyhow::Result<MetadataEntry> {
        ensure!(
            path == REMOTE_PATH && id == Some(OBJECT),
            "identity/path boundary violated"
        );
        let bytes = fs::read(source)?;
        progress(0, bytes.len() as u64)?;
        if self.mode.load(Ordering::SeqCst) == 1 {
            anyhow::bail!("injected offline before commit");
        }
        let mut remote = self.remote.lock().unwrap();
        ensure!(
            version == Some(Self::metadata(&remote).etag.as_str()),
            "fixture conflict"
        );
        remote.bytes = bytes;
        remote.version += 1;
        let result = Self::metadata(&remote);
        drop(remote);
        progress(result.size, result.size)?;
        if self.mode.load(Ordering::SeqCst) == 2 {
            anyhow::bail!("injected accepted write with lost reply");
        }
        Ok(result)
    }
}

struct Fixture {
    _temp: tempfile::TempDir,
    db: Database,
    cache: PathBuf,
    bridge: LocalFileBridge,
}
fn fixture() -> Fixture {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let cache = root.join("cache");
    fs::create_dir(&cache).unwrap();
    let db = Database::new(root.join("fixture.sqlite3"));
    let bridge = LocalFileBridge::open(db.clone(), &cache).unwrap();
    Fixture {
        _temp: temp,
        db,
        cache,
        bridge,
    }
}
fn edit(bridge: &LocalFileBridge) {
    let mut file = bridge.edit(OBJECT).unwrap();
    file.set_len(0).unwrap();
    file.write_all(EDITED).unwrap();
    file.finish().unwrap();
}
fn bytes(bridge: &LocalFileBridge) -> Vec<u8> {
    let mut file = bridge.journal.open_read(OBJECT).unwrap();
    let mut bytes = vec![];
    file.read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn actual_engine_download_edit_conditional_upload_and_generation_ack() {
    let fixture = fixture();
    let provider = MockProvider::new();
    let bridge = fixture.bridge;
    assert_eq!(bridge.hydrate(&provider, OBJECT).unwrap(), OBJECT);
    assert_eq!(bytes(&bridge), INITIAL);
    assert!(bridge.engine_release_ready(OBJECT, 0).unwrap());
    edit(&bridge);
    assert_eq!(bridge.status(OBJECT).unwrap().state, FileState::Dirty);
    assert!(!bridge.engine_release_ready(OBJECT, 1).unwrap());
    assert!(bridge.upload(&provider, OBJECT, no_progress()).unwrap());
    assert_eq!(provider.download(OBJECT).unwrap(), EDITED);
    assert_eq!(bytes(&bridge), EDITED);
    assert_eq!(bridge.status(OBJECT).unwrap().acknowledged_generation, 1);
    assert!(bridge.engine_release_ready(OBJECT, 1).unwrap());
    assert!(!bridge.engine_release_ready(OBJECT, 0).unwrap());
}
#[test]
fn accepted_write_lost_reply_restarts_and_requires_verified_download() {
    let f = fixture();
    let provider = MockProvider::new();
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    edit(&f.bridge);
    provider.mode.store(2, Ordering::SeqCst);
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).is_err());
    assert_eq!(provider.download(OBJECT).unwrap(), EDITED);
    assert!(f.bridge.status(OBJECT).unwrap().uncertain_upload);
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).is_err());
    drop(f.bridge);
    let bridge = LocalFileBridge::open(f.db, &f.cache).unwrap();
    assert!(bridge.status(OBJECT).unwrap().uncertain_upload);
    assert!(bridge.reconcile(&provider, OBJECT).unwrap());
    assert_eq!(bridge.status(OBJECT).unwrap().state, FileState::Cached);
    assert!(bridge.engine_release_ready(OBJECT, 1).unwrap());
    assert_eq!(bytes(&bridge), EDITED);
}
#[test]
fn offline_retry_waits_for_unchanged_remote_version_proof() {
    let f = fixture();
    let provider = MockProvider::new();
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    edit(&f.bridge);
    provider.mode.store(1, Ordering::SeqCst);
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).is_err());
    assert!(f.bridge.status(OBJECT).unwrap().uncertain_upload);
    provider.mode.store(0, Ordering::SeqCst);
    assert!(!f.bridge.reconcile(&provider, OBJECT).unwrap());
    assert_eq!(f.bridge.status(OBJECT).unwrap().state, FileState::Dirty);
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).unwrap());
    assert_eq!(bytes(&f.bridge), EDITED);
}
#[test]
fn remote_conflict_preserves_local_bytes_and_blocks_release() {
    let f = fixture();
    let provider = MockProvider::new();
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    edit(&f.bridge);
    provider.change(b"different generated remote content");
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).is_err());
    assert_eq!(f.bridge.status(OBJECT).unwrap().state, FileState::Conflict);
    assert_eq!(bytes(&f.bridge), EDITED);
    assert!(!f.bridge.engine_release_ready(OBJECT, 1).unwrap());
}
#[test]
fn provider_version_change_during_hydration_is_not_published() {
    let f = fixture();
    let provider = MockProvider::new();
    provider.mode.store(3, Ordering::SeqCst);
    assert!(f.bridge.hydrate(&provider, OBJECT).is_err());
    let record = f.db.get_by_cloud_remote_id(OBJECT).unwrap().unwrap();
    assert_eq!(record.state, FileState::OnlineOnly);
    assert!(record.cache_path.is_none());
    assert!(f.bridge.status(OBJECT).is_err());
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    assert_eq!(bytes(&f.bridge), INITIAL);
}
#[test]
fn external_byte_change_invalidates_stale_cached_generation() {
    let f = fixture();
    let provider = MockProvider::new();
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    let record = f.db.get_by_remote_id(OBJECT).unwrap().unwrap();
    fs::write(record.cache_path.unwrap(), EDITED).unwrap();
    assert!(!f.bridge.engine_release_ready(OBJECT, 0).unwrap());
    let state = f.bridge.status(OBJECT).unwrap();
    assert_eq!(state.state, FileState::Dirty);
    assert_eq!(state.generation, 1);
    assert!(state.dirty_intent);
    assert_eq!(bytes(&f.bridge), EDITED);
}
#[test]
fn missing_conditional_capability_never_starts_upload() {
    let f = fixture();
    let mut provider = MockProvider::new();
    provider.conditional = false;
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    edit(&f.bridge);
    assert!(f.bridge.upload(&provider, OBJECT, no_progress()).is_err());
    assert!(f.bridge.status(OBJECT).unwrap().upload_generation.is_none());
    assert_eq!(provider.download(OBJECT).unwrap(), INITIAL);
}

#[test]
fn cancellation_after_provider_commit_is_reconciled_without_losing_bytes() {
    let f = fixture();
    let provider = MockProvider::new();
    f.bridge.hydrate(&provider, OBJECT).unwrap();
    edit(&f.bridge);
    let progress: Progress = Arc::new(|done, _| {
        ensure!(done == 0, "generated cancellation after commit");
        Ok(())
    });
    assert!(f.bridge.upload(&provider, OBJECT, progress).is_err());
    assert_eq!(provider.download(OBJECT).unwrap(), EDITED);
    assert!(f.bridge.status(OBJECT).unwrap().uncertain_upload);
    assert!(!f.bridge.engine_release_ready(OBJECT, 1).unwrap());
    assert!(f.bridge.reconcile(&provider, OBJECT).unwrap());
    assert_eq!(bytes(&f.bridge), EDITED);
}
