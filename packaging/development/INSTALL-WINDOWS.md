# TwoDrive Development 0.1.4 — Windows x64

核对 Release 的 SHA256SUMS，解压完整目录后运行 `twodrive_dav.exe`。不要只复制一个 EXE；引擎、DLL 和 `data` 都必须保留。可以使用同目录 `install-desktop-windows.ps1` 安装到当前用户；自启动默认关闭。Development 不采用稳定客户端的登录状态。

连接页支持 WebDAV/NAS、已有或新邀请的 PC/NAS 共享、Google Drive 与 OneDrive。Google 登录需要用户自己的桌面应用凭据 JSON；本包不含账号令牌、用户 client secret 或设备身份。保存后点击“登录并使用”打开系统浏览器授权；“切换使用”选择其他连接，“当前使用”标明活动连接。PC/NAS 共享需对方运行 TwoDrive 分享服务，并通过可信渠道传递短时单次邀请；发现设备不等于已授权。

本版 Windows 支持文件浏览、读写传输、目录备份与托盘；Linux FUSE 功能不等于 Windows 挂载驱动支持。当前验证说明见 Release/verification.json；云端读写、组织盘、桌面缩放/多屏和网络打洞的范围分别说明。未来网盘适配须遵循 `docs/provider-integration-spec.md`；源码链接可通过同版本 GitHub 源码归档阅读。

English: Verify SHA256SUMS, extract every file, and run `twodrive_dav.exe`. The included installer is optional and operates on the current user. Keep the engine, DLLs and data directory together. Credentials are user-supplied; native Windows mounting is not implemented. The `docs/` directory includes the future-provider/login/upload design specification and setup guides.
