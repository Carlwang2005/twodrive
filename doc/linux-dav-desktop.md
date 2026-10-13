# TwoDrive Development desktop

The Flutter desktop client supports TwoDrive paired devices and HTTPS WebDAV
accounts. Its separate private state and installer do not manage the stable
OneDrive daemon. OneDrive and Google Drive are future integrations for this UI;
this delivery does not claim new OAuth integrations for those services.

## User interface

- Click the cloud tray icon to open a compact provider/account status panel.
  The gear menu offers settings, folder backups, pause/resume, sign-out and quit.
- The panel shows actual manual, FUSE and backup transfers. A green backup status
  requires a completed backup scan; a connection alone is not proof of backup.
- Use Connections to add a paired device or an HTTPS WebDAV account. Each account
  offers automatic physical egress with reported fallback, strict physical direct
  routing, or system routing/proxy. Disconnect before editing a connection.
- Use Backups to select local directories and remote destinations. Nothing is
  selected automatically. Backups upload additions and modifications; stopping a
  backup or deleting a local source does not delete the server copy.
- Pause persists across restarts. It stops new sync requests and backup streaming;
  an already executing filesystem request may finish. New local FUSE writes remain
  durable and resume through the existing recovery loop (up to its 60-second scan).
- Settings contains mount/unmount, clean-cache release and login autostart.
  **Autostart defaults off and installation never enables it.**

## Build and install

Requirements: Flutter 3.41.8, Rust 1.96, GTK 3 development files, clang, CMake,
Ninja, pkg-config, FUSE 3 and a session D-Bus StatusNotifierItem host. Current
native acceptance is Zorin/GNOME with its tray extension and GTK X11.
GNOME uses GTK XEmbed single-click events because this host reserves SNI Activate
for double-click. Other hosts use SNI. No shell extension is installed or modified.
The launcher selects X11 when DISPLAY is available, preserving an explicit GDK_BACKEND.
Other desktop hosts and Wayland compositor popup placement are not verified.

```bash
cargo build --locked --release --manifest-path experiments/twodrive-dev/Cargo.toml --features mount
cd apps/dav
flutter pub get --enforce-lockfile
flutter build linux --release
cd ../..
python3 scripts/install-dav-desktop.py \
  --bundle apps/dav/build/linux/x64/release/bundle \
  --engine-dir experiments/twodrive-dev/target/release
~/.local/bin/twodrive-dav
```

Public product name: **TwoDrive Development**. Internal `dav` paths and binary
names are retained for isolation and do not rename the product. Runtime lives in
`~/.local/share/twodrive-dav/runtime`; private profiles/cache/backup metadata live
under `~/.local/share/twodrive-dav/state`. The autostart entry, if explicitly enabled,
is `~/.config/autostart/twodrive-dav.desktop`.

## Safety and recovery

File actions bind both profile ID and revision before any remote request. Cache
binding includes the actual paired device identity or DAV origin/account. The
client refuses to adopt an already mounted directory, remove pending caches or
unmount with unfinished/conflicting writes. Downloads use a temporary sibling and
no-clobber rename. Credentials are private and omitted from profile listing IPC.

Folder backup configurations never delete source or remote files. Symlinks,
overlapping mappings, home/root, mount and private-state sources are rejected.
A private snapshot is streamed with bounded memory; durable upload intent is
written before PUT. A restarted worker reconciles an ambiguous committed PUT by
remote content hash before accepting its new ETag. Remote version conflicts stop
overwrites and remain visible. Source scans run every 10 seconds, skip unchanged
size/mtime fingerprints, and are bounded at 100000 entries per scan. Files changed
without any size/mtime change are outside this detection contract.

Safe reads have at most two retries for transient transport/502–504 failures,
before any response body is delivered. PUT, MOVE, DELETE and MKCOL are never
silently replayed. Physical HTTPS uses the existing bound DNS/CONNECT adapter;
automatic system/proxy fallback is selected by a safe connection probe before
mutations. Changing a route requires reconnecting. Numeric loopback gateways
always bypass external proxies; paired network traffic is governed by the
selected peer route policy.

## Verification (2026-10-13)

- Rust: 28 unit, 11 integration and 2 signed-control tests; strict Clippy and format.
- Flutter: 9 model/widget tests, analyzer and native release build.
- Local real FUSE: existing-mount ownership, file round trip, clean-cache release,
  pending writes retained while paused and uploaded after resume.
- Actual paired Windows: 2097356-byte SHA-256 round trip, upload/download, MOVE,
  FUSE read/write, backup create/update/pause/resume and remote conflict protection.
- Three route selections each connected to the actual Windows server. Diagnostics
  reported physical-only, system routing, and physical-first automatic respectively.
- Native D-Bus tray registration/Activate and repeated exit code 0 checks. The
  former GTK implicit-view destruction assertion is avoided by SystemNavigator.pop.
- A first Windows run encountered a transient 502. A synthetic regression fails
  before safe-read retry and passes afterward, while confirming PUT is not replayed.
  The deeper cause of that particular 502 is still unknown; later success is not
  proof that all network failures have disappeared.

Automated desktop checks use disposable local/remote UUID directories and never
print credentials or private listings. The recovery test reconstructs a durable
committed-upload intent in isolated state; it is not a whole-machine power-loss test.
See [incidents TD-20261013-01 through 14](incidents.md) for old/fixed evidence and
remaining boundaries. Yesterday's Windows native proxy fallback validation is
separately recorded in [its report](peer-windows-egress-2026-10-12.json).

The source includes Linux desktop CI and a portable bundle artifact. The installed
Development runtime connected to the real Windows gateway (PROPFIND 207); the
tray panel displayed TwoDrive / Windows and “文件已就绪”, with no synthetic backup
success. Autostart remained absent. Sanitized checks are stored in the
[validation report](linux-desktop-validation-2026-10-13.json). The installed and isolated Linux native fixtures now also compare the settled
window with the live pointer and workarea, accounting for GNOME icon-local event
coordinates and the mapped window rectangle. Installed opening took 28–54ms,
closing 4ms, with a stable 390×570 logical rectangle at 2× scaling. These are
synthetic host events, not hardware input or multi-monitor acceptance. [Final Linux CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38354827650)
passed the GTK rectangle fix; the Windows source is unchanged from the successful
[native Windows CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38353525532).
The earlier shared engine/provider workflows also passed. Local cleanup removed
12036603904 allocated bytes of owned build directories (11.2 GiB), 50 temporary
paths, the empty obsolete mount directory and the failed Windows diagnostic
artifact. Installed runtime, device pairing and stable daemon were retained. No stable
release is replaced by this Development build.

## Windows desktop

The same Flutter UI and IPC engine now have a native Windows runner, cloud tray,
per-user paths, owned process jobs and opt-in startup entry. Windows CI builds the
portable directory and exercises local WebDAV file operations and folder backups.
Windows native CI passed shared engine checks, nine Flutter tests, release build,
background tray toggles, outside-focus dismissal and Escape. The fixture uses
owned windows and notification messages; physical Windows GUI acceptance is not
claimed. The earlier first-activation access violation was reproduced before
window-manager taskbar initialization and absent afterward.

Extract the complete portable directory and run `twodrive_dav.exe`, or run its
`install-desktop-windows.ps1` to install in LocalAppData and create a Start Menu
shortcut. Installation does not enable startup. WebDAV browsing, file transfer
and upload-only folder backups are shared. FUSE mounting is a Linux capability;
the Windows UI does not present unsupported mount/cache actions. A Windows virtual
drive driver and macOS runner are outside this delivery.
