//! The paired transport currently exposes a loopback DAV gateway. This detail
//! stays in the adapter; services use the same provider contract as any backend.
use super::{Args, Connection, Process};
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use twodrive_dev::{profile::Profile, provider::Backend, state::State, webdav::WebDavBackend};
pub fn descriptor() -> Value {
    json!({"id":"peer","label":"TwoDrive","account_label":"远程设备","icon":"devices",
    "identity_fields":["peer_state"],"protected_paths":["peer_state"],"capabilities":{"upload":true,"conditional_upload":true,"mkdir":true,"rename":true,"delete":true,"backup":true},"fields":[{"key":"peer_state","label":"本机配对状态目录 · 绝对路径","required":true}]})
}
pub fn validate(p: &Profile) -> anyhow::Result<()> {
    ensure!(
        Path::new(p.option("peer_state")).is_absolute(),
        "配对状态必须是绝对路径"
    );
    Ok(())
}
fn backend(p: &Profile, url: &str) -> anyhow::Result<Backend> {
    let credentials = State::new(p.option("peer_state").into())?.read("credentials.json")?;
    Ok(Backend::new(WebDavBackend::routed(
        url,
        Some(credentials),
        p.writable,
        "direct",
    )?))
}
pub fn restore(p: &Profile, recipe: &Value) -> anyhow::Result<Backend> {
    backend(
        p,
        recipe["gateway"]
            .as_str()
            .context("missing paired gateway")?,
    )
}
pub fn connect(
    p: &Profile,
    args: &Args,
    cancel: &AtomicBool,
    network: Arc<Mutex<Vec<String>>>,
) -> anyhow::Result<Connection> {
    let peer = State::new(p.option("peer_state").into())?;
    let remote: iroh::EndpointAddr = peer.read("remote.json")?;
    let mut cmd = Command::new(&args.peer_bin);
    cmd.arg("--state")
        .arg(&peer.dir)
        .args(["connect", "--listen", "127.0.0.1:0"])
        .args(["--network-route", &p.network_route]);
    let mut child: Process = super::super::spawn(cmd)?;
    super::super::drain_errors(&mut child, network);
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
    let backend = backend(p, &url)?;
    backend.list_directory("/")?;
    ensure!(!cancel.load(Ordering::Relaxed), "连接已取消");
    Ok(Connection {
        backend,
        binding: format!("peer:{}", remote.id),
        gateway: Some(child),
        worker: json!({"kind":p.kind,"revision":p.revision,"gateway":url}),
    })
}
