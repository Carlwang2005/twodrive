# Development 存储连接 / Storage connections

2026-10-14：`dev` 的 Development 桌面“连接 → 添加连接”可在已有连接在线时使用。可新增、修改、删除其他连接；正在使用的连接需先断开才能编辑或删除。点击另一行切换连接，沿用原有待上传缓存保护。当前为单个活动连接，配置列表可保存多个账号。

## WebDAV 网盘与 NAS

选择 **WebDAV / NAS**，填写服务端提供的根 URL、用户名、密码，按需启用“允许修改远端文件”，保存后点击该连接。默认使用 HTTPS 并验证证书。局域网 NAS 仅提供 HTTP 时，在高级设置中显式开启“允许局域网 HTTP”，地址须使用数字私有 IP，例如 `http://192.168.1.10:5005/`；该段连接不加密。公共地址或域名 HTTP 仍拒绝，URL 不得内嵌凭据。局域网 HTTP 使用操作系统到该 IP 的路由；不会据此宣称已绕过 VPN/透明网关。

根 URL 应指向启用了 WebDAV 的共享目录。读取、上传、条件覆盖、新建目录、移动、删除和文件夹备份依赖服务端权限和协议实现。S3、SMB、FTP 等需要各自适配器，不能只填进 WebDAV 地址。

## OneDrive 与 Google Drive

选择 **OneDrive**，填写连接名称并保存，点击该行时打开系统浏览器完成 Microsoft 登录。登录和缓存使用 Development 独立目录，不修改稳定版服务及其配置；OneDrive 当前使用系统路由/代理。普通文件支持读写、修改、移动、删除，Linux 可挂载，文件夹备份使用既有流程。新增对象使用冲突失败策略；覆盖必须具有现存对象的明确版本。可保存多个账号配置，浏览器若自动选中账号，可在 Microsoft 页面选择另一个账号。

Google 登录见 [Google Drive 指南](google-drive-dev.md)。私有登录目录和 Linux 挂载路径可留空，由引擎按连接 ID 自动选择独立目录；高级设置允许指定路径。不要把不同账号的私有登录目录指向同一个位置。

## PC / NAS 的 TwoDrive 设备共享

对方运行 TwoDrive `twodrive-dev serve`，显式选择共享目录。普通 NAS 无法运行此程序时，优先使用它的 WebDAV 服务。

Linux 分享端示例（状态目录须独立；`--write` 显式允许修改）：

```bash
twodrive-dev --state "$HOME/.local/share/twodrive-share" serve \
  --root /absolute/shared-directory --write \
  --discoverable --device-name "My PC" --invite-file /absolute/private/invite.json
```

Windows 分享端示例：

```powershell
.\twodrive-dev.exe --state "$env:LOCALAPPDATA\twodrive-share" serve `
  --root D:\Share --write --discoverable --device-name "My PC" `
  --invite-file C:\Private\invite.json
```

在接收端选择 **PC / NAS（设备共享）**，点击“发现局域网设备”，选择对应设备，再通过文件选择器导入其可信配对邀请并保存。也可直接导入邀请。邀请十分钟有效、单次使用；过期后分享端使用相同 `--state` 执行 `invite --out NEW_ABSOLUTE_PRIVATE_FILE` 重新签发。邀请包含访问授权秘密，须通过可信渠道传递。

发现使用 IPv4 UDP 49183，需要分享端 `--discoverable` 且防火墙允许局域网探测。响应签名绑定设备公钥与本次随机探测，但设备名称不是可信身份。选择设备后邀请必须与所选公钥匹配；发现本身不会授予文件访问权。客户端复制邀请到私有配对目录，首次连接后删除私有副本，随后使用已授权身份重连。原邀请文件由用户保存或移除。

跨网络可直接导入邀请；发现广播不会跨公网。数据传输沿用既有 iroh QUIC/TLS、UDP 打洞和加密中继回退。直连能否成功取决于 NAT、防火墙及网络策略，不能保证每个网络都打洞成功。自定义中继、网络策略、设备撤销见 [设备共享指南](../experiments/twodrive-dev/README.zh-CN.md)。未开启 `--discoverable` 的原服务器仍可通过邀请连接，但不出现在发现列表。

## 实际验证与范围

- 在线管理限制：Rust 与 Flutter 回归均证明旧逻辑失败、修复通过；实际隔离 desktop IPC 验证添加/编辑/删除其他配置不改变活动连接及其 revision，活动配置仍不可编辑/删除，未知连接 ID 不会断开当前账号。
- 实际本机 UDP + QUIC：发现、选择身份、导入邀请、私有副本权限、配对、条件写入、下载字节比对、改名、删除、重连及切回已有 WebDAV 通过。由 `scripts/test-storage-connections.py` 复现；不是独立公网 NAT 测试。
- WebDAV：局域网 HTTP 显式选择与地址范围测试、真实本机 HTTP/文件系统/FUSE/备份测试；未测试实体 NAS 的 WebDAV 实现。
- OneDrive：分页/不可信 continuation 边界沿用 Graph 引擎；新浅层目录和创建失败/条件覆盖请求有本地 HTTP 回归。真实个人账号通过隔离登录副本完成 8 MiB+123 字节上传/下载比对、条件覆盖/旧版本拒绝、改名、删除与自有目录清理，见[实际摘要](onedrive-desktop-validation-2026-10-14.json)；新浏览器登录、组织账号、真实 OneDrive FUSE/备份及原生 Windows 本轮未验证。
- Google：既有真实账号普通文件、FUSE 和备份验证见 Google 指南。本轮新增连接操作不要求重新登录 Google。

故障与交付记录：[TD-20261014-05](incidents.md#td-20261014-05连接任意账号后禁止添加或管理其他连接)。[源码提交 37f8f3b](https://github.com/LumiaBlack51/twodrive/commit/37f8f3b69d25395952cb45e71bf5a1110fa91063)；[最终编译提交 863bf6d](https://github.com/LumiaBlack51/twodrive/commit/863bf6d6218853d79eecb57be6be8c8c2a4f6cef)。[Development 0.1.2 Release](https://github.com/LumiaBlack51/twodrive/releases/tag/dev-v0.1.2) 已发布；见[最终构建与范围](development-0.1.2.md)。

## English

The Development desktop supports adding and managing inactive connections while another account is connected. Clicking a saved row switches the active connection with pending-write protection. Configure WebDAV/NAS using its service URL and credentials; HTTPS is the default. Numeric private-IP HTTP requires explicit opt-in and uses OS routing without claiming VPN bypass. OneDrive uses the existing Microsoft OAuth/Graph engine with independent Development account state; Google setup is described in its guide.

For PC/NAS sharing, run the updated `twodrive-dev serve` on the sharing device with an explicit directory, optional `--write`, and opt-in `--discoverable --device-name NAME`. The GUI scans IPv4 LAN UDP port 49183. Select the device and import a trusted, ten-minute, single-use invitation; discovery alone never authorizes access. Invitations can also connect across networks without discovery. QUIC/TLS transport attempts UDP NAT traversal and can fall back to encrypted relay. Native Windows, physical NAS and independent-network hole punching are separate unverified scopes for this change. The local regression scripts isolate their state and touch only disposable data.

Protocol references: [Microsoft upload session](https://learn.microsoft.com/graph/api/driveitem-createuploadsession?view=graph-rest-1.0), [driveItem conflict policy](https://learn.microsoft.com/graph/api/resources/driveitem?view=graph-rest-1.0).
