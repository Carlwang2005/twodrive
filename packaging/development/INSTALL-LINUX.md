# TwoDrive Development 0.1.4 — Linux x64

先核对 Release 的 SHA256SUMS，解压完整目录，在 `bundle` 内执行：

```bash
python3 install-dav-desktop.py --bundle . --engine-dir . --gnome-anchor ./gnome-tray-anchor
~/.local/bin/twodrive-dav
```

要求 GTK 3、会话 D-Bus 与托盘宿主；挂载需 FUSE 3。安装到当前用户，不替换稳定 OneDrive 服务，不自动启用自启动。运行中的更新可加 `--preserve-running-runtime` 保留旧进程路径；安装器不会重启客户端。

GNOME/Zorin 的 TwoDrive 专用定位组件支持 X11 和原生 Wayland。新组件首次安装可能需下一次登录桌面才加载；不要为了更新强制退出正在进行的工作。组件已加载时启动器在 Wayland 会话使用原生 Wayland，显式 GDK_BACKEND 优先。其他 Wayland compositor 尚未验收。

连接页添加 WebDAV/NAS、PC/NAS（需分享端运行 TwoDrive）、Google Drive 或 OneDrive。Google 需用户自备桌面应用凭据 JSON；本包不含任何账号/令牌/用户 client secret。首次 Microsoft/Google 连接打开系统浏览器授权。普通 NAS 可使用它的 WebDAV 服务。详细规范/使用说明在 `docs/`，其中源码链接与历史验证链接请通过 GitHub 同版本源码阅读。

English: Verify SHA256SUMS and keep the whole bundle. Run the command above from `bundle` to install for this user. GNOME's new positioning component may load only on the next desktop login. Explicit GDK_BACKEND is preserved; autostart remains unchanged. Credentials are supplied by the user, never bundled. Consult `docs/` and the matching source archive for the provider/auth/upload contract and validation boundaries.
