//! Opt-in generated-provider host using the actual engine cache bridge and OS
//! callbacks. No account/profile, network or existing sync root is accepted.
use crate::{
    local_files::{LocalFileBridge, digest_file},
    model::{CloudBackend, DeltaResult, MetadataEntry},
    provider::{Capabilities, Progress, Provider},
};
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};
use twodrive_core::{Database, FileState};
use twodrive_local_files::native::{self, HydrationSource, NativeObservation, Session};

const SIZE: usize = 65536;
fn objects() -> Vec<MetadataEntry> {
    [
        "generated-中文.bin".to_owned(),
        format!("generated-{}.bin", "long-".repeat(35)),
        "generated-offline.bin".to_owned(),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, name)| {
        MetadataEntry::new_file(
            format!("generated-object-{index}"),
            format!("/{name}"),
            SIZE as u64,
            1,
            "generated-version-1",
        )
    })
    .collect()
}
fn generated_bytes(index: usize) -> Vec<u8> {
    (0..SIZE).map(|i| ((i + index * 17) % 251) as u8).collect()
}
fn object(id: &str) -> anyhow::Result<(usize, MetadataEntry)> {
    objects()
        .into_iter()
        .enumerate()
        .find(|(_, m)| m.remote_id == id)
        .context("unknown generated object")
}
pub(crate) struct GeneratedProvider;
impl CloudBackend for GeneratedProvider {
    fn is_conflict_error(&self, _: &anyhow::Error) -> bool {
        false
    }
    fn upload_file_with_version(
        &self,
        _: &str,
        _: &Path,
        _: Option<&str>,
        _: Option<&str>,
        _: &mut dyn FnMut(u64, u64) -> anyhow::Result<()>,
    ) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("generated provider is read-only")
    }
    fn list_all(&self) -> anyhow::Result<Vec<MetadataEntry>> {
        Ok(objects())
    }
    fn list_delta(&self, _: Option<&str>) -> anyhow::Result<DeltaResult> {
        Ok(DeltaResult {
            entries: objects(),
            deleted_remote_ids: vec![],
            delta_link: None,
        })
    }
    fn get_metadata(&self, id: &str) -> anyhow::Result<Option<MetadataEntry>> {
        Ok(Some(object(id)?.1))
    }
    fn get_metadata_by_path(&self, path: &str) -> anyhow::Result<Option<MetadataEntry>> {
        Ok(objects().into_iter().find(|m| m.path == path))
    }
    fn download(&self, id: &str) -> anyhow::Result<Vec<u8>> {
        Ok(generated_bytes(object(id)?.0))
    }
    fn download_to(
        &self,
        id: &str,
        writer: &mut dyn Write,
        progress: &mut dyn FnMut(u64) -> anyhow::Result<()>,
    ) -> anyhow::Result<u64> {
        let bytes = self.download(id)?;
        for (index, chunk) in bytes.chunks(4096).enumerate() {
            progress((index * 4096) as u64)?;
            writer.write_all(chunk)?;
        }
        progress(SIZE as u64)?;
        Ok(SIZE as u64)
    }
    fn upload(&self, _: &str, _: Vec<u8>) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("generated provider is read-only")
    }
    fn upload_with_etag(
        &self,
        _: &str,
        _: Vec<u8>,
        _: Option<&str>,
    ) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("generated provider is read-only")
    }
    fn create_folder(&self, _: &str) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("mkdir unsupported")
    }
    fn rename(&self, _: &str, _: &str) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("rename unsupported")
    }
    fn delete(&self, _: &str) -> anyhow::Result<()> {
        anyhow::bail!("delete unsupported")
    }
}
impl Provider for GeneratedProvider {
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn list_directory(&self, _: &str) -> anyhow::Result<Vec<MetadataEntry>> {
        Ok(objects())
    }
    fn upload_controlled(
        &self,
        _: &str,
        _: &Path,
        _: Option<&str>,
        _: Option<&str>,
        _: Progress,
    ) -> anyhow::Result<MetadataEntry> {
        anyhow::bail!("generated provider is read-only")
    }
}

struct EngineSource {
    bridge: LocalFileBridge,
    gate: Mutex<()>,
    offline: AtomicBool,
    failed: Mutex<HashSet<String>>,
    reading: Mutex<HashSet<String>>,
}
impl HydrationSource for EngineSource {
    fn size(&self, identity: &[u8]) -> Result<usize, String> {
        object(std::str::from_utf8(identity).map_err(|_| "invalid identity")?)
            .map(|(_, m)| m.size as usize)
            .map_err(|e| e.to_string())
    }
    fn read_range(
        &self,
        identity: &[u8],
        offset: usize,
        length: usize,
        cancelled: &AtomicBool,
    ) -> Result<Vec<u8>, String> {
        let id = std::str::from_utf8(identity).map_err(|_| "invalid identity")?;
        object(id).map_err(|e| e.to_string())?;
        self.reading
            .lock()
            .map_err(|_| "read state poisoned")?
            .insert(id.into());
        let result = self
            .read(identity, offset, length, cancelled)
            .map_err(|e| e.to_string());
        self.reading
            .lock()
            .map_err(|_| "read state poisoned")?
            .remove(id);
        let mut failed = self.failed.lock().map_err(|_| "error state poisoned")?;
        if result.is_err() {
            failed.insert(id.into());
        } else {
            failed.remove(id);
        }
        result
    }
}
impl EngineSource {
    fn read(
        &self,
        identity: &[u8],
        offset: usize,
        length: usize,
        cancelled: &AtomicBool,
    ) -> anyhow::Result<Vec<u8>> {
        let _gate = self
            .gate
            .lock()
            .map_err(|_| anyhow::anyhow!("source gate poisoned"))?;
        let id = std::str::from_utf8(identity)?;
        object(id)?;
        ensure!(
            length <= 65536 && offset.checked_add(length).is_some_and(|end| end <= SIZE),
            "bounded range required"
        );
        ensure!(!cancelled.load(Ordering::SeqCst), "read cancelled");
        ensure!(
            !self.bridge.journal().native_dirty(id)?,
            "native edit requires recovery"
        );
        let record = self
            .bridge
            .journal()
            .database()
            .get_by_cloud_remote_id(id)?;
        let local_id = if let Some(record) = record.filter(|r| r.cache_path.is_some()) {
            record.metadata.remote_id
        } else {
            ensure!(
                !self.offline.load(Ordering::SeqCst),
                "generated provider offline"
            );
            self.bridge.hydrate_cancellable(id, cancelled)?
        };
        let state = self.bridge.status(&local_id)?;
        ensure!(
            state.state == FileState::Cached
                && !state.dirty_intent
                && !state.recovery_required
                && !state.uncertain_upload
                && !state.pending_operations,
            "cache is not authoritative"
        );
        let mut file = self.bridge.journal().open_read(&local_id)?;
        let expected = file.acknowledged_digest().to_owned();
        if crate::local_files::digest_reader(&mut file)? != expected {
            drop(file);
            self.bridge
                .journal()
                .record_external_edit(&local_id, state.generation)?;
            anyhow::bail!("private cache bytes changed; preserved for recovery");
        }
        file.seek(SeekFrom::Start(offset as u64))?;
        let mut bytes = vec![0; length];
        file.read_exact(&mut bytes)?;
        ensure!(!cancelled.load(Ordering::SeqCst), "read cancelled");
        Ok(bytes)
    }
}

/// This service owns Session on its creating thread; raw OS handles are never
/// unsafely marked Send. A small channel is the engine/desktop boundary.
pub struct FixtureService {
    stopped: AtomicBool,
    requests: mpsc::SyncSender<Command>,
    worker: Option<thread::JoinHandle<()>>,
}
struct Command {
    op: String,
    id: String,
    value: bool,
    reply: mpsc::SyncSender<anyhow::Result<Value>>,
}
impl FixtureService {
    /// Explicit user-authorized new root only. State must be a NEW sibling of
    /// that root, not the desktop profile/cache or another client's data.
    pub fn start(root: PathBuf) -> anyhow::Result<Self> {
        ensure!(
            root.is_absolute() && !root.exists(),
            "new absolute generated root required"
        );
        let (requests, incoming) = mpsc::sync_channel::<Command>(4);
        let (ready, started) = mpsc::sync_channel(1);
        let worker = thread::spawn(move || {
            let host = Host::start(&root);
            match host {
                Err(error) => {
                    let _ = ready.send(Err(error));
                }
                Ok(mut host) => {
                    if ready.send(Ok(())).is_err() {
                        return;
                    }
                    loop {
                        match incoming.recv_timeout(Duration::from_millis(500)) {
                            Ok(command) => {
                                let stop = command.op == "stop";
                                let result = host.command(&command.op, &command.id, command.value);
                                let _ = command.reply.send(result);
                                if stop {
                                    break;
                                }
                            }
                            Err(mpsc::RecvTimeoutError::Timeout) => host.tick(),
                            Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        }
                    }
                }
            }
        });
        match started.recv().context("fixture owner exited")? {
            Ok(()) => Ok(Self {
                stopped: AtomicBool::new(false),
                requests,
                worker: Some(worker),
            }),
            Err(error) => {
                let _ = worker.join();
                Err(error)
            }
        }
    }
    pub fn request(&self, op: &str, id: &str, value: bool) -> anyhow::Result<Value> {
        if self.stopped.load(Ordering::SeqCst) {
            ensure!(op == "stop", "fixture stopped");
            return Ok(json!({"unregistered":true,"files_retained":true}));
        }
        let (reply, result) = mpsc::sync_channel(1);
        self.requests
            .send(Command {
                op: op.into(),
                id: id.into(),
                value,
                reply,
            })
            .context("fixture owner disconnected")?;
        let result = result.recv().context("fixture owner exited")?;
        if op == "stop" && result.is_ok() {
            self.stopped.store(true, Ordering::SeqCst);
        }
        result
    }
    pub fn active(&self) -> bool {
        !self.stopped.load(Ordering::SeqCst)
    }
}
impl Drop for FixtureService {
    fn drop(&mut self) {
        // Finish returns explicit evidence. Drop also keeps callbacks alive until
        // disconnect; no private/generated bytes are deleted on shutdown.
        if self.worker.is_some() {
            if let Err(error) = self.request("stop", "", false) {
                eprintln!("native fixture cleanup unconfirmed: {error}");
            }
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
}
struct Host {
    session: Session,
    source: Arc<EngineSource>,
    paths: Vec<PathBuf>,
    stopped: bool,
    observed_pin: Vec<(bool, bool)>,
    shell_error: bool,
}
impl Host {
    fn start(root: &Path) -> anyhow::Result<Self> {
        // Create sibling state only beneath original locked ordinary ancestors.
        let state = root.with_extension("engine-state");
        let state_guard = native::GeneratedDirectory::create(&state).map_err(anyhow::Error::msg)?;
        let state = state_guard.path();
        let cache_guard =
            native::GeneratedDirectory::create(&state.join("cache")).map_err(anyhow::Error::msg)?;
        let cache = cache_guard.path();
        let bridge = LocalFileBridge::generated_fixture(
            Database::new(state.join("fixture.sqlite3")),
            cache,
        )?;
        let source = Arc::new(EngineSource {
            bridge,
            gate: Mutex::new(()),
            offline: AtomicBool::new(false),
            failed: Mutex::default(),
            reading: Mutex::default(),
        });
        let session =
            Session::create_with_source(root, source.clone()).map_err(anyhow::Error::msg)?;
        let mut host = Self {
            session,
            source,
            paths: vec![],
            stopped: false,
            observed_pin: vec![(false, false); objects().len()],
            shell_error: false,
        };
        host.session.attach_shell().map_err(anyhow::Error::msg)?;
        for entry in objects() {
            host.paths.push(
                host.session
                    .placeholder_for(Path::new(&entry.name), entry.remote_id.as_bytes(), SIZE)
                    .map_err(anyhow::Error::msg)?,
            );
        }
        host.items()?;
        Ok(host)
    }
    fn observe(&self, index: usize, entry: &MetadataEntry) -> anyhow::Result<NativeObservation> {
        match native::inspect_item(&self.paths[index], entry.remote_id.as_bytes(), SIZE) {
            Ok(os) => {
                if os.modified_bytes != 0 || !os.os_in_sync || os.logical_size != SIZE as u64 {
                    self.source
                        .bridge
                        .journal()
                        .record_native_edit(&entry.remote_id)?;
                }
                Ok(os)
            }
            Err(error) => {
                self.source
                    .bridge
                    .journal()
                    .record_native_edit(&entry.remote_id)?;
                Err(anyhow::Error::msg(error))
            }
        }
    }
    fn items(&mut self) -> anyhow::Result<Value> {
        let result = self.collect_items();
        if result.is_err() {
            self.shell_error = true;
            let root = self.session.root().to_owned();
            let _ = self.session.publish_shell(&root, shell_text("unknown"));
        }
        result
    }
    fn collect_items(&mut self) -> anyhow::Result<Value> {
        self.shell_error = false;
        let mut items = vec![];
        for (index, entry) in objects().into_iter().enumerate() {
            let os = self.observe(index, &entry)?;
            let journal = self.source.bridge.journal();
            let record = journal
                .database()
                .get_by_cloud_remote_id(&entry.remote_id)?;
            let state = record
                .as_ref()
                .filter(|r| r.cache_path.is_some())
                .map(|r| journal.snapshot(&r.metadata.remote_id))
                .transpose()?;
            let dirty = journal.native_dirty(&entry.remote_id)?;
            let failed = self
                .source
                .failed
                .lock()
                .map_err(|_| anyhow::anyhow!("error state poisoned"))?
                .contains(&entry.remote_id);
            let reading = self
                .source
                .reading
                .lock()
                .map_err(|_| anyhow::anyhow!("read state poisoned"))?
                .contains(&entry.remote_id);
            let sync = if dirty {
                "pending_sync"
            } else if failed {
                "error"
            } else if reading {
                "hydrating"
            } else if state.as_ref().is_some_and(|s| s.sync_confirmed()) {
                "in_sync"
            } else {
                "remote_version_known"
            };
            if self
                .session
                .publish_shell(&self.paths[index], shell_text(sync))
                .is_err()
            {
                self.shell_error = true;
            }
            items.push(json!({"id":entry.remote_id,"name":entry.name,"size":entry.size,
                "residency":if os.pinned && !os.resident {"pin_pending"} else if os.resident {"locally_available"} else if os.partial {"partially_available"} else {"online_only"},
                "pinned":os.pinned,"os_in_sync":os.os_in_sync,"modified_bytes":os.modified_bytes,
                "downloaded":os.resident,"cache_resident":state.as_ref().is_some_and(|s|s.resident),
                "sync":sync,
                "native_dirty":dirty,"generation":state.as_ref().map(|s|s.generation),"acknowledged_generation":state.as_ref().map(|s|s.acknowledged_generation),
                "pending_operations":state.as_ref().map(|s|s.pending_operations),"readers":state.as_ref().map(|s|s.readers),"writers":state.as_ref().map(|s|s.writers),
                "recovery_required":state.as_ref().map(|s|s.recovery_required)}));
        }
        let summary = aggregate(&items);
        let root = self.session.root().to_owned();
        if self
            .session
            .publish_shell(
                &root,
                shell_text(summary["sync"].as_str().unwrap_or("unknown")),
            )
            .is_err()
        {
            self.shell_error = true;
        }
        Ok(
            json!({"schema_version":1,"scope":"generated_fixture_only","items":items,"summary":summary,"root_path":self.session.root(),"shell_registered":true,"shell_state_error":self.shell_error,"fetch_callbacks":self.session.calls(),"fetch_failures":self.session.failures(),"offline":self.source.offline.load(Ordering::SeqCst)}),
        )
    }
    fn tick(&mut self) {
        // Bounded metadata-only observation of our generated leaves. React only
        // to a new Shell pin/free-space intent, never to arbitrary directory data.
        for (index, entry) in objects().into_iter().enumerate() {
            let Ok(os) = self.observe(index, &entry) else {
                continue;
            };
            let pin = (os.pinned, os.release_requested);
            let previous = std::mem::replace(&mut self.observed_pin[index], pin);
            if previous != pin {
                if os.pinned && !os.resident {
                    let _ = self.command("read", &entry.remote_id, false);
                } else if os.release_requested && os.resident {
                    let _ = self.command("dehydrate", &entry.remote_id, false);
                }
            }
        }
        if self.items().is_err() {
            self.shell_error = true;
        }
    }
    fn command(&mut self, op: &str, id: &str, value: bool) -> anyhow::Result<Value> {
        if op == "stop" {
            self.retain_generated_files()?;
            self.session.finish().map_err(anyhow::Error::msg)?;
            self.stopped = true;
            return Ok(json!({"unregistered":true,"files_retained":true}));
        }
        ensure!(!self.stopped, "fixture stopped");
        match op {
            "items" => self.items(),
            "offline" => {
                self.source.offline.store(value, Ordering::SeqCst);
                self.items()
            }
            "reconnect" => {
                self.session.disconnect().map_err(anyhow::Error::msg)?;
                self.session.verify_shell().map_err(anyhow::Error::msg)?;
                self.session.connect().map_err(anyhow::Error::msg)?;
                self.items()
            }
            "pin" | "unpin" => {
                let (index, entry) = object(id)?;
                self.observe(index, &entry)?;
                native::pin(&self.paths[index], op == "pin").map_err(anyhow::Error::msg)?;
                self.observed_pin[index] = (op == "pin", false);
                if op == "pin" {
                    return self.command("read", id, false);
                }
                self.items()
            }
            "read" => {
                let (index, entry) = object(id)?;
                self.observe(index, &entry)?;
                // Real normal OS read: only CldApi invokes EngineSource.
                let bytes = fs::read(&self.paths[index])?;
                ensure!(
                    bytes == generated_bytes(index),
                    "generated file content differs; preserved"
                );
                self.items()
            }
            "dehydrate" => {
                let (index, entry) = object(id)?;
                self.observe(index, &entry)?;
                let bridge = &self.source.bridge;
                ensure!(!bridge.journal().native_dirty(id)?, "native dirty veto");
                let record = bridge
                    .journal()
                    .database()
                    .get_by_cloud_remote_id(id)?
                    .context("item not downloaded")?;
                let state = bridge.status(&record.metadata.remote_id)?;
                native::with_exclusive_placeholder(
                    &self.paths[index],
                    id.as_bytes(),
                    SIZE,
                    |locked| {
                        let os = locked.inspect()?;
                        if !os.clean() || os.logical_size != SIZE as u64 {
                            return Err("native state veto".into());
                        }
                        let result = bridge
                            .journal()
                            .with_clean_cache(
                                &record.metadata.remote_id,
                                state.generation,
                                |file, expected| {
                                    ensure!(
                                        digest_file(file)? == expected,
                                        "private cache content changed"
                                    );
                                    locked.dehydrate().map_err(anyhow::Error::msg)
                                },
                            )
                            .map_err(|e| e.to_string())?;
                        result.ok_or_else(|| "journal veto".to_owned())
                    },
                )
                .map_err(anyhow::Error::msg)?;
                // Only the OS copy was released. Retained private cache is
                // separately exposed; full cache eviction stays unsupported.
                self.items()
            }
            _ => anyhow::bail!("unsupported native fixture operation"),
        }
    }
    fn retain_generated_files(&self) -> anyhow::Result<()> {
        // This Provider is internal deterministic data, never a real account.
        // Materialize only clean missing ranges before destructive unregister.
        self.source.offline.store(false, Ordering::SeqCst);
        for (index, entry) in objects().into_iter().enumerate().take(self.paths.len()) {
            let os = self.observe(index, &entry)?;
            if !os.resident {
                ensure!(
                    !self
                        .source
                        .bridge
                        .journal()
                        .native_dirty(&entry.remote_id)?,
                    "unretained dirty file requires recovery; root retained"
                );
                let bytes = fs::read(&self.paths[index])?;
                ensure!(
                    bytes == generated_bytes(index),
                    "generated retention content differs"
                );
            }
        }
        Ok(())
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        if !self.stopped {
            let result = self
                .retain_generated_files()
                .and_then(|()| self.session.finish().map_err(anyhow::Error::msg));
            if let Err(error) = result {
                eprintln!("generated root retained; cleanup unconfirmed: {error}");
            }
        }
    }
}

fn shell_text(sync: &str) -> &'static str {
    match sync {
        "in_sync" => "Acknowledged generation (generated provider)",
        "pending_sync" => "Pending recovery - local edits preserved",
        "hydrating" => "Reading generated provider",
        "error" => "Read failed - retry required",
        "conflict" => "Conflict - local bytes preserved",
        "remote_version_known" => "Remote version known - no local acknowledgement",
        _ => "Unknown - synchronization unverified",
    }
}
fn aggregate(items: &[Value]) -> Value {
    let sync = [
        "conflict",
        "error",
        "pending_sync",
        "hydrating",
        "remote_version_known",
    ]
    .into_iter()
    .find(|label| items.iter().any(|i| i["sync"] == *label))
    .unwrap_or(
        if !items.is_empty() && items.iter().all(|i| i["sync"] == "in_sync") {
            "in_sync"
        } else {
            "unknown"
        },
    );
    json!({"sync":sync,"files":items.len(),"native_resident":items.iter().filter(|i| i["downloaded"] == true).count(),"private_cache_resident":items.iter().filter(|i|i["cache_resident"] == true).count(),"pinned":items.iter().filter(|i| i["pinned"] == true).count()})
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires its own explicitly approved NEW generated Shell root"]
    fn untouched_placeholders_survive_normal_exit() {
        let root = PathBuf::from(
            std::env::var_os("TWODRIVE_APPROVED_RETENTION_FIXTURE_ROOT")
                .expect("explicit approved retention root required"),
        );
        let mut host = Host::start(&root).unwrap();
        assert_eq!(host.session.calls(), 0);
        assert!(
            host.session
                .finish()
                .unwrap_err()
                .contains("not fully retained")
        );
        host.session.verify_shell().unwrap();
        assert_eq!(host.session.calls(), 0);
        host.command("stop", "", false).unwrap();
        drop(host);
        native::verify_unregistered(&root).unwrap();
        for (index, entry) in objects().into_iter().enumerate() {
            assert_eq!(
                fs::read(root.join(entry.name)).unwrap(),
                generated_bytes(index)
            );
        }
        println!("generated untouched placeholders fully retained; Shell/CFAPI unregistered");
    }
    #[test]
    fn folder_summary_keeps_errors_pending_and_cache_separate() {
        let clean =
            json!({"sync":"in_sync","downloaded":false,"cache_resident":true,"pinned":false});
        let pending =
            json!({"sync":"pending_sync","downloaded":true,"cache_resident":true,"pinned":true});
        assert_eq!(aggregate(&[])["sync"], "unknown");
        assert_eq!(aggregate(std::slice::from_ref(&clean))["sync"], "in_sync");
        let summary = aggregate(&[clean.clone(), pending.clone()]);
        assert_eq!(summary["sync"], "pending_sync");
        assert_eq!(summary["native_resident"], 1);
        assert_eq!(summary["private_cache_resident"], 2);
        for (state, expected) in [
            ("error", "error"),
            ("conflict", "conflict"),
            ("remote_version_known", "remote_version_known"),
        ] {
            assert_eq!(
                aggregate(&[clean.clone(), json!({"sync":state})])["sync"],
                expected
            );
        }
        assert!(shell_text("unknown").contains("unverified"));
    }
    #[test]
    #[ignore = "requires an explicitly approved NEW CFAPI fixture root; never run automatically"]
    fn native_engine_end_to_end_requires_explicit_registration() {
        let root = PathBuf::from(
            std::env::var_os("TWODRIVE_APPROVED_NATIVE_FIXTURE_ROOT")
                .expect("explicit approved fixture root required"),
        );
        let service = FixtureService::start(root.clone()).unwrap();
        let result = (|| -> anyhow::Result<Value> {
            let initial = service.request("items", "", false)?;
            ensure!(
                initial["shell_registered"] == true && initial["shell_state_error"] == false,
                "Shell registration/property publication failed"
            );
            ensure!(
                initial["fetch_callbacks"] == 0,
                "metadata unexpectedly hydrated"
            );
            ensure!(
                initial["items"]
                    .as_array()
                    .context("items")?
                    .iter()
                    .all(|item| item["residency"] == "online_only"
                        && item["cache_resident"] == false),
                "initial state"
            );
            for id in ["generated-object-0", "generated-object-1"] {
                service.request("read", id, false)?;
            }
            let first = root.join(&objects()[0].name);
            let open = fs::File::open(&first)?;
            ensure!(
                service
                    .request("dehydrate", "generated-object-0", false)
                    .is_err(),
                "open file did not veto"
            );
            drop(open);
            let released = service.request("dehydrate", "generated-object-0", false)?;
            ensure!(
                released["items"][0]["residency"] == "online_only"
                    && released["items"][0]["cache_resident"] == true,
                "OS/cache separation"
            );
            service.request("offline", "", true)?;
            service.request("read", "generated-object-0", false)?;
            ensure!(
                service
                    .request("read", "generated-object-2", false)
                    .is_err(),
                "unavailable source succeeded"
            );
            service.request("offline", "", false)?;
            service.request("read", "generated-object-2", false)?;
            let reconnected = service.request("reconnect", "", false)?;
            ensure!(
                reconnected["items"][2]["downloaded"] == true,
                "reconnect changed residency"
            );
            let second = root.join(&objects()[1].name);
            let pinned = service.request("pin", "generated-object-1", false)?;
            ensure!(
                pinned["items"][1]["pinned"] == true && pinned["items"][1]["downloaded"] == true,
                "pin not backed by native residency"
            );
            ensure!(
                service
                    .request("dehydrate", "generated-object-1", false)
                    .is_err(),
                "pin did not veto"
            );
            service.request("unpin", "generated-object-1", false)?;
            ensure!(
                fs::read(second)? == generated_bytes(1),
                "unpin discarded bytes"
            );
            let mut dirty = generated_bytes(0);
            let prefix = b"generated unuploaded edit";
            dirty[..prefix.len()].copy_from_slice(prefix);
            let mut file = fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&first)?;
            file.write_all(&dirty)?;
            file.sync_all()?;
            drop(file);
            let mut final_status = service.request("items", "", false)?;
            final_status.as_object_mut().unwrap().remove("root_path");
            ensure!(
                final_status["items"][0]["native_dirty"] == true
                    && final_status["items"][0]["sync"] == "pending_sync",
                "dirty state not durable"
            );
            ensure!(
                service
                    .request("dehydrate", "generated-object-0", false)
                    .is_err(),
                "dirty release succeeded"
            );
            ensure!(fs::read(&first)? == dirty, "whole dirty file changed");
            Ok(
                json!({"metadata_no_hydration":true,"normal_os_reads_through_engine":true,"chinese_long_leaf":true,"open_handle_veto":true,"native_copy_dehydration":true,"private_cache_retained":true,"offline_cached_read":true,"offline_missing_cache_failure_and_retry":true,"pin_veto":true,"native_dirty_durable_veto":true,"whole_dirty_file_preserved":true,"final_status":final_status}),
            )
        })();
        let cleanup = service.request("stop", "", false);
        drop(service);
        native::verify_unregistered(&root).expect("new fixture root cleanup unconfirmed");
        assert_eq!(cleanup.unwrap()["unregistered"], true);
        let report = result.unwrap();
        println!(
            "{}",
            json!({"result":report,"unregistered":true,"files_retained":true,"explorer_icons":"unverified","real_cloud":"not_tested"})
        );
    }
    #[test]
    fn actual_engine_source_hydrates_opaque_object_and_reuses_private_cache_offline() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let cache = root.join("cache");
        fs::create_dir(&cache).unwrap();
        let source = EngineSource {
            bridge: LocalFileBridge::generated_fixture(
                Database::new(root.join("state.db")),
                &cache,
            )
            .unwrap(),
            gate: Mutex::new(()),
            offline: AtomicBool::new(false),
            failed: Mutex::default(),
            reading: Mutex::default(),
        };
        let cancel = AtomicBool::new(false);
        let id = b"generated-object-0";
        assert_eq!(
            source.read_range(id, 0, 4096, &cancel).unwrap(),
            generated_bytes(0)[..4096]
        );
        source.offline.store(true, Ordering::SeqCst);
        assert_eq!(
            source.read_range(id, 4096, 4096, &cancel).unwrap(),
            generated_bytes(0)[4096..8192]
        );
        assert!(
            source
                .read_range(b"generated-object-1", 0, 4096, &cancel)
                .is_err()
        );
        assert!(source.failed.lock().unwrap().contains("generated-object-1"));
        assert!(source.reading.lock().unwrap().is_empty());
        source.offline.store(false, Ordering::SeqCst);
        assert_eq!(
            source
                .read_range(b"generated-object-1", 0, 4096, &cancel)
                .unwrap(),
            generated_bytes(1)[..4096]
        );
        assert!(!source.failed.lock().unwrap().contains("generated-object-1"));
        cancel.store(true, Ordering::SeqCst);
        assert!(source.read_range(id, 0, 4096, &cancel).is_err());
        assert!(
            source
                .read_range(b"/generated-object-0", 0, 4096, &AtomicBool::new(false))
                .is_err()
        );
    }
}
