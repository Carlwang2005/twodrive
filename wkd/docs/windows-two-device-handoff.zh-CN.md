# 双 Windows、低流量跨 NAT 实验交接

日期：2026-10-28。交接给另一台高性能 Windows 笔记本的操作者或本机编码助手。
此文档是待执行方案，不是双机测试通过记录。只在个人 fork 的工作分支上提交；不要合并 PR #46，也不要直接改上游 dev。

## 1. 固定设备与网络角色

| 代号 | 设备 | 测试时网络 | 首轮角色 |
| --- | --- | --- | --- |
| A | 原来的 Windows 电脑，本会话所在设备 | 手机蜂窝热点 | connect 客户端 |
| B | 新的高性能 Windows 笔记本 | 家庭宽带 Wi-Fi 或网线 | serve 服务端、代码下载及构建端 |

手机关闭 Wi-Fi，确认热点上网来源为蜂窝网络；B 不连接这个热点。A 拔掉其他上网网线并断开家庭 Wi-Fi，不能让它自动切回家庭网络。
两台电脑放在身边，一个人分两个终端即可操作。先让 B 启动服务，随后在 A 配对、执行小文件回归；再交换服务端角色。
热点与家庭宽带的出口都要实测，不能仅凭 SSID 不同宣称独立 NAT。CGNAT 无法确认就填“未知”。

## 2. 流量约束：先下载与构建，再开始热点实验

- 所有 clone、Git/Rust/Python/编译器安装、Cargo 依赖、构建和产物下载都在 **B 的家庭宽带**完成。
- 使用 U 盘把同一份 exe、必要脚本、邀请文件传给 A。不要通过热点下载 Rust/Flutter、Cargo 依赖、仓库历史或 CI 构建。
- 热点阶段每轮只运行 `--small-count 0 --attempts 1`，**不用 `--full`、`--faults`、`--soak-seconds`，不复制真实文件**。
- 服务端增加 `--max-upload 1048576`，限制单次 PUT 为 1 MiB。这不是网络流量总限额，也不限制下载或系统后台联网。
- 计划四组各 5 次建连，共 20 轮：A→B auto、A→B relay-only、B→A auto、B→A relay-only。箭头表示客户端到服务端；每轮既上传也下载。
- 当前小语料为 0、4099、65537 字节，每轮上传、下载及 MOVE 后下载约 **0.20 MiB**，20 轮约 **4 MiB 文件正文**。HTTP、配对、中继发现和重传开销另计，不能保证蜂窝账单值。
- **全设备流量达到 50 MiB 暂停检查；达到 100 MiB 停止本轮，不自动重试。**这是一项人工操作阈值，不是脚本或运营商硬限额。每轮前后记录；同时查看手机热点用量。
- 关闭云盘同步、视频、游戏下载等后台大流量任务，Windows 将热点设为按流量计费连接。不要为了测试关闭安全防护。
- 一次失败先保存结果，排查后最多额外重试一次；同组第二次失败停止该组，余下轮次标阻塞。不要无限重新配对。
- 两小时循环、100 MiB/1 GiB、千文件、并发和大多数故障注入放在之后的**双机同一局域网**阶段。它们不替代热点跨 NAT 证据。

## 3. B 下载仓库，核对基线

在 **B** 的 PowerShell 中执行。目标目录应为不存在的新目录；已有同名目录时先检查，不覆盖。

```powershell
git -c core.longpaths=true clone --depth 1 --single-branch --branch codex/windows-two-device-handoff-20261028 https://github.com/Carlwang2005/twodrive.git C:\TwoDriveLab\repo
Set-Location C:\TwoDriveLab\repo
git status --short --branch
git rev-parse HEAD
git remote -v
```

仓库入口是 [个人 fork](https://github.com/Carlwang2005/twodrive)，交接分支为 `codex/windows-two-device-handoff-20261028`。
传输基线来自上游 dev `0fc9e1203433144fcd8a69ce0580123880b50d1d`；测试工具提交为 `4a7b77f3027e3dfd85d3089c2b1adc16bdeb5f7e`，交接分支后续仅添加文档、整理 wkd 路径及相应 CI 引用；未修改生产传输。
记录当前实际 HEAD，后续两端使用同一 exe；不要自动更新到其他分支。浅克隆不一定包含父提交，不能以 `git show 4a7b77f` 不可用判断源码不对。

首先阅读：

1. 根目录 `AGENTS.md`、`CONTRIBUTING.md`。
2. `experiments/twodrive-dev/README.zh-CN.md`，CLI 与配对说明。
3. `doc/provider-boundaries-2026-10-13.md`，前后端与适配器边界。
4. `wkd/validation/udp-2026-10-25/README.zh-CN.md` 与 `doc/incidents.md`。
5. `doc/peer-windows-egress-2026-10-12.json`，了解历史真机记录的验证边界。

不使用 OneDrive Full/Lite，不启动真实账户或原有 TwoDrive 服务。

## 4. B 构建并向 A 离线交付

B 需有 Git、Python 3、Rust（包要求至少 1.91）和 MSVC C++/Windows SDK 构建环境。缺少依赖时只通过家庭宽带补齐；记录实际版本。
暂不安装 Flutter，先完成独立 CLI 验证。

```powershell
Set-Location C:\TwoDriveLab\repo
python --version
rustc --version
cargo --version
$env:RUSTFLAGS = '-C target-feature=+crt-static'
cargo build --manifest-path experiments/twodrive-dev/Cargo.toml --release --locked --bin twodrive-dev
if ($LASTEXITCODE -ne 0) { throw '构建失败，先排查，不开始热点实验' }
python wkd/scripts/test-reliability.py
if ($LASTEXITCODE -ne 0) { throw '测试工具回归失败' }
Get-FileHash experiments/twodrive-dev/target/release/twodrive-dev.exe -Algorithm SHA256
```

这与仓库 Windows CI 的静态 CRT 构建参数相同。不要添加 Linux 的 `--features mount`。
把该 exe 和 `wkd/scripts/reliability.py`、`wkd/scripts/test-reliability.py`、本文档通过 U 盘复制到两端的 `%LOCALAPPDATA%\TwoDriveLab\bin`。
脚本存入其中的 `scripts` 子目录。两端各执行 `Get-FileHash`，SHA-256 必须完全相同；B 本地构建的散列不必等于过去 CI 的散列，但必须明确记录来源。
本机旧 CI exe 的 SHA-256 为 `b1185b615eb63b266cc30baf3cfb7dbf8c5b1746e68117dcece92b1af034de17`，不要一端用旧 CI、另一端用新构建。

## 5. 两端网络前置检查

在两个设备上分别完成，开始传输前再复核一次：

- 正常退出 PulseVPN、Clash 等代理软件；确认核心进程和 TUN/VPN 服务未继续接管网络。原机曾两次停止后仍检出 PulseVPN 进程；这是未解决的前置阻塞，不能只关闭窗口就继续。
- 记录个人原配置，再关闭 Windows 静态系统代理、设置脚本/PAC 和自动代理发现；实验后恢复。以下命令只记录，不替你修改它们。
- 保持防火墙开启；若拦截，只放行经哈希核验的测试 exe，且使用当前网络配置文件。不要关闭所有防火墙或允许公网 HTTP/WebDAV。
- 清除测试 PowerShell 自身的代理环境变量。关闭这个独立终端后不影响其他终端；不要清除系统级持久变量。
- 核对适配器、IPv4/IPv6 默认路由和实际出口。原始记录保存在私有实验目录，分享前脱敏。

两端各开一个独立测试终端，执行以下初始化；后续命令都使用这些变量：

```powershell
$Lab = Join-Path $env:LOCALAPPDATA 'TwoDriveLab'
$Exe = Join-Path $Lab 'bin\twodrive-dev.exe'
$Runner = Join-Path $Lab 'bin\scripts\reliability.py'
New-Item -ItemType Directory -Force (Join-Path $Lab 'private'),(Join-Path $Lab 'reports') | Out-Null
Get-ChildItem Env: | Where-Object Name -Match '^(HTTP_PROXY|HTTPS_PROXY|ALL_PROXY|NO_PROXY)$' | Remove-Item
Get-FileHash $Exe -Algorithm SHA256
Get-NetFirewallProfile | Select-Object Name,Enabled
Get-NetAdapter | Select-Object ifIndex,InterfaceDescription,Status
Get-NetRoute | Where-Object DestinationPrefix -in @('0.0.0.0/0','::/0') | Select-Object InterfaceIndex,DestinationPrefix,RouteMetric
netsh winhttp show proxy
Get-ItemProperty 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings' | Select-Object ProxyEnable,@{Name='StaticProxyPresent';Expression={[bool]$_.ProxyServer}},@{Name='PACPresent';Expression={[bool]$_.AutoConfigURL}}
Get-Process | Where-Object ProcessName -Match 'PulseVPN|clash|v2ray|sing-box|wireguard|openvpn|tailscale' | Select-Object ProcessName,Id
```

以上进程名匹配不是完整 VPN 探测器；还需核对你实际安装的软件、路由和服务。若仍有接管流量的组件，记录“环境阻塞”，先停止实验。
记录 Windows 自动代理发现设置的实际状态，不能仅用 PACPresent=false 证明 WPAD 已关闭。
两端可各做一次小型出口查询，保存在本机私有目录：

```powershell
curl.exe --noproxy '*' --fail --max-time 10 https://api.ipify.org -o (Join-Path $Lab 'private\exit-ip.txt')
if ($LASTEXITCODE -ne 0) { throw '实际出口查询失败，暂不作独立出口结论' }
```

该请求向外部查询服务暴露出口 IP，不上传凭据或文件。比较 A/B 结果与网络接线，脱敏报告只写出口是否不同、运营商/网络类别及 CGNAT 是否确认；不要把原始地址上传 Git。

流量采样：在 A 上从前面的适配器表找到**热点 Wi-Fi 的接口名称**，把下一行的值改成实际名称。第一次执行整段；之后每轮前后只重复从 `$MeterNow` 开始的三行，累计值达到上述阈值即暂停/停止：

```powershell
$MeterInterface = 'Wi-Fi'
$MeterStart = Get-NetAdapterStatistics -Name $MeterInterface
$MeterNow = Get-NetAdapterStatistics -Name $MeterInterface
$UsedMiB = (($MeterNow.ReceivedBytes + $MeterNow.SentBytes) - ($MeterStart.ReceivedBytes + $MeterStart.SentBytes)) / 1MB
'热点接口累计收发：{0:N2} MiB' -f $UsedMiB
```

保留 `$MeterStart`，不要每轮重置。接口重启/计数器重置会使读数失效，出现负值应重新记起点并保留先前消耗。该值含本机背景流量、未含手机自身全部流量，以手机记录辅助判断。

## 6. 四组跨 NAT 小流量测试

每组使用新的组名和全新的服务端/客户端状态目录，避免邀请被重复消费或旧授权混用。
初始组名 `ab-auto`：A 是客户端，B 是服务端。两端分别设置：

```powershell
$Group = 'ab-auto'
$Network = @()
$GroupRoot = Join-Path $Lab ('private\' + $Group)
if (Test-Path -LiteralPath $GroupRoot) { throw '组目录已存在，先检查或改用新组名，不能覆盖旧证据' }
New-Item -ItemType Directory -Path $GroupRoot | Out-Null
```

### 6.1 服务端：B 首先运行

```powershell
$ServerState = Join-Path $GroupRoot 'server'
$Share = Join-Path $GroupRoot 'share'
$Ticket = Join-Path $GroupRoot 'invite.json'
New-Item -ItemType Directory -Path $Share | Out-Null
& $Exe --state $ServerState serve --root $Share --write --max-upload 1048576 --invite-file $Ticket @Network 2>&1 | Tee-Object -FilePath (Join-Path $GroupRoot 'server-private.log')
```

保持该终端运行。通过 U 盘或现有可信私密渠道把 invite.json 交给客户端，放到客户端的 `$GroupRoot\invite.json`；十分钟内兑换，只可使用一次。
邀请不能提交 Git、发到公开 PR 或粘贴给无权访问共享的人。配对后删除两端该邀请的副本及 U 盘副本，保留私有配对状态用于第 2–5 次连接。

### 6.2 客户端：A 运行第 1–5 轮

在 A 同一初始化终端运行。**每次只执行一轮，不写五次无间隔自动循环**，留出流量与路径检查时间。

```powershell
$Round = 1
$ClientState = Join-Path $GroupRoot 'client'
$ConnectArgs = @('--state', $ClientState, 'connect', '--listen', '127.0.0.1:4918') + $Network
if ($Round -eq 1) { $ConnectArgs += @('--ticket-file', (Join-Path $GroupRoot 'invite.json')) }
$Stdout = Join-Path $GroupRoot ("connect-$Round-private.stdout.log")
$Stderr = Join-Path $GroupRoot ("connect-$Round-private.stderr.log")
$Started = Get-Date
$ProcessArguments = ($ConnectArgs | ForEach-Object { '"' + $_ + '"' }) -join ' '
$ClientProcess = Start-Process -FilePath $Exe -ArgumentList $ProcessArguments -PassThru -WindowStyle Hidden -RedirectStandardOutput $Stdout -RedirectStandardError $Stderr
```

上面将每个参数加引号，支持用户名/状态路径中的空格；状态和邀请参数使用具体目录/文件名，不以反斜杠结尾。
如后台启动仍失败，改用前台方式 `& $Exe @ConnectArgs`，并在第二个已初始化的终端执行测试、在前台 Ctrl+C 停止；不得把启动参数错误归为网络失败。
原始日志可能有 IP、公钥和私有路径，只能在私有目录保留。

最多等待 45 秒：读取 stdout 中的 `WebDAV: http://127.0.0.1:4918/`，同时确认该端口的拥有 PID 就是本轮 `$ClientProcess.Id`；仅 credentials.json 存在不足以判断准备完成。
记录从 `$Started` 到首次观察到 WebDAV 就绪的时间（标明人工观察误差）；45 秒未就绪按失败收集日志并停止自有进程，不永久等待。
发现端口已被其他程序占用则换一个双方记录好的回环端口，并同步改下面的 --gateway-port；不要停止不属于本实验的程序。

```powershell
Get-NetTCPConnection -LocalPort 4918 -State Listen -ErrorAction SilentlyContinue | Select-Object LocalAddress,LocalPort,OwningProcess
python $Runner --gateway-port 4918 --credentials (Join-Path $ClientState 'credentials.json') --attempts 1 --small-count 0 --timeout 30 --report (Join-Path $Lab ("reports\$Group-$Round.json"))
$RoundExitCode = $LASTEXITCODE
```

每轮检查报告和 CLI 日志。记录：成功/失败、就绪耗时、初始 transport、RTT、文件大小/SHA-256、PUT/GET/MOVE/Range/DELETE、热点累计用量。
运行退出码非零即失败，不能因 UI 或 CLI 出现过“连接成功”就算通过。
完成后仅停止此轮自有客户端并等待退出，确认回环端口释放：

```powershell
if (-not $ClientProcess.HasExited) { Stop-Process -Id $ClientProcess.Id; $ClientProcess.WaitForExit() }
Get-NetTCPConnection -LocalPort 4918 -State Listen -ErrorAction SilentlyContinue | Select-Object OwningProcess
```

下一轮把 `$Round` 改为 2，再执行 6.2；依次到 5。保留同一 ClientState，但第 2–5 轮不再传 --ticket-file。
这五轮是五次独立建立 QUIC 连接，不是五次重新配对。服务端保持运行，客户端必须确实退出后才重启。

### 6.3 切换组和角色

每组结束在服务端 Ctrl+C 停止自己的 serve，再核对自有文件、进程、端口；不要继续闲置中继连接。

| 组名 | 服务端 | 客户端 | 两端 `$Network` |
| --- | --- | --- | --- |
| ab-auto | B 家庭网络 | A 热点 | `@()` |
| ab-relay | B 家庭网络 | A 热点 | `@('--relay-only')` |
| ba-auto | A 热点 | B 家庭网络 | `@()` |
| ba-relay | A 热点 | B 家庭网络 | `@('--relay-only')` |

每一组两端用新组目录；重复 6.1/6.2。relay-only 必须两端都加选项，初始 transport 应为 relay，否则记录失败；auto 允许 relay 回退，不能要求一定打洞成功。
这不是 `--no-relay` 的跨 NAT 测试；不应禁用发现后盲目重复尝试。另行安排双机 LAN 的 --no-relay 对照。

## 7. 路径证据缺口与明确停点

当前基线 CLI 只在配对完成时打印一次 transport/Selected path。`network-route direct` 仅表示物理出口策略，不能证明 UDP 直连。
上述流程可以补充“真实两机、独立出口、首次路径和文件完整性”证据，**仍不能验收整个文件传输始终直连**。
网关模式报告保守保留 independent_nat_established=false、continuous_path_evidence=false；不要手工改成 true 来掩盖采集能力缺失。另用双机拓扑记录关联说明实际环境。

另一台编码助手如继续开发，应先在后端增加有界、脱敏的路径遥测：连接实例、时间戳、selected direct-udp/relay/negotiating、RTT、重连/路径切换；测试期间定时采样并保留变化。不要输出密钥、邀请、原始地址或给每个数据包刷日志。
新增回归尽量脱离 GUI，完成后在 B 构建并通过 U 盘向 A 同步新 exe，重新核对 SHA-256。开发期间关闭热点测试进程；未经验证的新输出不能作为已具备的功能写入结论。
本次交接不授权合并 PR 或扩大蜂窝流量预算；不要求现在一定改出该能力。做不到就标“未测/证据不足”。

## 8. 局域网补测及桌面阶段

结束热点阶段，停止两端自有进程并记录消费，再将 A 与 B 接到同一家庭局域网。确认不再走蜂窝出口后，才能取消 1 MiB 上传限制，运行大文件、千文件、并发与两小时循环。
两小时应补采两端 CPU/工作集/句柄/连接数和吞吐时间序列，记录中断后的可用性；仅 `--soak-seconds 7200` 的轮次完成标记不能证明无泄漏。
0/50/99% 关闭请求不等于真实断网、退出客户端或停止服务端，相关恢复组合要分开测试。当前 PUT 无断点续传能力，完整重传不能写成续传成功。
磁盘不足/权限不足需隔离可丢弃环境；不得填满系统盘或修改个人目录权限。
CLI 证据完成后再构建 apps/dav，验证它调用同一后端的进度、错误、取消、已有暂停恢复、重连和关窗任务行为；不能在 Flutter 重写传输逻辑。
Windows 挂载/虚拟盘作为单独适配器范围，不因未实现而判传输失败。

## 9. 交回材料

每组五轮一行摘要，失败和未执行轮次也保留；附两端系统/版本、提交 SHA、exe SHA、拓扑/出口脱敏对比、代理关闭记录、每轮连接耗时及流量增量。
逐文件校验 JSON 保留；原始 stdout/stderr、exit-ip.txt、邀请和设备状态只存私有目录，脱敏后才能分享。禁止把整个 TwoDriveLab/private、Cargo target 或真实私人文件清单提交仓库。
新增故障依 AGENTS.md 记录 incidents.md，首轮 1 GiB 超时目前仍根因未知；不能把新测试通过写成旧问题已修复。
恢复个人代理/VPN/PAC 配置；只清理自己创建的临时共享、状态、邀请和防火墙规则。未知残留先记录，不批量删除。
所有新增改动仅推个人 fork 的工作分支；交接时给分支和提交链接，不执行合并。
