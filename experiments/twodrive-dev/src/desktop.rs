use anyhow::{Context, ensure};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use twodrive_core::{Database, FileState};
use twodrive_dev::{
    model::{CloudBackend, MetadataEntry},
    state::{Credentials, State},
    webdav::WebDavBackend,
};

#[path = "desktop_backup.rs"]
mod backup;
#[cfg(all(target_os = "linux", feature = "mount"))]
#[path = "mount.rs"]
mod mount;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    state: PathBuf,
    #[arg(long)]
    peer_bin: PathBuf,
    #[arg(long)]
    launcher: PathBuf,
    #[arg(long)]
    mount_worker: Option<String>,
    #[arg(long)]
    mount_url: Option<String>,
    #[arg(long)]
    backup_worker: Option<String>,
    #[arg(long, default_value = "direct")]
    http_route: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Profile {
    id: String,
    revision: String,
    name: String,
    kind: String,
    url: String,
    username: String,
    password: String,
    peer_state: PathBuf,
    writable: bool,
    mount_path: PathBuf,
    network_route: String,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            id: String::new(),
            revision: String::new(),
            name: String::new(),
            kind: "dav".into(),
            url: String::new(),
            username: String::new(),
            password: String::new(),
            peer_state: PathBuf::new(),
            writable: false,
            mount_path: PathBuf::new(),
            network_route: "auto".into(),
        }
    }
}
#[derive(Default, Serialize, Deserialize)]
struct Config {
    profiles: Vec<Profile>,
    selected: Option<String>,
}
#[derive(Deserialize)]
struct Request {
    id: u64,
    op: String,
    #[serde(default)]
    args: Value,
}

type Output = Arc<Mutex<std::io::Stdout>>;
type Cancellations = Arc<Mutex<HashMap<u64, Arc<AtomicBool>>>>;
fn emit(out: &Output, value: Value) {
    if let Ok(mut out) = out.lock() {
        let _ = writeln!(out, "{value}");
        let _ = out.flush();
    }
}
fn string<'a>(args: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .context(format!("missing {key}"))
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
}
fn is_mount(path: &Path) -> anyhow::Result<bool> {
    Ok(Command::new("mountpoint")
        .args(["-q", "--"])
        .arg(path)
        .status()?
        .success())
}
fn metadata(m: MetadataEntry) -> Value {
    json!({"path":m.path,"name":m.name,"directory":m.is_dir,"size":m.size,
        "modified":m.modified_unix,"etag":m.etag})
}

#[path = "desktop_process.rs"]
mod process_support;
struct Process(Child, #[cfg(windows)] process_support::WindowsJob);
impl Drop for Process {
    fn drop(&mut self) {
        #[cfg(windows)]
        let _ = &self.1;
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn spawn(mut command: Command) -> anyhow::Result<Process> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // No gateway or mount worker should outlive a crashed desktop engine.
    #[cfg(target_os = "linux")]
    unsafe {
        command.pre_exec(|| {
            let parent = libc::getppid();
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) != 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::getppid() != parent {
                return Err(std::io::Error::other("parent exited"));
            }
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let mut child = command.spawn()?;
    #[cfg(windows)]
    let job = process_support::WindowsJob::attach(&mut child)?;
    // The mutable binding is needed by the Windows job attachment.
    let _ = &mut child;
    Ok(Process(
        child,
        #[cfg(windows)]
        job,
    ))
}
fn drain_errors(process: &mut Process, network: Arc<Mutex<Vec<String>>>) {
    if let Some(stderr) = process.0.stderr.take() {
        thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                // Only the network layer's credential-free diagnostics enter the UI.
                if line.starts_with("network:") {
                    let mut lines = network.lock().unwrap();
                    if lines.len() >= 24 {
                        lines.remove(0);
                    }
                    lines.push(line.chars().take(512).collect());
                }
            }
        });
    }
}

struct App {
    args: Args,
    state: State,
    config: Config,
    connected: Option<Profile>,
    backend: Option<WebDavBackend>,
    gateway: Option<Process>,
    mounted: Option<Process>,
    backup: Option<Process>,
    network: Arc<Mutex<Vec<String>>>,
}
impl App {
    fn profile(&self, id: &str) -> anyhow::Result<Profile> {
        self.config
            .profiles
            .iter()
            .find(|p| p.id == id)
            .cloned()
            .context("profile not found")
    }
    fn mount_state(&self, p: &Profile) -> PathBuf {
        self.state.dir.join(format!("mount-{}", p.id))
    }
    fn credentials(p: &Profile) -> anyhow::Result<Option<Credentials>> {
        if p.kind == "peer" {
            return Ok(Some(
                State::new(p.peer_state.clone())?.read("credentials.json")?,
            ));
        }
        Ok((!p.username.is_empty()).then(|| Credentials {
            username: p.username.clone(),
            password: p.password.clone(),
        }))
    }
    fn cache(&self, p: &Profile, clear: bool) -> anyhow::Result<Value> {
        let path = self.mount_state(p).join("mount.sqlite3");
        if !path.exists() {
            return Ok(json!({"bytes":0,"pending":0,"released":0}));
        }
        let db = Database::new(path);
        let released = if clear { db.prune_cache(0)? } else { 0 };
        let records = db.all_records()?;
        let pending = records
            .iter()
            .filter(|r| {
                matches!(
                    r.state,
                    FileState::Writing
                        | FileState::Dirty
                        | FileState::Uploading
                        | FileState::Conflict
                        | FileState::Error
                )
            })
            .count()
            + db.pending_deletes()?.len()
            + db.pending_metadata_operations()?.len();
        let bytes: u64 = records
            .iter()
            .filter_map(|r| r.cache_path.as_ref())
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum();
        let waiting: Vec<_> = records.iter().filter(|r| matches!(r.state, FileState::Writing | FileState::Dirty | FileState::Uploading | FileState::Conflict | FileState::Error)).take(30).map(|r| json!({"name":r.metadata.name,"path":r.metadata.path,"kind":"upload","state":if matches!(r.state, FileState::Conflict | FileState::Error) {"需要处理"} else {"等待同步"},"bytes_done":0,"bytes_total":r.metadata.size})).collect();
        Ok(json!({"bytes":bytes,"pending":pending,"released":released,"waiting":waiting}))
    }
    fn unmount(&mut self, check_pending: bool) -> anyhow::Result<()> {
        if self.mounted.is_none() {
            return Ok(());
        }
        let p = self.connected.as_ref().context("missing mount profile")?;
        if !is_mount(&p.mount_path)? {
            self.mounted.take();
            return Ok(());
        }
        if check_pending {
            ensure!(
                self.cache(p, false)?["pending"] == 0,
                "挂载仍有未上传或冲突的数据，请先完成同步；缓存已保留"
            );
        }
        let result = Command::new("fusermount3")
            .arg("-u")
            .arg(&p.mount_path)
            .output()?;
        ensure!(
            result.status.success(),
            "挂载仍被占用，请关闭使用该目录的程序后重试"
        );
        self.mounted.take();
        Ok(())
    }
    fn disconnect(&mut self) -> anyhow::Result<()> {
        self.unmount(true)?;
        self.backup.take();
        if let Some(p) = &self.connected {
            backup::cleanup_spool(&self.state, p)?;
        }
        self.backend = None;
        self.gateway.take();
        self.connected = None;
        self.network.lock().unwrap().clear();
        Ok(())
    }
    fn connect(&mut self, id: &str, cancel: &AtomicBool) -> anyhow::Result<Value> {
        self.disconnect()?;
        let p = self.profile(id)?;
        self.network.lock().unwrap().clear();
        let mut gateway = None;
        let url = if p.kind == "peer" {
            let mut cmd = Command::new(&self.args.peer_bin);
            cmd.arg("--state")
                .arg(&p.peer_state)
                .args(["connect", "--listen", "127.0.0.1:0"])
                .args(["--network-route", &p.network_route]);
            let mut child = spawn(cmd)?;
            drain_errors(&mut child, self.network.clone());
            let stdout = child.0.stdout.take().context("gateway stdout")?;
            let (send, receive) = mpsc::channel();
            thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Some(url) = line.strip_prefix("WebDAV: ") {
                        let _ = send.send(url.to_owned());
                    }
                }
            });
            let start = Instant::now();
            let url = loop {
                ensure!(!cancel.load(Ordering::Relaxed), "连接已取消");
                ensure!(
                    start.elapsed() < Duration::from_secs(60),
                    "连接超时，请确认服务器和网络可用"
                );
                ensure!(
                    child.0.try_wait()?.is_none(),
                    "配对网关已退出，请检查配对状态、网络和占用情况"
                );
                if let Ok(url) = receive.recv_timeout(Duration::from_millis(100)) {
                    break url;
                }
            };
            gateway = Some(child);
            url
        } else {
            p.url.clone()
        };
        let initial = if p.kind == "peer" || p.network_route != "system" {
            "direct"
        } else {
            "system"
        };
        let attempt = || -> anyhow::Result<_> {
            let b = WebDavBackend::routed(&url, Self::credentials(&p)?, p.writable, initial)?;
            let entries = b.list_directory("/")?;
            Ok((b, entries))
        };
        let (backend, entries) = match attempt() {
            Ok(value) => value,
            Err(error)
                if p.kind == "dav"
                    && p.network_route == "auto"
                    && error
                        .downcast_ref::<twodrive_dev::webdav::DavError>()
                        .is_none() =>
            {
                self.network.lock().unwrap().push(
                    "network: route=system/proxy fallback; physical WebDAV probe failed".into(),
                );
                let b = WebDavBackend::routed(&url, Self::credentials(&p)?, p.writable, "system")?;
                let entries = b.list_directory("/")?;
                (b, entries)
            }
            Err(error) => return Err(error),
        };
        if p.kind == "dav" {
            self.network
                .lock()
                .unwrap()
                .push(format!("network: WebDAV route={}", backend.route()));
        }
        let revision = p.revision.clone();
        self.backend = Some(backend);
        self.gateway = gateway;
        self.connected = Some(p);
        self.config.selected = Some(id.to_owned());
        self.state.save("profiles.json", &self.config)?;
        self.start_backup()?;
        Ok(
            json!({"entries":entries.into_iter().map(metadata).collect::<Vec<_>>(),"path":"/","revision":revision}),
        )
    }
    fn start_backup(&mut self) -> anyhow::Result<()> {
        self.backup.take();
        let p = self.connected.as_ref().context("请先连接")?;
        backup::cleanup_spool(&self.state, p)?;
        if !p.writable {
            return Ok(());
        }
        backup::reset_report(&self.state, p)?;
        let backend = self.backend.as_ref().context("请先连接")?;
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg("--state")
            .arg(&self.state.dir)
            .arg("--peer-bin")
            .arg(&self.args.peer_bin)
            .arg("--launcher")
            .arg(&self.args.launcher)
            .arg("--backup-worker")
            .arg(&p.id)
            .arg("--mount-url")
            .arg(backend.root_url())
            .arg("--http-route")
            .arg(backend.route());
        let mut child = spawn(cmd)?;
        drain_errors(&mut child, self.network.clone());
        if let Some(mut stdout) = child.0.stdout.take() {
            thread::spawn(move || {
                let _ = std::io::copy(&mut stdout, &mut std::io::sink());
            });
        }
        self.backup = Some(child);
        Ok(())
    }
    fn autostart_path(&self) -> anyhow::Result<PathBuf> {
        #[cfg(windows)]
        return Ok(
            PathBuf::from(std::env::var_os("APPDATA").context("APPDATA missing")?)
                .join("Microsoft/Windows/Start Menu/Programs/Startup/TwoDrive Development.vbs"),
        );
        #[cfg(not(windows))]
        {
            let config = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or(
                    PathBuf::from(std::env::var_os("HOME").context("HOME is unset")?)
                        .join(".config"),
                );
            Ok(config.join("autostart/twodrive-dav.desktop"))
        }
    }
    fn status(&mut self) -> anyhow::Result<Value> {
        let gateway_dead = self
            .gateway
            .as_mut()
            .is_some_and(|p| p.0.try_wait().ok().flatten().is_some());
        let mount_dead = self
            .mounted
            .as_mut()
            .is_some_and(|p| p.0.try_wait().ok().flatten().is_some());
        if gateway_dead {
            self.backend = None;
        }
        let cache = self
            .connected
            .as_ref()
            .map(|p| self.cache(p, false))
            .transpose()?;
        let backup = self
            .connected
            .as_ref()
            .map(|p| backup::report(&self.state, p))
            .transpose()?;
        let mount_activity: Value = self
            .connected
            .as_ref()
            .and_then(|p| fs::read(self.mount_state(p).join("activity.json")).ok())
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or(json!({"active":[]}));
        let backup_failed = self
            .backup
            .as_mut()
            .is_some_and(|p| p.0.try_wait().ok().flatten().is_some());
        Ok(
            json!({"capabilities":{"mount":cfg!(all(target_os="linux",feature="mount"))},"paused":self.state.dir.join("paused").exists(),"backup":backup,"backup_failed":backup_failed,"mount_activity":mount_activity,
            "connected":self.backend.is_some(),"profile":self.connected.as_ref().map(|p|&p.id),
            "mounted":self.mounted.is_some(),"mount_failed":mount_dead,"gateway_failed":gateway_dead,
            "cache":cache,"network":self.network.lock().unwrap().clone(),
            "autostart":self.autostart_path()?.exists()}),
        )
    }
    fn handle(
        &mut self,
        req: &Request,
        cancel: Arc<AtomicBool>,
        out: &Output,
    ) -> anyhow::Result<Value> {
        let a = &req.args;
        match req.op.as_str() {
            "profiles" => {
                let profiles: Vec<_> = self
                    .config
                    .profiles
                    .iter()
                    .map(|p| {
                        let mut value = serde_json::to_value(p).unwrap();
                        value["password_saved"] = json!(!p.password.is_empty());
                        value.as_object_mut().unwrap().remove("password");
                        value
                    })
                    .collect();
                Ok(json!({"profiles":profiles,"selected":self.config.selected}))
            }
            "save_profile" => {
                ensure!(self.connected.is_none(), "请先断开连接再修改配置");
                let mut p: Profile = serde_json::from_value(a.clone())?;
                p.revision = twodrive_dev::state::random_secret();
                if p.id.is_empty() {
                    p.id = format!(
                        "p-{}",
                        twodrive_dev::state::random_secret()
                            .replace('_', "-")
                            .replace('-', "a")
                    );
                }
                ensure!(valid_id(&p.id), "invalid profile id");
                ensure!(
                    matches!(p.network_route.as_str(), "auto" | "direct" | "system"),
                    "invalid network route"
                );
                ensure!(
                    self.connected.as_ref().is_none_or(|c| c.id != p.id),
                    "请先断开连接再编辑配置"
                );
                ensure!(
                    !p.name.trim().is_empty() && p.name.len() <= 120,
                    "连接名称不能为空"
                );
                ensure!(
                    p.kind == "dav" || p.kind == "peer",
                    "unsupported profile kind"
                );
                ensure!(p.mount_path.is_absolute(), "挂载目录必须是绝对路径");
                if let Some(previous) = self.config.profiles.iter().find(|c| c.id == p.id)
                    && p.password.is_empty()
                    && a["clear_password"] != true
                {
                    p.password = previous.password.clone();
                }
                if p.kind == "dav" {
                    WebDavBackend::new(&p.url, Self::credentials(&p)?, p.writable)?;
                } else {
                    ensure!(p.peer_state.is_absolute(), "配对状态必须是绝对路径");
                }
                self.config.profiles.retain(|c| c.id != p.id);
                self.config.profiles.push(p.clone());
                self.config.selected = Some(p.id.clone());
                self.state.save("profiles.json", &self.config)?;
                Ok(json!({"id":p.id}))
            }
            "remove_profile" => {
                let id = string(a, "id")?;
                ensure!(
                    self.connected.as_ref().is_none_or(|p| p.id != id),
                    "请先断开连接"
                );
                let p = self.profile(id)?;
                ensure!(
                    self.cache(&p, false)?["pending"] == 0,
                    "此连接有待恢复的缓存，不能删除配置"
                );
                self.config.profiles.retain(|p| p.id != id);
                if self.config.selected.as_deref() == Some(id) {
                    self.config.selected = None;
                }
                self.state.save("profiles.json", &self.config)?;
                Ok(json!({}))
            }
            "connect" => self.connect(string(a, "id")?, &cancel),
            "disconnect" | "quit" => {
                self.disconnect()?;
                Ok(json!({}))
            }
            "status" => self.status(),
            "pause" => {
                let file = self.state.dir.join("paused");
                if a["paused"] == true {
                    self.state.save("paused", &true)?;
                } else if file.exists() {
                    fs::remove_file(file)?;
                }
                self.status()
            }
            "backups" | "save_backups" => {
                let p = self.profile(string(a, "id")?)?;
                if req.op == "save_backups" {
                    ensure!(p.writable, "只读连接不能备份");
                    let folders: Vec<backup::Folder> =
                        serde_json::from_value(a["folders"].clone())?;
                    backup::validate(&folders, &p, &self.state)?;
                    if self.connected.as_ref().is_some_and(|v| v.id == p.id) {
                        self.backup.take();
                        backup::cleanup_spool(&self.state, &p)?;
                    }
                    backup::save_folders(&self.state, &p, &folders)?;
                    if self.connected.as_ref().is_some_and(|v| v.id == p.id) {
                        self.start_backup()?;
                    }
                }
                Ok(json!({"folders":backup::folders(&self.state, &p)?}))
            }
            "sign_out" => {
                let id = string(a, "id")?.to_owned();
                self.disconnect()?;
                let p = self.profile(&id)?;
                ensure!(self.cache(&p, false)?["pending"] == 0, "请先处理待上传缓存");
                self.config.profiles.retain(|p| p.id != id);
                self.config.selected = None;
                self.state.save("profiles.json", &self.config)?;
                Ok(json!({}))
            }
            "autostart" => {
                let path = self.autostart_path()?;
                if a["enabled"] == true {
                    ensure!(self.args.launcher.is_file(), "installed launcher not found");
                    fs::create_dir_all(path.parent().unwrap())?;
                    let launcher = self.args.launcher.to_string_lossy();
                    ensure!(
                        !launcher.contains(['\n', '\r', '%']),
                        "unsupported launcher path"
                    );
                    #[cfg(not(windows))]
                    let escaped = launcher
                        .replace('\\', "\\\\")
                        .replace('"', "\\\"")
                        .replace('`', "\\`")
                        .replace('$', "\\$");
                    #[cfg(not(windows))]
                    fs::write(
                        &path,
                        format!(
                            "[Desktop Entry]\nType=Application\nName=TwoDrive Development\nExec=\"{escaped}\" --background\nIcon=twodrive-dav\nTerminal=false\nX-GNOME-Autostart-enabled=true\n"
                        ),
                    )?;
                    #[cfg(windows)]
                    {
                        let escaped = launcher.replace('"', "\"\"");
                        let script = format!(
                            "CreateObject(\"WScript.Shell\").Run \"\"\"{escaped}\"\" --background\", 0, False\r\n"
                        );
                        let bytes: Vec<u8> = std::iter::once(0xfeffu16)
                            .chain(script.encode_utf16())
                            .flat_map(u16::to_le_bytes)
                            .collect();
                        fs::write(&path, bytes)?;
                    }
                } else if path.exists() {
                    fs::remove_file(path)?;
                }
                self.status()
            }
            "cache_info" | "clear_cache" => {
                let p = self.profile(string(a, "id")?)?;
                self.cache(&p, req.op == "clear_cache")
            }
            "mount" => {
                ensure!(
                    cfg!(all(target_os = "linux", feature = "mount")),
                    "此平台暂不支持文件管理器挂载，请使用文件浏览与文件夹备份"
                );
                ensure!(self.mounted.is_none(), "已经挂载");
                let p = self.connected.as_ref().context("请先连接")?;
                ensure!(
                    !is_mount(&p.mount_path)?,
                    "目录已被其他进程挂载，请使用另一个空目录"
                );
                let url = self.backend.as_ref().context("请先连接")?.root_url();
                let mut cmd = Command::new(std::env::current_exe()?);
                cmd.arg("--state")
                    .arg(&self.state.dir)
                    .arg("--peer-bin")
                    .arg(&self.args.peer_bin)
                    .arg("--launcher")
                    .arg(&self.args.launcher)
                    .arg("--mount-worker")
                    .arg(&p.id)
                    .arg("--mount-url")
                    .arg(url)
                    .arg("--http-route")
                    .arg(self.backend.as_ref().unwrap().route());
                let mut child = spawn(cmd)?;
                drain_errors(&mut child, self.network.clone());
                if let Some(mut stdout) = child.0.stdout.take() {
                    thread::spawn(move || {
                        let _ = std::io::copy(&mut stdout, &mut std::io::sink());
                    });
                }
                let start = Instant::now();
                let ready = (|| -> anyhow::Result<()> {
                    loop {
                        ensure!(!cancel.load(Ordering::Relaxed), "挂载已取消");
                        ensure!(
                            child.0.try_wait()?.is_none(),
                            "挂载失败，请检查目录为空、FUSE 可用、缓存绑定匹配"
                        );
                        if is_mount(&p.mount_path)? {
                            break Ok(());
                        }
                        ensure!(start.elapsed() < Duration::from_secs(60), "挂载超时");
                        thread::sleep(Duration::from_millis(100));
                    }
                })();
                self.mounted = Some(child);
                if let Err(error) = ready {
                    self.unmount(false)
                        .context("挂载启动未完成且目录被占用，请先关闭占用程序再卸载")?;
                    return Err(error);
                }
                self.status()
            }
            "unmount" => {
                self.unmount(true)?;
                self.status()
            }
            op => {
                ensure!(
                    self.connected
                        .as_ref()
                        .is_some_and(|p| a["profile"].as_str() == Some(p.id.as_str())),
                    "文件操作所属连接已改变，请刷新后重试"
                );
                ensure!(
                    self.connected
                        .as_ref()
                        .is_some_and(|p| a["revision"].as_str() == Some(p.revision.as_str())),
                    "连接配置已更新，请刷新后重新选择文件"
                );
                let backend = self.backend.as_ref().context("请先连接服务器")?;
                ensure!(!cancel.load(Ordering::Relaxed), "操作已取消");
                if req.op != "list" {
                    ensure!(
                        !self.state.dir.join("paused").exists(),
                        "同步已暂停，请先恢复"
                    );
                }
                let path = string(a, "path")?;
                match op {
                    "list" => Ok(
                        json!({"path":path,"entries":backend.list_directory(path)?.into_iter().map(metadata).collect::<Vec<_>>()}),
                    ),
                    "mkdir" => Ok(metadata(backend.create_folder(path)?)),
                    "rename" => Ok(metadata(backend.rename(path, string(a, "to")?)?)),
                    "delete" => {
                        ensure!(path != "/", "不能删除共享根目录");
                        backend.delete(path)?;
                        Ok(json!({}))
                    }
                    "upload" => {
                        let out = out.clone();
                        let id = req.id;
                        let last = Arc::new(Mutex::new(Instant::now() - Duration::from_secs(1)));
                        let progress = Arc::new(move |done, total| {
                            ensure!(
                                !cancel.load(Ordering::Relaxed),
                                "上传已取消；若服务器已接收，请刷新确认结果"
                            );
                            let mut last = last.lock().unwrap();
                            if done == total || last.elapsed() >= Duration::from_millis(150) {
                                emit(
                                    &out,
                                    json!({"id":id,"event":"progress","done":done,"total":total}),
                                );
                                *last = Instant::now();
                            }
                            Ok(())
                        });
                        Ok(metadata(backend.upload_controlled(
                            path,
                            Path::new(string(a, "source")?),
                            a["etag"].as_str(),
                            progress,
                        )?))
                    }
                    "download" => {
                        let output = PathBuf::from(string(a, "output")?);
                        ensure!(output.is_absolute(), "下载目标必须是绝对路径");
                        ensure!(!output.exists(), "目标文件已存在，请选择另一个名称");
                        let parent = output.parent().context("invalid output")?;
                        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
                        let mut last = Instant::now() - Duration::from_secs(1);
                        let total = a["size"].as_u64().unwrap_or(0);
                        let done = backend.download_to(path, &mut temp, &mut |done| {
                            ensure!(!cancel.load(Ordering::Relaxed), "下载已取消");
                            if last.elapsed() >= Duration::from_millis(150) {
                                emit(out,json!({"id":req.id,"event":"progress","done":done,"total":total})); last = Instant::now();
                            } Ok(())
                        })?;
                        ensure!(!cancel.load(Ordering::Relaxed), "下载已取消");
                        temp.flush()?;
                        temp.as_file().sync_all()?;
                        temp.persist_noclobber(output)?;
                        Ok(json!({"bytes":done}))
                    }
                    _ => anyhow::bail!("unknown operation"),
                }
            }
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        let _ = self.unmount(false);
    }
}

pub fn run() -> anyhow::Result<()> {
    let args = Args::parse();
    let state = State::new(args.state.clone())?;
    let config = if state.dir.join("profiles.json").exists() {
        state.read("profiles.json")?
    } else {
        Config::default()
    };
    let mut app = App {
        args,
        state,
        config,
        connected: None,
        backend: None,
        gateway: None,
        mounted: None,
        backup: None,
        network: Arc::new(Mutex::new(Vec::new())),
    };
    if let Some(id) = &app.args.backup_worker {
        let p = app.profile(id)?;
        let url = app
            .args
            .mount_url
            .as_deref()
            .context("backup URL missing")?;
        let backend =
            WebDavBackend::routed(url, App::credentials(&p)?, p.writable, &app.args.http_route)?
                .with_pause_file(app.state.dir.join("paused"));
        return backup::run(&app.state, &p, backend);
    }
    #[cfg(all(target_os = "linux", feature = "mount"))]
    if let Some(id) = &app.args.mount_worker {
        let p = app.profile(id)?;
        ensure!(!is_mount(&p.mount_path)?, "mount target is already mounted");
        let url = app.args.mount_url.as_deref().context("mount URL missing")?;
        let backend =
            WebDavBackend::routed(url, App::credentials(&p)?, p.writable, &app.args.http_route)?
                .with_pause_file(app.state.dir.join("paused"));
        let binding = if p.kind == "peer" {
            let remote: iroh::EndpointAddr =
                State::new(p.peer_state.clone())?.read("remote.json")?;
            format!("peer:{}", remote.id)
        } else {
            format!("dav:{}:{}", p.url, p.username)
        };
        return mount::mount(
            State::new(app.mount_state(&p))?,
            p.mount_path,
            backend,
            binding,
            p.writable,
        );
    }
    let _lock = app.state.run_lock()?;
    let out = Arc::new(Mutex::new(std::io::stdout()));
    let cancellations: Cancellations = Arc::new(Mutex::new(HashMap::new()));
    let (send, receive) = mpsc::sync_channel::<Request>(32);
    let worker_out = out.clone();
    let worker_cancel = cancellations.clone();
    let worker = thread::spawn(move || {
        for req in receive {
            let cancel = worker_cancel
                .lock()
                .unwrap()
                .get(&req.id)
                .cloned()
                .unwrap_or_default();
            let result = app.handle(&req, cancel, &worker_out);
            let quit = req.op == "quit" && result.is_ok();
            let value = match result {
                Ok(value) => json!({"id":req.id,"ok":true,"result":value}),
                Err(error) => json!({"id":req.id,"ok":false,"error":error.to_string()}),
            };
            emit(&worker_out, value);
            worker_cancel.lock().unwrap().remove(&req.id);
            if quit {
                break;
            }
        }
    });
    let mut input = std::io::stdin().lock();
    loop {
        // UI requests never need large payloads: files stream from disk.
        let mut line = String::new();
        use std::io::Read;
        if input.by_ref().take(1024 * 1024 + 1).read_line(&mut line)? == 0 {
            break;
        }
        ensure!(line.len() <= 1024 * 1024, "IPC line exceeds limit");
        let req: Request = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if req.op == "cancel" {
            if let Some(id) = req.args["id"].as_u64()
                && let Some(cancel) = cancellations.lock().unwrap().get(&id)
            {
                cancel.store(true, Ordering::Relaxed);
            }
            emit(&out, json!({"id":req.id,"ok":true,"result":{}}));
        } else {
            let id = req.id;
            let mut flags = cancellations.lock().unwrap();
            if flags.contains_key(&id) {
                emit(
                    &out,
                    json!({"id":id,"ok":false,"error":"duplicate request id"}),
                );
                continue;
            }
            flags.insert(id, Arc::new(AtomicBool::new(false)));
            drop(flags);
            // Never block the reader behind a full work queue: cancellation must
            // still get through while a network request is waiting.
            match send.try_send(req) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(_)) => {
                    cancellations.lock().unwrap().remove(&id);
                    emit(
                        &out,
                        json!({"id":id,"ok":false,"error":"request queue is full"}),
                    );
                }
                Err(mpsc::TrySendError::Disconnected(_)) => break,
            }
        }
    }
    for cancel in cancellations.lock().unwrap().values() {
        cancel.store(true, Ordering::Relaxed);
    }
    drop(send);
    let _ = worker.join();
    Ok(())
}
