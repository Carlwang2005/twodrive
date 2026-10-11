# TwoDrive 故障记录

从 2026-09-15 起持续维护。每次故障新增条目，最近的条目放在前面；复发也单独记录并关联历史条目。维护流程见 [AGENTS.md](../AGENTS.md)。更早的调查保留在[文档索引](README.md#engineering-notes)，不追溯标记为已按本流程验证。

## 记录模板

复制以下模板填写。未知项明确写“未确认”或“未执行”，不要省略验证边界。

```markdown
## TD-YYYYMMDD-NN：故障标题

- 日期：
- 状态：调查中 / 已定位待修复 / 已修复待验证 / 已验证（注明范围）
- 影响版本与环境：
- 关联历史故障：无 / 条目编号或调查链接

### 症状与影响

用户可观察到的错误、影响的操作及数据影响；区分已确认影响与潜在风险。

### 触发条件与复现

最小操作步骤、必要状态、复现是否稳定。

### 根因与证据

出错的代码路径、因果链、必要的脱敏日志，以及排除或尚未排除的解释。
根因未确认时写明当前假设和缺少的证据。

### 解决办法与恢复操作

代码修改及其原理；如有本地恢复操作，另述操作范围和结果。

### 验证结果与边界

实际运行的测试、旧代码与修复后的结果、模拟/真实环境、未执行的验证。

### 防复发措施与后续

回归测试名称、需要保留的约束、未完成事项；复发时说明与历史修复的关系。

### 交付记录

已核实的提交、Release 和详细调查链接；尚未提交或发布时明确注明。
```



## TD-20261011-14：新控制客户端未绑定当前请求的签名响应，租期末回执易失效

- 日期：2026-10-11
- 状态：已定位 / 修复验证中；审查发现，未发生实机攻击。
- 影响版本与环境：c3940ea 的尚未实机启用 test-agent 控制协议。
- 关联历史故障：TD-20261011-11 的控制工具边界审查；与 Windows 路径错误不同。

### 症状与影响

客户端只以会话与序号判断完成，新客户端若先读到旧的有效签名响应，重用其序号后可能把该旧序号的另一操作回执当作当前结果。报告有效期等于进程租期，收集租期结束后清理回执时会被判过期。

### 触发条件与复现

合成控制响应保留同一会话/序号，替换为旧指令的签名回执；不是实机入侵证据。租期退出需要先停止/回收 child，写回执时可能已超过原到期秒。

### 根因与证据

响应未回显所执行请求的设备签名；序号在单一 agent 内防执行重放，但不能独自证明回执属于当前指令。清理回执有效期与执行授权租期未分离。

### 解决办法与恢复操作

签名报告回显实际请求的 signature，客户端校验请求与回执严格相同；报告有效期增加五分钟收集窗口，绝对 agent/child 租期不延长。原服务/实机未更新，无恢复操作。

### 验证结果与边界

旧 c3940ea 客户端在合成回执用例中实际失败（未抛拒绝错误）；修复后的拒绝与正确回执两项 Python 回归通过。真实 QUIC 控制 smoke 和本机完整生命周期通过，请求签名回显已端到端匹配；lease_expires 与报告 expires 相差 300 秒的集成断言通过。最终 Windows 构建待验证；不会把协议审查发现说成实际入侵。

### 防复发措施与后续

保留同会话/序号旧回执拒绝用例，真实 QUIC 控制 smoke 必须匹配当前请求；租期与回执收集窗口分别验证。

### 交付记录

待提交和原生验证。

## TD-20261011-13：修正路径后的 Windows 控制生命周期仍未通过

- 日期：2026-10-11
- 状态：已修复；Windows 完整生命周期已通过，release 构建中。
- 影响版本与环境：53e4b24，原生 Windows CI 38141818742。
- 关联历史故障：TD-20261011-12 的路径赋值原生回归已通过；完整生命周期另一次失败，需按新阶段核实。

### 症状与影响

Linux 原生完整流程通过；Windows cargo test 仍失败，未交付实机控制包、未推进 dev。

### 触发条件与复现

完成 ProviderPath 修正后再次执行完整 Windows bootstrap 测试，等待具体日志。

### 根因与证据

已确认相邻机制遗漏：ProviderPath 返回的原生 verbatim 路径能供 Rust/.NET 使用，但 PowerShell 5.1 Join-Path 解析该路径时没有 PSDrive，抛 PSArgumentNullException/drive null。CI 精确定位 bootstrap:16，在读取身份/授权 hash 前失败；尚未进入 Rust ready。不是 TD-20261011-12 原生路径字符串问题复发。

### 解决办法与恢复操作

bootstrap 改用 .NET Path.Combine、File/Directory 与流式 SHA256，避免把有效原生路径再次交给 PowerShell provider 路径拼接；原生回归扩展到实际 hash 函数、旧 Join-Path 失败。实机未更新，没有实机恢复。

### 验证结果与边界

旧 53e4b24 原生生命周期在 Join-Path 处失败。c3940ea 的原生 5.1 实际赋值/流式 hash 回归通过（38142327437）；CI 38142329825 的 Windows 单元、9 项既有集成、2 项控制 CLI 完整 bootstrap/child 回归已通过，release 正在构建。

### 防复发措施与后续

按具体阶段保留失败证据，修复和原生验证后再交付。

### 交付记录

[CI 38141818742](https://github.com/LumiaBlack51/twodrive/actions/runs/38141818742)；修正待补充。

## TD-20261011-12：远程控制首次 Windows 原生生命周期回归失败

- 日期：2026-10-11
- 状态：已确认路径机制并通过原生 5.1 回归；完整生命周期复测中。
- 影响版本与环境：ee7bdd9/e7f3bbc，原生 Windows CI 38141236619/38141388630 的 cargo test。
- 关联历史故障：TD-20261011-11 新控制工具验证；不能仅以测试失败认定与 TD-20261011-10 网络退出相同根因。

### 症状与影响

Windows 严格编译检查通过，但 cargo test 失败；控制端尚未交付 Windows 实机，dev 未推进。

### 触发条件与复现

第一次执行原生 Windows bootstrap/child 完整生命周期回归。待读取具体失败日志。

### 根因与证据

原生日志显示首次 ready 前退出：`The filename, directory name, or volume label syntax is incorrect. (os error 123)`，35 秒后等待报告断言失败。CI 38141820760 的 Windows PowerShell 5.1 回归确认：Resolve-Path.Path 将 Rust 的 verbatim 状态路径表示为 provider-qualified 字符串，原生 IO 拒绝该字符串；ProviderPath 返回可被原生 IO 使用的真实路径，执行 bootstrap 中的实际赋值语句通过。

### 解决办法与恢复操作

改用 ProviderPath，给状态、queue 和签名报告写入补充阶段上下文；保留原生 verbatim 路径回归与完整 bootstrap 测试。没有实机恢复操作。

### 验证结果与边界

本机完整单元/集成与 QUIC 控制通道通过。旧原生完整生命周期在 ready 前失败；修正后的 5.1 真实路径赋值回归通过，完整生命周期与 release 仍待 CI 38141818742。

### 防复发措施与后续

保留首次 bootstrap 完整原生测试，修复后重新验证再交付。

### 交付记录

[失败 CI 38141236619](https://github.com/LumiaBlack51/twodrive/actions/runs/38141236619)、[同源码机制验证 CI 38141388630](https://github.com/LumiaBlack51/twodrive/actions/runs/38141388630)；[修正 53e4b24](https://github.com/LumiaBlack51/twodrive/commit/53e4b24fc8220360ed906deb9ab63f410238b4ab)、[原生路径回归 CI 38141820760](https://github.com/LumiaBlack51/twodrive/actions/runs/38141820760)。

## TD-20261011-11：跨端验证切换失败后未保留 child 退出证据，需反复人工操作

- 日期：2026-10-11
- 状态：已定位，远程测试控制实现验证中。
- 影响版本与环境：至 7337283 的临时 Windows 验证脚本；旧 serve 仅提供文件传输。
- 关联历史故障：TD-20261011-07/09/10；07/09 根因已分别验证，10 的核心退出原因仍未确认。

### 症状与影响

用户多次执行脚本和复制日志；最后的 Windows 兜底启动失败没有留下 stderr，阻断自主定位和完整验证。没有推进 dev。

### 触发条件与复现

脚本重启唯一传输端执行分阶段测试；child 退出后 catch 只保存异常类型，finally 删除 runtime 日志。实际 TD-20261011-10 报告未提供核心错误。

### 根因与证据

测试进程与控制传输生命周期耦合；退出错误和日志未进入报告。脚本没有通用的远程重试/诊断机制。这是测试工具缺陷，不据此断言核心网络退出根因。

### 解决办法与恢复操作

用户明确授权实现远程控制。增加 opt-in、绝对租期的 test-agent，通过旧配对共享接收设备签名的 start/snapshot/stop/finish 请求；响应由原 Windows 身份签名，随机会话、序号和有效期防重放。旧 serve 不重启，child 使用独立临时身份、共享与签名指定的临时客户端；保存有限长度 stdout/stderr 和退出码。候选构建复制到私有 runtime，核对签名请求的 SHA256 后执行。Windows 使用独立进程 Job，关闭控制句柄时终止自有子树。

本地恢复：此前三轮 bundle 已删除，原配对状态及旧 serve 保留；首次 bootstrap 是旧版没有执行 API 的技术边界。

### 验证结果与边界

本机 18 项单元、10 项既有集成与 2 项控制 CLI 集成全部通过，严格 Clippy/格式检查通过。实际 child 退出错误保留、失败后重启、重放拒绝、finish 清理及原身份/授权字节不变通过。实际 QUIC/WebDAV 双通道 smoke 也通过：签名指令经过旧服务通道，临时客户端与 child 独立身份读写，停止 child 后旧传输仍可用。Windows 原生完整 bootstrap 与实机控制仍待完成。提供测试进程管理，不开放监听任意 shell；签名 owner 可授权运行候选构建，候选仍具有启动者的系统权限。没有 Windows 核心兜底成功结论。

### 防复发措施与后续

保留签名/篡改/会话/过期/重放/未知指令回归、实际 child 失败日志捕获与失败后重试、退出清理和原身份/授权字节不变测试。实机取回 TD-20261011-10 退出原因再修复、验证并推进 dev。

### 交付记录

[控制端提交 ee7bdd9](https://github.com/LumiaBlack51/twodrive/commit/ee7bdd9267fb52d4a04d05f1f2c7fadf03b3cc51)；原生构建和实机验证待补充。

## TD-20261011-10：Windows 默认出口通过后兜底阶段启动失败

- 日期：2026-10-11
- 状态：调查中 / 根因未确认。
- 影响版本与环境：e3dc0b7 原生核心、7337283 Windows 脚本；用户校园网 TUN 与 Linux 热点 TUN。
- 关联历史故障：TD-20261011-07/09 的脚本序列化与读取已通过该次实机默认阶段；本条记录后续 Windows 兜底阶段失败。

### 症状与影响

Windows auto_ready 报告正常；双端默认 auto 与 Linux 人为无网卡兜底各完成 2 MiB 全量哈希、MOVE、Range、DELETE。随后 Windows 兜底阶段报告 RuntimeException，尚未完成 Windows 兜底数据验证，dev 未推进。

### 触发条件与复现

同一脚本先停止默认 serve，再以不存在网卡参数启动 auto serve，预期系统出口兜底。固定 next 信号已发送；用户确认具体警告为 `Test server exited during fallback startup.`。

### 根因与证据

根因未确认。启动检查检测到 child 已退出，报告却只保留 error_type，finally 删除了 stderr，缺少核心退出原因。不能以 RuntimeException 或 30 秒注册预算猜测根因。

### 解决办法与恢复操作

待定位。失败报告已收集，四项恢复/清理标志为 true；Linux 删除自有 bundle 后 GET 404、共享根目录可见项为 0，持久身份和配对保留。默认阶段实际路由选择 aps1、初始 RTT 115.1 ms；用户授权增加远程控制，采用旧 serve 保持传输、独立子进程状态与带设备签名的测试指令，取回退出日志后再定位。

### 验证结果与边界

默认和 Linux 兜底阶段已实际通过，Windows 默认公网 TCP 实际采样 94/94 为物理源地址、TUN fake-IP 存在、路由未变化。Windows 兜底尚未通过，不报告整轮成功。

### 防复发措施与后续

补充启动阶段诊断与对应原生回归，获得实际 Windows 兜底结果后才能推进 dev。

### 交付记录

待补充修正和整轮实机记录。

## TD-20261011-09：修正报告字符串后读取正在重定向的 Windows 日志失败

- 日期：2026-10-11
- 状态：已验证原生活跃日志共享机制，后续实机默认阶段读取与报告成功。
- 影响版本与环境：验证分支 e3dc0b7 的 verify-network.ps1，用户 Windows PowerShell 实机。
- 关联历史故障：TD-20261011-07 的报告修正遗漏实际 Start-Process 日志写入共享条件；不是 JSON 内存异常复发。

### 症状与影响

新脚本在 ReadAllLines(auto.stderr) 处报告文件正由另一进程使用；随后 Validation ended 并返回提示符。新版 serve 和网络验证未完成，dev 未推进。

### 触发条件与复现

Start-Process -RedirectStandardError 的日志 writer 仍保持打开时，报告采样读取同一文件。原生 CI 的合成日志写完即关闭，没有覆盖持续写入句柄。

### 根因与证据

已确认：ReadAllLines 的默认共享方式没有允许已经存在的写入访问，导致 Windows sharing violation。原生 CI 用实际活跃 Start-Process 重定向 writer 复现旧函数 IOException/HRESULT 32，新 FileStream 共享方式通过。用户提供原生 MethodInvocationException，明确路径为本轮 auto.stderr；需用实际 Start-Process 活跃重定向的原生对照验证。

### 解决办法与恢复操作

已用显式允许 ReadWrite/Delete 共享的 FileStream/StreamReader 读取纯字符串，保持报告不带 provider 属性。新增活跃 Start-Process 原生回归，待执行。实机失败报告已收集：identity_and_grants_preserved、original_server_restarted、bundle_payloads_removed、temporary_runtime_removed 全为 true；Linux 删除自有报告/control bundle 后 GET 404，root 可见项为 0。身份与授权目录保留。

### 验证结果与边界

此前静态日志原生回归已通过、实机活跃日志条件失败。CI 38138971808 对旧 ReadAllLines 实际复现共享冲突；修复后的 PowerShell 5.1 与 7 在子进程仍存活、重定向 writer 仍打开时，读取与实际 Wait-Stage/Save-Report 均通过，fixture 子进程和目录已清理。Rust 源码与 e3dc0b7 完全相同，沿用 CI 38137498647 原生 binaries，不将脚本专用 CI 说成新的核心构建；校园网实机默认阶段 47 次采样/94 条物理 TCP，报告正常；后续兜底 child 退出另记 TD-20261011-10。

### 防复发措施与后续

原生测试应使用仍在运行的 Start-Process 重定向 child，验证旧读取失败、新读取成功，并停止 child、删除自有测试目录；实机完成后才能推进 dev。

### 交付记录

[修正提交 7337283](https://github.com/LumiaBlack51/twodrive/commit/733728343cb6b99b70a56f668265b00c0ea2cbfa)；[原生活跃 writer CI 38138971808](https://github.com/LumiaBlack51/twodrive/actions/runs/38138971808)；核心构建 [CI 38137498647](https://github.com/LumiaBlack51/twodrive/actions/runs/38137498647)。

## TD-20261011-08：DNS 并发回归新增后原生 CI 检查失败

- 日期：2026-10-11
- 状态：已验证两平台严格 Clippy、回归与 release CI。
- 影响版本与环境：验证分支 254918b，原生 CI 38137377190。
- 关联历史故障：TD-20261011-06/07 的修复验证；不是用户 serve 内存故障。

### 症状与影响

Linux job 在严格 Clippy 阶段失败，尚未进入 release/实机复测，dev 未推进。

### 触发条件与复现

对验证分支执行双平台原生 CI，具体诊断待读取。

### 根因与证据

提取 prefer_records 后，Backend::query 仅向 FuturesUnordered::push 提供共享引用，原来的 let mut 已不再需要。严格 -D warnings 将 unused_mut 当作错误；Linux 原生日志定位 src/network/dns.rs:59。不是运行期内存或地址发现故障。

### 解决办法与恢复操作

移除多余 mut，保持严格检查；用户设备未重启，无数据恢复操作。

### 验证结果与边界

CI 38137377190 的两平台原先严格 Clippy 失败；修正后的 CI 38137498647 两平台全部通过。Linux 15 单元/10 集成，Windows 10 单元/9 集成，严格检查、release 和 LAN smoke 均通过；原生报告 5.1/7 回归通过。校园网/TUN 实机仍待复测。

### 防复发措施与后续

修正具体检查失败后运行对应双平台检查。

### 交付记录

[失败 CI 38137377190](https://github.com/LumiaBlack51/twodrive/actions/runs/38137377190)；[修正提交 e3dc0b7](https://github.com/LumiaBlack51/twodrive/commit/e3dc0b76a5b5ad274b6ac809003762146d359842)；[通过 CI 38137498647](https://github.com/LumiaBlack51/twodrive/actions/runs/38137498647)。

## TD-20261011-07：Windows PowerShell 5.1 验证报告写入内存耗尽

- 日期：2026-10-11
- 状态：已验证原生 5.1 报告机制；校园网/TUN 实机复测待执行。
- 影响版本与环境：验证分支 bdab13e 的 verify-network.ps1；用户以 powershell.exe 启动，原生 CI 此前仅用 pwsh 解析语法。
- 关联历史故障：TD-20261011-06 同轮切换不可达；本条记录独立的测试脚本故障，不将其当成 Rust serve 内存耗尽。

### 症状与影响

用户报告验证窗口警告 System.OutOfMemoryException。auto.stdout 已出现 Serving device，auto.stderr 已出现物理网卡选择；windows-report.json 未生成。脚本最终恢复和清理状态尚未确认，dev 未推进。

### 触发条件与复现

脚本将 Get-Content 返回的网络日志行存入报告，再调用 ConvertTo-Json -Depth 10。用户原生系统运行一次失败，尚未取得完整异常栈。

### 根因与证据

已确认脚本的报告数据边界缺陷：PowerShell 5.1 的日志字符串带 PSDrive/PSProvider 等扩展属性，深层 JSON 序列化递归展开文件系统对象。[PowerShell #6388](https://github.com/PowerShell/PowerShell/issues/6388)记录相同 Get-Content / Depth 10 路径；[官方文档](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.utility/convertto-json?view=powershell-7.6)说明新版字符串序列化与旧版不同。原生 5.1 已执行旧 Wait-Stage/Save-Report（将 JSON 深度限到 3，避免 OOM），稳定违反“network_messages 只能为纯字符串”的断言；新实际函数以原深度 10 成功写入紧凑报告。

### 解决办法与恢复操作

已改用纯 .NET 日志字符串生成报告，并增加 powershell.exe 实际 Wait-Stage/Save-Report 运行测试，原生 CI 待执行。Linux 本轮验证客户端已停止。用户后续提供异常栈，明确 Save-Report 第 49 行 ConvertTo-Json 失败且已返回提示符；fresh 已配对 Linux 网关重新连通 Windows，原 bundle 的 exe、脚本与报告均 GET 404，root 仅有本轮 bundle。Windows 用户只读复查确认 TemporaryRuntimePresent=false、ServerCount=1；Linux 已删除空的自有 bundle，GET 404、root 可见项为 0，持久身份目录保留。

### 验证结果与边界

CI 38137498647 的 Windows 原生运行步骤已通过：旧 5.1 报告有界失败，新 5.1 与 pwsh 实际 Wait-Stage/Save-Report 测试成功，验证单行、多行、空与不存在日志。网络 API 使用隔离 fixture，未操作真实代理或服务；该测试证明序列化机制，校园网/TUN 实机复测待执行。

### 防复发措施与后续

用隔离合成日志验证实际 Wait-Stage/Save-Report 的紧凑 JSON 与纯字符串字段，包含缺少/单行/多行日志；原生 CI 同时使用 5.1 与 pwsh。报告不保存 provider 对象或凭据。

### 交付记录

[修复提交 254918b](https://github.com/LumiaBlack51/twodrive/commit/254918b5c53e427335c5ae1f743ab54787e7ad4d)；[通过原生 CI 38137498647](https://github.com/LumiaBlack51/twodrive/actions/runs/38137498647)。未推进 dev、未发布。

## TD-20261011-06：Windows 实机切换新版 serve 后已配对客户端无法访问共享

- 日期：2026-10-11
- 状态：调查中 / 根因未确认。
- 影响版本与环境：验证分支 `bdab13e` 的原生 release；Windows 校园网 TUN、Linux 热点 FlClash TUN。
- 关联历史故障：TD-20261011-03 的 Windows 实机验证；TD-20261011-04 涉及地址发现负响应，但尚不能认为根因相同。

### 症状与影响

通过旧 Windows serve 完成 15 MiB 新程序和脚本 PUT/GET 哈希校验后，用户确认启动临时 Windows 验证脚本。Linux 网关先无邀请连通旧 serve，切换期间随后 GET windows-report.json 持续返回 502，尚未取得新版 serve 的报告。没有推进 dev。

### 触发条件与复现

同一持久身份和授权，从旧 Windows serve 切换到自动物理出口的新 serve，未更新客户端已有地址提示。当前只观察到该次真实切换；用户日志确认新版已出现 Serving device，物理接口已选定。随后脚本发生内存异常，另记 TD-20261011-07。

### 根因与证据

已发现 DNS 响应选择缺陷，待回归与实机确认其修复效果。2026-10-11 19:44 从热点物理源地址查询同一公共设备 TXT：1.1.1.1 在约 25 ms 返回 NXDOMAIN（SOA MINIMUM=0），8.8.8.8 在约 315 ms 返回新加坡 relay 的 NOERROR/TXT、TTL 30。已有提示为美西 relay。Backend::query 接受第一个有效结果，快速空响应会取消尚未完成的有效记录查询，无法发现切换后的新 relay。TD-20261011-04 正确区分负响应与出口故障，但遗漏并发正/负响应冲突；不能以重新触发系统兜底代替修正有效 DNS 结果选择。脚本报告故障另记 TD-20261011-07，两者均需处理。

### 解决办法与恢复操作

DNS 并发选择改为优先非空的有效答案；空响应等待其余有界候选，均无记录时仍返回正常负响应，TTL 取最小值；全部请求失败仍报出口失败。新增确定顺序的 fast_negative_does_not_hide_later_positive_dns_record 与 negative_dns_remains_valid_when_no_server_has_records，原生两平台已通过；同一回归临时还原旧 first-valid 选择循环时，两平台均因漏掉有效记录稳定失败，退出后自动恢复源码。临时 Windows 脚本退出后 fresh 客户端已重新连通；脚本修复另记 TD-20261011-07。未改全局路由或代理。

### 验证结果与边界

修复后的原生 CI 38137498647 已通过（Linux 15 单元/10 集成、Windows 10 单元/9 集成及 release/LAN smoke），旧 DNS 选择实际失败与新选择通过对照成立。旧 serve 的程序传送通过；新版 Windows 双端 TUN 实机验证尚未通过，不将原生 CI 当作这次网络验证。

### 防复发措施与后续

确定机制后补充相关回归与实机复测；保留失败阶段和恢复结果。

### 交付记录

[构建提交 bdab13e](https://github.com/LumiaBlack51/twodrive/commit/bdab13ec0d3a2804b93c55498c73385075ed5df7)；[原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38135147276)。待补充实机记录。

## TD-20261011-05：自动物理出口的 Windows 原生 CI 未通过

- 日期：2026-10-11
- 状态：已验证修正后的 Windows 原生测试、release 和双进程 smoke；用户 Windows TUN 实机待执行。
- 影响版本与环境：验证分支 `1ae7b8f`；GitHub Windows 原生 CI 38134766370。
- 关联历史故障：TD-20261011-03 的 Windows 验证；已阅读 TD-20260916-03 的原生 CLI 历史和 TD-20261011-04，不凭相同 CI 失败认定同一根因。

### 症状与影响

Windows matrix job 的两项新网络测试失败；编译和严格 Clippy 成功。未交付新版程序、未重启用户 Windows serve，也未推进 dev。

### 触发条件与复现

对验证分支 workflow_dispatch 原生构建，cargo test 的 8 项单元测试中 6 项通过、2 项失败：Winsock 接口读取和连接器受控回退。

### 根因与证据

原生读取实得索引 14，测试错误期望 network-order 的 234881024。[Microsoft 文档](https://learn.microsoft.com/en-us/windows/win32/winsock/ipproto-ip-socket-options)说明 IP_UNICAST_IF 的设置参数为 network-order，而读取返回 host-order；有效设置的生产代码无需改字节序。另一测试把 u32::MAX 当作不存在接口索引，Windows 的 IPv4 接口索引实际只编码 24 位；原生 loopback 连接成功，不能将这个越界设置当作稳定故障注入。该证据不证明校园网或用户 Windows 的实际出口缺陷。

### 解决办法与恢复操作

读取测试改为检查 host-order 的真实索引；Windows 约束函数明确拒绝 0（未指定）和 IPv4 大于 24 位的非法索引，使同一回退测试在进入 OS 之前得到确定失败，并覆盖两种非法索引。正常 IPv4 设置继续采用 network-order。用户设备和稳定服务未进行恢复操作。

### 验证结果与边界

修正前 Windows 原生测试已实际失败；[修正后的原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38135147276) 完整通过，Windows 8 项单元、9 项集成、严格 Clippy、PowerShell 脚本解析、static-CRT release 构建以及独立两进程 2,097,105 字节 GET/PUT/MOVE/DELETE smoke 通过。Linux 的 13 项单元、10 项集成、严格 Clippy、release/LAN smoke 同轮通过。原生 CI 没有用户的校园网和 TUN，不能替代用户实机结果。

### 防复发措施与后续

定位实际失败机制后补充回归，原生 CI 与真实两端验证通过后才能推进 dev。

### 交付记录

[验证提交 1ae7b8f](https://github.com/LumiaBlack51/twodrive/commit/1ae7b8f064511d67c1e88be2ee82ef9b0bfa783c)；[失败 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38134766370)。尚未提交到 dev，未发布。

[修正提交 bdab13e](https://github.com/LumiaBlack51/twodrive/commit/bdab13ec0d3a2804b93c55498c73385075ed5df7)；[通过 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/38135147276)。验证分支尚未推进 dev。

## TD-20261011-04：物理 DNS 将 NXDOMAIN 错当出口失败

- 日期：2026-10-11
- 状态：已验证回归与真实 Linux/Windows 默认模式复测。
- 影响版本与环境：TD-20261011-03 尚未提交的自动物理出口实现，Linux 热点与 FlClash TUN。
- 关联历史故障：TD-20261011-03；这是新 DNS 实现中的响应分类缺陷。

### 症状与影响

默认 auto 的真实双机 2 MiB PUT/GET/MOVE/Range/DELETE 校验全部通过，观察到公网 TCP 使用物理网卡，但设备发现多次报告 DNS TXT 的 system/proxy fallback。该轮不能报告为所有组件均绕过代理。

### 触发条件与复现

公共递归 DNS 对设备发现 TXT 返回 NXDOMAIN。物理源地址上的 dig 查询两个独立公共 nameserver 均得到正常 NXDOMAIN 响应，耗时约 20/329 ms；未泄露私钥或邀请。

### 根因与证据

network/dns.rs 的 validate_response 仅接受 RCODE::NoError，将匹配事务、问题的 NameError 当作无响应，Resolver 因此启动系统 DNS 兜底。有效的无记录结果不代表物理出口失败。对端 TXT 尚无记录的具体原因未确认；已有邀请地址提示仍可建立 relay 连接。

### 解决办法与恢复操作

匹配事务/问题的 NXDOMAIN 与 NODATA 返回空记录，按 [RFC 2308](https://www.rfc-editor.org/rfc/rfc2308) 取 SOA TTL/MINIMUM 较小值作负缓存，缺少 SOA 或 MINIMUM=0 不缓存。空地址结果只在某个地址查询确实失败时允许 DNS 兜底；超时、SERVFAIL、非法响应仍可失败。未调整全局代理或路由。

### 验证结果与边界

同一新增 valid_nxdomain_is_not_a_physical_route_failure 在修复前实际失败（DNS response unusable），修复后通过，同时验证错误事务/问题、SERVFAIL 仍失败，以及负缓存缺少 SOA/MINIMUM=0 时 TTL=0。实验默认与 features mount 的 13 项单元、9 项集成测试均通过，两种配置的严格 Clippy 通过。旧实现的真实 auto 结果保存在[出口验证记录](peer-direct-egress-2026-10-11.json)，保留该轮 TXT 兜底证据。修正后的真实默认 auto 再次完成 2 MiB PUT/GET 哈希、MOVE、256 KiB Range、DELETE，61 次网络/socket 采样未见变化，所见公网 TCP 均约束 wlo1，整轮没有 system/proxy 兜底提示；共享目录为空，两个本轮文件名均 GET 404。该验证不建立对端 TXT 发布成功或 UDP 直连的结论。

### 防复发措施与后续

验证有效 NXDOMAIN 返回空记录，事务/问题错误及 SERVFAIL 继续失败；必须保留失败/修复后的对照与真实复测。

### 交付记录

尚未提交、未发布。

## TD-20261011-03：dev 在 TUN 代理下未统一选择物理出口

- 日期：2026-10-11
- 状态：已验证 Linux 物理出口、真实双机数据与自动兜底回归；Windows 原生出口未验证。
- 影响版本与环境：dev `355287f` 的 WebDAV/QUIC 实验；Linux FlClash TUN 与真实 Windows 对端。
- 关联历史故障：TD-20261011-02 的路由补查确认代理与美西 relay 路径；本次是用户要求修正出口选择，不是其测试脚本超时复发。

### 症状与影响

TwoDrive 可传输并校验数据，但本机策略路由将其 relay 流量送入 TUN，当前代理规则进一步经 VPS 转发；实际所选 relay 在 NA-west，RTT 中位数约 502 ms。用户要求在虚拟网卡存在时自动绕过代理，避免将该路径误当原生 NAT 打洞。

### 触发条件与复现

物理 Wi-Fi 已连接手机热点，同时 FlClash 的自动路由和 fake-IP DNS 开启。现有 dev endpoint 使用默认系统 DNS 与 socket 出口；此前只读路由检查及 15 次真实对端 OPTIONS 已确认所选 relay。

### 根因与证据

已确认应用缺少覆盖 DNS、UDP 和 relay TCP 的统一直连出口策略。实验的 WebDAV 客户端已使用 no_proxy，但该设置只关闭显式 HTTP 代理，不改变系统策略路由。锁定 iroh 1.3.0 支持 UDP 地址绑定，relay TCP 内部仍调用未绑定的 TcpStream::connect。采用进程内带随机认证的回环 CONNECT 连接器，将 relay TLS 透明转发到物理接口绑定的 TCP socket；定制 Resolver 通过物理 DNS socket 查询 A/AAAA/TXT，UDP 绑定物理源地址，地址发布使用相同连接器及原设备签名。物理接口判断依据 OS 与 netdev 元数据，排除 TUN、回环、down 和 fake-IP 地址。不能将源地址绑定等同于绕过所有系统透明拦截。

### 解决办法与恢复操作

已实现仅作用于 dev serve/connect 的 --network-route auto/direct/system，默认 auto；用户明确选择允许代理兜底并提示。DNS、TCP、签名发布和 endpoint 初始化/注册失败可报告 system/proxy fallback；首次 peer 传输失败先关闭 endpoint、以同一持久身份重建系统出口，在发送认证前重试。邀请失败或授权拒绝不触发这项重试。direct 关闭自动兜底，Linux 若接口约束被拒绝则失败；auto 可退为源地址约束并明确提示限制。TLS 证书验证、QUIC 设备认证、单次邀请协议保持。

只在本机启动了隔离 dev 网关，保留既有配对身份；未调整全局代理、路由或稳定服务，也未替换 Windows exe。所有测试文件仅为本轮拥有的合成文件，操作结束删去，Windows 对外可见共享列表为空。本机 dev 进程退出，配对状态仍仅 594 字节；清理独立 experiments/twodrive-dev/target 的全部可再生构建缓存（包括既有缓存），cargo clean 实际删除 15997 项、11.7 GiB。稳定 daemon 仍 active、PID 2404、NRestarts=0；保留源码与脱敏报告。

### 验证结果与边界

新增 12 项网络回归覆盖：物理网卡筛选、fake-IP 拒绝、TCP/DNS 源地址、IPv6-only 的回环访问、DNS 事务/问题/CNAME/负响应及 TCP 截断重试、连接器认证/关闭、auto 与 direct 回退差异、地址发布签名/防篡改和 TLS 必需。受控同一 TCP listener 的旧未绑定连接产生 127.0.0.1，新连接产生指定的 127.0.0.2，证明旧源地址行为违反新出口约束。12 项网络与原 model 共 13 项单元测试、9 项集成（包括一次性邀请、真实本机 QUIC、撤销既有连接）通过，fmt、默认与 features mount 严格 Clippy 均通过；两种配置的同一 22 项测试均通过，不含真实 OneDrive。

真实 Linux→Windows 验证：保留 FlClash TUN 与热点连接，direct 禁止兜底时完成 2 MiB PUT/GET 全量 SHA-256、MOVE、256 KiB Range、DELETE；22 次网络/socket 采样未见路由变化，所见公网 TCP 均带物理 wlo1 接口约束。auto 首轮传输也通过，但 TXT 负响应被误判为出口失败，已另记 TD-20261011-04，不将此轮称为全组件绕过代理。修正后最终默认 auto 再次完成相同校验和清理，61 次采样未见网络变化，所见公网 TCP 均约束 wlo1，无系统/代理兜底提示。初始所选 US-west relay RTT 为 760.9 ms，2 MiB 上传 48.606 秒、读回 9.150 秒，不能据此宣称绕过代理改善速度；[脱敏实测记录](peer-direct-egress-2026-10-11.json)保留全部轮次。

边界：两轮配对时所选路径均为加密 relay，未持续观察 QUIC 路径或建立 UDP peer 直连证据；Windows 使用旧 exe，未验证新增 Windows 原生构建/IP_UNICAST_IF/TUN 行为。iroh UDP 只绑定物理源地址，物理接口按启动时快照选择；网络变化后的重新选择需重启 dev。SO_BINDTODEVICE/IP_UNICAST_IF 无法保证绕过强制 VPN、WFP、透明网关；未保证所有虚拟适配器识别或任意 SOCKS 兜底。策略不覆盖外部 WebDAV 挂载和稳定 OneDrive。周期采样也不能排除采样间的变化。

### 防复发措施与后续

保留物理出口与受控回退回归，保证所有相关 HTTP/TLS 组件接入相同出口，凭据仅留内存、Debug 脱敏、连接器随 endpoint 退出关闭。保留旧代理路径证据与每轮实际结果；后续需 Windows 原生编译和 Windows TUN 实测，不能仅凭 Linux 成功扩大平台承诺。

### 交付记录

Windows 原生验证准备（2026-10-11）：增加 Winsock 接口约束读取回归、Windows/Linux 共用的自动回退回归，以及指定物理网卡的诊断参数。实机脚本沿用原身份与授权，受控重启测试 serve，并在结束或超时后恢复旧程序、清理自身临时负载；脚本不接受任意远程命令。原生 CI 和 Windows 实机验证结果待补充。先在临时验证分支构建，验证完成前不推进 dev。

尚未提交、未发布；[脱敏实测记录](peer-direct-egress-2026-10-11.json)。

## TD-20261011-02：热点复测的 64 MiB 上传触发临时脚本超时

- 日期：2026-10-11
- 状态：已验证临时脚本的发送期限修正；未确认生产客户端传输缺陷。
- 影响版本与环境：dev `355287f` 的 CI release 客户端与临时 Python HTTP 测试脚本；真实 Windows/Linux 双机。
- 关联历史故障：TD-20261007-02 同为验证期限不足，但该次涉及 FUSE 60 秒恢复周期，本次是 Python 大请求体发送预算，根因不同；TD-20261011-01 是另一项临时诊断工具缺陷。

### 症状与影响

用户重新连接热点后，15 次认证 OPTIONS 成功且观察路径为 relay，无邀请重连成功；随后 64 MiB 临时文件上传报 URLError，未完成读回校验。网络监测采样 189 次，未发现活动 Wi-Fi 连接或路由变化。测试脚本 finally 删除本次临时文件，复查共享目录为空，未触碰服务器私有身份状态。

### 触发条件与复现

临时 Python 脚本以 `urllib.request.urlopen(..., timeout=40)` 发送 64 MiB 合成文件，第一次复测在发送阶段报 URLError。之前 16 MiB 上传在约 32 秒完成，该预算对大文件不足。

### 根因与证据

已确认是临时脚本的 40 秒请求体发送预算不足。Python 标准库 `HTTPConnection.send` 对 bytes 请求体调用一次 socket.sendall；`AbstractHTTPHandler.do_open` 仅将 request 阶段的 OSError 包装为 URLError，随后 getresponse 在该内层捕获之外。实测错误为 `URLError: <urlopen error timed out>`。将测试预算提高至 240 秒并保持 64 MiB 请求体与生产代码不变后，记录到单次请求体发送实际耗时 64.587 秒，完整上传耗时 68.943 秒，读回哈希一致。这个实际发送耗时超过原预算，解释了第一次失败；两轮网络监测均未发现本机连接/路由变化。

### 解决办法与恢复操作

仅提高临时验证脚本的 HTTP socket 期限至 240 秒，并记录大请求体发送耗时；没有改变校验条件或生产程序。失败与成功两轮均在 finally 删除自己创建的文件并停止本机测试网关。服务器原子上传对象的 Drop 清理未提交临时路径；WebDAV 目录列表隐藏其保留前缀，故列表为空的直接证据范围是对外可见共享项。

### 验证结果与边界

旧 40 秒预算的真实双机复测失败；240 秒预算的真实双机复测成功完成 64 MiB PUT/GET 全量 SHA-256 比对、MOVE、1 MiB Range 读回及 DELETE，两个临时文件名均 GET 404，共享目录 PROPFIND 返回 0 个可见子项。复测约 226.847 秒，420 次采样未发现活动 Wi-Fi 连接标识、链路状态、全部路由表或策略路由规则变化；15 次独立认证 OPTIONS 的路径观察仍为 relay。详细实际结果见[脱敏验证记录](peer-cross-network-2026-10-11.json)。

这是临时工具的旧预算失败、新预算通过对照，未改动生产程序；不表示 UDP 直连通过，也不证明所有网络吞吐下 240 秒足够。此前数据校验保留，历史固定网络条件仍不确定。按用户要求追加只读路由检查：本机 FlClash TUN 策略路由接管 IPv4，当前代理配置与已保存选项将 relay 送往 Hysteria2 VPS；独立 15 次请求实际所选 relay 位于 NA-west，RTT 中位数约 502 ms。因此本轮不属于绕过代理的原生 NAT 穿透验证；没有修改网络配置。

### 防复发措施与后续

跨网络大文件验证要根据文件大小与已测吞吐设置有界发送预算，并区分请求体发送、响应和后台提交的期限；记录实际发送耗时及 finally 清理结果。保留本次失败/成功对照，测试完成删除临时工具和数据。没有已确认的生产缺陷，未新增生产回归用例。

### 交付记录

本次尚未提交、未发布；[脱敏验证记录](peer-cross-network-2026-10-11.json)包含两次真实尝试及清理范围。

## TD-20261011-01：跨网络验证的临时路径探针收到空 HTTP 响应

- 日期：2026-10-11
- 状态：已验证临时诊断程序的请求生命周期修正；未确认 TwoDrive 生产代码缺陷。
- 影响版本与环境：dev `355287f` 的协议库与独立临时 Rust 诊断辅助程序；真实 Linux/Windows 双机。用户随后报告热点曾断开并自动回落校园网，因此此前整段测试的固定网络条件无法确认。
- 关联历史故障：TD-20261007-02 同为验证工具边界问题，但本次是手工 QUIC/HTTP 探针，不认定同源。

### 症状与影响

现有 release 客户端已通过 64 MiB 读取、真实 FUSE 字节比对，以及 16 MiB 临时文件上传、读回、移动和删除。随后独立路径探针的设备身份认证成功、初始路径为 relay，但第一条 OPTIONS 的 HTTP 响应为空，探针退出，未完成持续路径观察。

### 触发条件与复现

共享测试数据清理后，使用保留的本机设备身份新建 QUIC 连接，认证后手工写入完整 OPTIONS 请求、调用发送流 finish，再读取响应。旧辅助程序在身份认证成功后得到空响应；移除提前 finish 后，同一设备、相同 OPTIONS 请求连续 15 次成功。

### 根因与证据

已定位为辅助程序在等待 HTTP 响应前过早关闭 QUIC 发送流。认证帧返回 accepted 后，旧程序调用 finish，产生 EOF；实验的 `peer::serve` 使用 Hyper HTTP/1 默认的 `half_close(false)`。锁定的 Hyper 1.11.1 在 `mid_message_detect_eof` 中发现响应处理中 EOF 时关闭读端并返回 incomplete，符合空响应现象。仅移除辅助程序的提前 finish 后，15 次 OPTIONS 全部成功，观察持续约 48 秒且所选路径均为 relay。没有证据把该诊断工具问题归因于 NAT 或设备授权失效。

### 解决办法与恢复操作

辅助程序保持请求发送端打开直到收到响应，沿用服务端 HTTP 策略；未修改 TwoDrive 生产代码。

本地恢复操作另述：原本机测试 FUSE 挂载已正常卸载，原 release 客户端已退出。Windows 经用户确认的 7 项合成测试数据已删除，共享目录当时复查为空；私有设备身份与授权状态按用户要求保留。用户重新连接热点后进行受网络监测的独立复测，后续临时数据及工具的最终清理结果见脱敏记录。

### 验证结果与边界

真实 Windows/Linux release 客户端此前完成 64 MiB 远程下载与真实 FUSE 哈希比对、16 MiB 上传/读回/移动/删除；这些结果保留为双机数据正确性证据。因用户报告网络回落，不把此前整段测试称为稳定的热点与校园网跨 NAT 验证。旧临时探针失败，修正后实际 15 次认证 OPTIONS 通过；属于真实双机诊断工具对照，没有新增生产代码回归测试。

重新连接后的持续路径观察、64 MiB 读写复测、网络稳定监测与最终清理详见[脱敏验证记录](peer-cross-network-2026-10-11.json)。relay 可达不等于 UDP 穿透成功；没有真实 OneDrive 测试，没有修改原有用户服务或共享目录以外的 Windows 状态。

### 防复发措施与后续

手工 HTTP/QUIC 探针应遵循服务端的半关闭策略，先验证请求/响应生命周期再统计路径变化；本条记录保留旧程序失败与修正后通过的实际对照。固定网络测试要在操作期间监测活动 Wi-Fi 连接标识及路由，事后单次查看不能排除中途回落。测试结束清理辅助程序、下载包及挂载缓存，仅保留必要的设备配对状态与脱敏记录。

### 交付记录

本次尚未提交、未发布。临时诊断程序不安装到稳定 TwoDrive，也不进入 Windows 开发包；[脱敏验证记录](peer-cross-network-2026-10-11.json)保存实际范围与结果。

## TD-20261007-03：整合树 Windows 原生 peer 测试失败

- 日期：2026-10-07
- 状态：已验证 Windows 原生测试编译与执行修复；Linux/Windows peer 全流程通过。
- 影响版本与环境：dev 805ceba；GitHub windows-latest，Peer native builds 37593664900。
- 关联历史故障：与 TD-20260916-WIN-AUTH-TEST 同为测试缺少平台限定；本次是较新 main 新增的 known-folder 夹具，不是旧 OAuth 修复回归。TD-20261006-04 是生产依赖隔离，与本次编译目标不同。

### 症状与影响

Windows 的 cargo test -p twodrive-peer -p twodrive-core -p twodrive-backend --locked 在测试编译阶段失败；Linux 全工作区测试通过。该 job 未交付 Windows peer 资产；并行 Windows Full/Lite 构建仍在执行。未改动用户安装、配置或真实云端。

### 触发条件与复现

合并 Windows PR #9 和 peer-control 后首次原生 CI；运行包含 core 单元测试的 Windows 命令即可触发。Windows 预览历史流程只测试 Windows 引擎/后端，没有编译 core 的 cfg(test) 目标；peer-control 历史基线尚无该夹具。

### 根因与证据

known_folders.rs:229 的 tests 模块无条件导入 std::os::unix::fs::symlink；Windows 报 E0433 cannot find unix in os。该夹具由 main 的 6f3c40a 加入，检查 Unix/XDG 根目录和符号链接。生产 core 可编译；不是 P2P 握手或 QUIC 加密机制故障。

### 解决办法与恢复操作

仅为该 Unix 夹具及其 symlink 导入添加 cfg(unix)。保留不依赖 Unix 系统调用的 XDG 解析测试和所有其他 core/后端/peer Windows 测试，不绕过整套原生验证。生产路径未改动；没有执行用户环境恢复操作。

### 验证结果与边界

旧树 805ceba 在原生 CI 明确编译失败。修正后本地 backend/core/peer 94 项默认测试通过，原 Unix 夹具仍在 Linux 执行；fmt 和严格 Clippy 通过。c73fece 原生 Windows 93 项默认测试通过（仅该 Unix 夹具未编译），额外更新进程测试通过；Linux 151 项默认工作区测试和额外更新进程测试通过。两平台 peer release、自检和资产上传均通过；Windows DPAPI 往返、私有身份持久化及系统 DLL 检查通过。均使用合成状态，未验证真实 OneDrive 或异地设备握手。

### 防复发措施与后续

保留 Windows core 单元测试编译与执行在 peer CI 中；这是此次可复现构建缺陷的回归门槛。新增平台夹具必须使用对应 cfg，不能只验证生产 library 或上层引擎测试。

### 交付记录

[整合提交 805ceba](https://github.com/LumiaBlack51/twodrive/commit/805ceba6616efd6d77014736881ad1e9d74a6528)，[失败 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37593664900/job/112700935062)；[修复 c73fece](https://github.com/LumiaBlack51/twodrive/commit/c73fece6b8e9215b562ea88317a31f9f5723cf7e)，[成功原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37594656006)。peer 资产 [Windows](https://github.com/LumiaBlack51/twodrive/actions/runs/37594656006/artifacts/11469772936) / [Linux](https://github.com/LumiaBlack51/twodrive/actions/runs/37594656006/artifacts/11470636670)。未发布 Release 或替换用户安装。


## TD-20261007-02：dev 合并后的隔离 FUSE 上传冒烟超时

- 日期：2026-10-07
- 状态：已定位并验证冒烟等待边界；保留既有 60 秒重试语义。
- 影响版本与环境：未提交的 Windows PR #9 + peer-control + dev 整合树，Linux 临时目录与真实 FUSE。
- 关联历史故障：TD-20261006-02/03 是 WebDAV 上传/ETag 适配；本次仅确认挂载上传超时，不据症状判定同源。

### 症状与影响

双 CLI LAN 直连的 GET/PUT/MOVE/DELETE 阶段已完成，挂载读取也已完成；创建 new-folder/written.txt 并 fsync 后，等待服务器字节一致超过 45 秒。测试数据是临时合成内容；未操作用户安装、账户、云端或现有挂载。

### 触发条件与复现

cargo build 实验 --features mount 后，scripts/smoke.py --network lan --mount 首次运行超时（exit 1）。原脚本退出清理临时目录，正常终止进程时未输出失败日志，尚无后台错误证据。

### 根因与证据

不是已确认的合并回归。恢复留档后再次以 45 秒截止运行仍失败：父目录已在服务器创建且本地 metadata 操作已清空；子文件保留在本地 dirty 缓存，没有后台传输错误。`recover_dirty_record` 在父目录操作未完成时返回 false，worker 本轮结束；`mount_backend` 的周期恢复每 60 秒重新排队。两文件与原 dev 71c31e0 完全相同。将该测试等待范围覆盖完整重试周期后，真实挂载在 59.8 秒确认服务器内容，符合该机制。

### 解决办法与恢复操作

修正测试截止时间：挂载保存及依赖操作允许 90 秒覆盖已有 60 秒恢复周期；增加失败日志和显式 --keep-state 保留一次性证据。没有改变引擎调度，也没有把后台尚未完成的本地保存显示为成功上传。仅正常退出并卸载该测试自己的挂载，未改动真实服务。

### 验证结果与边界

本轮 151 项主工作区默认 Rust 测试、10 项 dev 测试、严格 Clippy/格式、19 Nautilus 和 4 Settings 测试通过；额外 peer 更新子进程和引擎 IPC 通过。原 45 秒冒烟两次失败；修正测试边界后真实 FUSE + QUIC LAN 冒烟通过读取、完整字节保存/后台上传、目录移动与删除（保存耗时实测 59.8 秒）。独立只读冒烟通过，内核写入返回 EROFS，服务器 PUT 被拒绝。不能声称立即上传，不能将本机实验当作异地 NAT 或真实 OneDrive 端到端。

### 防复发措施与后续

保留真实挂载字节比对、失败日志/状态选项，以及包含实际恢复周期的截止时间。该验证修正不改善现有子文件等待延迟；若后续优化父目录完成后的子项唤醒，须另做调度机制回归。

### 交付记录

[整合提交 805ceba](https://github.com/LumiaBlack51/twodrive/commit/805ceba6616efd6d77014736881ad1e9d74a6528)，最终运行代码 [c73fece](https://github.com/LumiaBlack51/twodrive/commit/c73fece6b8e9215b562ea88317a31f9f5723cf7e)。[WebDAV/QUIC 原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37594655979) Linux/Windows 全部成功：Linux 10、Windows 8 项测试及 2,097,105 字节双进程 LAN 比对通过；CI 不运行 FUSE，实际挂载结果为本条所述本地验证。未发布 Release 或替换用户安装。


## TD-20261007-01：dev 多分支合并工具截断工作区 manifest

- 日期：2026-10-07
- 状态：已验证 Linux/Windows 合并 manifest、编译、协议与组合包回归。
- 影响版本与环境：未提交的 dev + Windows PR #9 + peer-control 合并树；仅隔离 worktree。
- 关联历史故障：TD-20261006-04 是 Unix 平台依赖问题；本次是合并工具机制，不属同源。

### 症状与影响

合并冲突消解后的 cargo metadata 报 Cargo.toml 第 8 行 unclosed array。尚未编译或交付该中间树；原 main 工作目录、安装、服务和用户数据未改动。

### 触发条件与复现

合并 codex/peer-control 时，临时 Python helper 对多行 Git 冲突块使用 DOTALL 正则，起止标记中的 .* 也可跨行匹配。工作区 members 尾部及后续字段被误吞；manifest 检查立即失败（退出 101）。

### 根因与证据

此问题属于本轮合并操作错误。起止标记应限制到单行，不能贪婪匹配后续完整内容。不是源 Windows 或 peer 实现本身的 Cargo 语法错误。

### 解决办法与恢复操作

从已核实的 Windows 合并父提交恢复完整 Cargo.toml、核心 manifest 和较新的 OAuth 实现；明确加入 peer member。Cargo.lock 按父提交的完整 package 段合并，并保留既有锁定版本，再由 Cargo 重新核对实际依赖。Gitignore 同时保留 Windows 产物与 peer 秘钥排除规则。恢复操作仅作用于本轮合并文件，无用户配置/文件恢复。

### 验证结果与边界

恢复后 cargo metadata --no-deps --offline 成功解析；离线更新合并依赖图后 --locked 编译通过。主工作区 151 项默认 Rust 测试通过（6 项 FUSE 和 1 项更新进程默认忽略），另显式执行更新进程测试通过；dev 的 1 项兼容性 + 9 项协议测试、两工作区 fmt/严格 Clippy、19 Nautilus、4 Settings 通过。Linux 独立引擎进程的 IPC/单实例/重启/字节比对通过。

最终 c73fece 原生 Linux/Windows 的三条整合 CI 全部成功：Windows core/backend/peer 93 项、额外更新进程、Windows 引擎 11 项、Flutter 12 项、analyze/release、命名管道 IPC、Full/Lite 中同一工具哈希及实际打包 QUIC 双进程传输通过。Windows dev 8 项协议测试通过。下载最终组合资产后，独立验证 Full 25 / Lite 9 文件的 manifest 长度/哈希、两 ZIP 校验值与内容、四个工具跨版一致。真实云端、异地 NAT、用户设备和 CFAPI 未验证，6 项既有 FUSE ignored 未在本轮执行。

### 防复发措施与后续

保留源父提交恢复边界，manifest 使用标准 TOML 解析校验；冲突标记按单行限制，不通过文本截取拼接 package 半段。提交前运行格式、严格 Clippy、两工作区测试和原生 CI，并保留所有已有故障条目。

### 交付记录

Windows 来源 5120cf7（[PR #9，已合并到 dev](https://github.com/LumiaBlack51/twodrive/pull/9)），peer 来源 dd0686b，原 dev 71c31e0；Windows 合并 d3c9e32，peer/包整合 [805ceba](https://github.com/LumiaBlack51/twodrive/commit/805ceba6616efd6d77014736881ad1e9d74a6528)，平台测试限定 [c73fece](https://github.com/LumiaBlack51/twodrive/commit/c73fece6b8e9215b562ea88317a31f9f5723cf7e)。[Windows 组合包 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37594661999)、[peer CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37594656006)、[WebDAV/QUIC CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37594655979) 均成功。已核验[组合资产](https://github.com/LumiaBlack51/twodrive/actions/runs/37594661999/artifacts/11469933545)来源为 c73fece；Full ZIP SHA-256 `49c2ab6540228984745233bbed51b9235f38a1148c880c12566318d0c68b6552`，Lite ZIP `0b4663b87ab01e72c23836350e11df4263c170139cb44d3ad541ded7c992835e`。仅记录更新在构建后提交，不改变可执行文件的源码/锁文件/构建脚本；资产内文档是该构建的快照。未合并 main、发布 Release 或替换用户安装。


## TD-20261007-PR-WSL：提交前 fetch 的宿主调用超时

- 状态：重试成功；一次 WSL 调用返回 Wsl/Service/WSAETIMEDOUT，根因未确认。
- 影响：首次远端基线检查未完成，不计通过；无应用/凭据/云端数据变化。
- 处理：重新通过 WSL 执行有界 git fetch，仅更新远端跟踪引用，成功取得 main bdacfb9。
- 验证：随后 Git 查询、workspace 测试 127 passed/6 ignored、Clippy 均成功。
- 边界/后续：不归因为 TwoDrive 缺陷；保留超时事实，网络操作继续使用有界等待。

提交检查补记：git diff --cached --check 发现此前未跟踪证据中的 CRLF、终端尾部空格及 EOF 空行；仅规范文本空白后重跑，保留日志内容与所有成功/失败结论。旧 git diff --check 未包含未跟踪文件，不能代表这些文件此前已通过暂存检查。

TD-20261007-PR-WSL 补记：创建 PR 后一次新 WSL 调用再次在启动阶段超时；后续核对工作树干净、HEAD 174a12b，确认该次没有文档写入或提交。重试读取 PR 状态成功；宿主间歇启动超时根因仍未定位，不影响已创建 PR。
## TD-20261006-04：dev Windows 原生编译检查失败

- 日期：2026-10-06
- 状态：已验证（原生 Linux/Windows 编译、协议测试与双进程 CLI）。
- 影响版本与环境：dev 提交 `0222663`，GitHub windows-latest，静态 CRT 编译参数。
- 关联历史故障：TD-20261006-02/03 属于 Linux 协议/文件适配问题；当前未认定同源。

### 症状与影响

Windows fmt 通过，Clippy/编译步骤失败，后续测试与产物构建被跳过；尚未交付 Windows 可用性。

### 触发条件与复现

[原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37469772524/job/112289915114) 首次编译。同次 Linux CI fmt/Clippy/测试、release 构建、双 CLI 进程传输与资产上传全部通过。

### 根因与证据

`twodrive-core/src/credentials.rs:2` 无条件导入 `std::os::unix::fs::PermissionsExt`，Windows 原生编译报 E0433；第 34 行 `Permissions::from_mode` 报 E0599。实验模块无条件依赖稳定核心/后端，导致本来仅用于 Linux 的凭据代码也参与 Windows 编译。错误发生在实验协议代码编译前，与 QUIC/NAT/加密不属同一机制。

### 解决办法与恢复操作

稳定核心、后端及 FUSE 依赖仅在 Linux 启用；实验内新增可移植元数据和最小存储接口，WebDAV 客户端与协议测试通过统一 `model` 引用。Linux 仍重导出原接口，Windows 不引入稳定 OAuth/数据库实现。未改动稳定源码、用户安装或真实目录，无额外本地恢复。

### 验证结果与边界

旧 Windows CI 明确失败，未运行协议测试或构建产物。修正后本机 Linux 新增元数据兼容性测试与 9 项协议集成测试全部通过，fmt/Clippy 警告视错误通过；`cargo tree --target x86_64-pc-windows-msvc --locked` 核对无稳定核心/后端/FUSE/SQLite 依赖。

[a15be05 原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37471312984) 已全部成功：Linux 1 项元数据兼容性测试 + 9 项协议测试，Windows 8 项协议测试；两平台 fmt、Clippy 警告视错误、release 构建、双 CLI 进程 GET/PUT/MOVE/DELETE 与 2,097,105 字节比对、产物上传通过。Windows 使用静态 CRT。原失败分支不能编译，修正后同一原生流程成功，验证了依赖隔离机制。

**边界**：Windows 未执行 Unix 权限、软链接与 FIFO 的专用测试，也未验证用户特定 ACL、Explorer WebDAV 客户端或 Windows FUSE 挂载。CI 冒烟各自在单个 runner 内启动两进程，不能据此声称两台异地 NAT 验证。Linux/公共中继与稳定基线范围见 TD-20261006-03。

### 防复发措施与后续

保留原生 Windows 编译、协议测试、release 构建与双 CLI 进程 GET/PUT/MOVE/DELETE 冒烟；保留 `portable_metadata_matches_the_linux_contract`，不通过修改稳定代码绕过隔离边界。保留平台矩阵，源码改动后实际核对 CI 结果。

### 交付记录

源码 [0222663](https://github.com/LumiaBlack51/twodrive/commit/022266391500e1293f722bccb9d333479c7c3cbf)，草稿 [PR #8](https://github.com/LumiaBlack51/twodrive/pull/8)；隔离修复 [a15be05](https://github.com/LumiaBlack51/twodrive/commit/a15be05fe8c038f4ac4556a72a2b0a649c5ec38f)；[成功 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/37471312984)，资产 [Linux](https://github.com/LumiaBlack51/twodrive/actions/runs/37471312984/artifacts/11416947772) / [Windows](https://github.com/LumiaBlack51/twodrive/actions/runs/37471312984/artifacts/11417581314)。资产对应已验证源码，不作为稳定 Release。

## TD-20261006-03：dev 条件覆盖与根目录文件 PUT 被错误拒绝

- 日期：2026-10-06
- 状态：已验证（本机 WebDAV、QUIC 和隔离 FUSE）；真实异地 NAT 与外部厂商服务未验证。
- 影响版本与环境：未提交 `dev` 实验 `twodrive-dev 0.1.0`，Linux 临时目录；稳定 TwoDrive 不受影响。
- 关联历史故障：TD-20261006-02 首轮问题修正后暴露；新的 ETag/根目录同步问题属于新增适配器的不同机制。

### 症状与影响

有效 ETag 覆盖返回 HTTP 412；位于共享根目录的文件 PUT 返回 HTTP 404。后者在原子替换后失败，可能已提交内容；不能将错误响应等同于远端未写入。测试未操作用户文件。

### 触发条件与复现

`webdav_contract_unicode_versions_moves_and_snapshot_deletions` 在条件覆盖时失败；`streaming_download_cancellation_and_full_file_upload` 创建 `/large.bin` 时失败。嵌套路径的字节数组 PUT 与本机 QUIC 读写已经通过，当时 8 项测试为 6 通过、2 失败。

### 根因与证据

- dav-server 0.11 的 PROPFIND `getetag` 直接输出元数据裸值，HTTP 条件检查使用带引号 ETag；客户端直接复用 XML 值造成不匹配。保留已带引号/弱标记形式，只给裸值补 HTTP 引号，并要求覆盖使用单个带引号的强 ETag，防止弱/星号/无效条件被服务忽略。
- `Path::parent()` 对根目录相对文件名返回空路径。提交后调用 capability `open_with("")` 返回 ENOENT，映射成 HTTP 404；不是 HTTP File body framing 或原文件缺失。改为对空父路径使用 `.`。

### 解决办法与恢复操作

修正 ETag 语法与父目录句柄选择。未修改稳定引擎、真实配置或安装服务；仅使用测试目录，无额外本地恢复。

### 验证结果与边界

- 两项测试在修正前分别失败（412/404），修正后通过。5 MiB 文件流上传/下载字节一致，下载取消有实际回调；旧 ETag/未带版本的覆盖被拒绝，正确版本可覆盖，后续 MOVE/DELETE 与快照删除检测通过。
- `invalid_conditional_tokens_fail_before_network_io` 覆盖弱标记、星号、裸值、多个 token 和换行；拒绝发生在网络操作前。
- 最终 9 项实验集成测试全部通过，fmt、Clippy 警告视错误通过。两独立 CLI 进程的 LAN、公网基础设施默认模式和强制公共中继模式均通过 GET/PUT/MOVE/DELETE，2,097,105 字节比对一致；强制中继输出已核实为 relay。
- 隔离真实 FUSE + QUIC 测试通过读取、持久保存/后台上传、目录移动和删除；另验证默认只读挂载返回 EROFS、服务器拒绝 PUT。
- 稳定工作区 101 项 Rust 默认测试、19 项 Nautilus 和 4 项 Settings 测试通过；原有 6 项环境相关 FUSE 测试仍默认忽略，本次新实验挂载冒烟与它们是不同验证。
- **边界**：两端进程均在本机，公共中继是实际外部基础设施，但未证明两台异地 NAT 机器打洞；未测试全部 WebDAV 厂商、真实 OneDrive 端到端、完整桌面 GUI、断点上传或双向目录镜像。Windows 原生编译、协议与产物结果另见 TD-20261006-04，未将 CI 配置当作验证成功。

### 防复发措施与后续

保留真实协议与磁盘内容测试、条件写语法检查、独立缓存绑定和完整快照失败时不生成删除结果的约束。路径 ID 随移动变化，WebDAV 刷新语义及外部服务的原子性边界见 [设计](dev-webdav-peer-design.md) 与 [实验指南](../experiments/twodrive-dev/README.zh-CN.md)。

### 交付记录

实现与修复已提交 [0222663](https://github.com/LumiaBlack51/twodrive/commit/022266391500e1293f722bccb9d333479c7c3cbf)，草稿 [PR #8](https://github.com/LumiaBlack51/twodrive/pull/8)。Windows 原生结果见 TD-20261006-04；不合并或发布到稳定 main。

## TD-20261006-02：dev WebDAV 适配器首轮端到端读写失败

- 日期：2026-10-06
- 状态：已验证（Linux 临时目录）；后续独立机制见 TD-20261006-03。
- 影响版本与环境：`dev` 分支新增 `twodrive-dev 0.1.0` 实验；未影响稳定程序。
- 关联历史故障：无相同实现；[provider 边界记录](refactor-2026-09-16.md) 涉及共享接口，失败在新增适配器。

### 症状与影响

首轮 PUT 返回 HTTP 500，上传上限较小时单资源 PROPFIND 返回 HTTP 502，4 项读写测试失败。已确认 PUT 错误可发生在原子替换之后；原有数据均为临时测试文件。

### 触发条件与复现

初始 8 项集成测试：4 通过、4 失败。普通、Unicode、流式 PUT 和真实本机 QUIC 转发均失败；设置 64 字节上传上限后，174 字节 PROPFIND XML 也被拒绝。

### 根因与证据

- cap-std 的 Linux 目录能力可能使用 O_PATH。复制能力句柄后直接 fsync 返回 EBADF（脱敏诊断为 errno 9），此时文件已同步、已 rename；HTTP 500 并不说明写入未发生。
- 把上传上限应用到全部方法的请求 body，误拒绝 PROPFIND XML；该错误被 DAV 库转为 HTTP 502。
- 本轮源码检查另发现普通文件检查原位于 open 之后，根目录中的 FIFO 可阻塞读取。直接命名管道回归覆盖该路径，增加 open 前类型检查及 Unix O_NONBLOCK，之后再检查句柄类型，防止本机替换竞态使网络读取阻塞。

### 解决办法与恢复操作

以目录能力重新打开可读父目录句柄并同步，保持访问范围约束；文件体上限只用于 PUT，XML 采用独立 64 KiB 上限。Content-Length 和实际流长度超限返回 413，中断/超限写入删除临时文件，保留原内容；命名管道与设备不作为普通文件打开。未操作真实账户、用户目录、系统服务或安装包。

### 验证结果与边界

- 修正前 PUT/小上限元数据测试失败（500/502），修正后通过。`interrupted_and_oversized_put_preserve_original_content` 核对原内容与临时文件清理；元数据读取不受 64 字节上传上限影响。
- `capability_scope_rejects_symlink_escapes_and_special_files` 覆盖外部目录/文件软链接、设备软链接、命名管道、`..` 和保留临时文件名。临时移除普通文件检查和非阻塞打开后，旧机制的单项回归在编译完成并进入测试后超过 12 秒，被隔离测试超时终止（退出 124）；恢复保护后该项通过。
- 权限、过期/单次/并发邀请、真实 QUIC 未授权拒绝和现有连接撤销测试通过。验证为临时磁盘与本机设备，未覆盖所有文件系统的断电恢复、磁盘故障或 Windows ACL 部署。

### 防复发措施与后续

保留文件提交后响应、上传错误不破坏原内容、独立 XML 限制与 capability 范围回归；无法声称所有 500/502 错误永久消失。其他新机制和完整测试范围见 TD-20261006-03。

### 交付记录

实现与修复已提交 [0222663](https://github.com/LumiaBlack51/twodrive/commit/022266391500e1293f722bccb9d333479c7c3cbf)，草稿 [PR #8](https://github.com/LumiaBlack51/twodrive/pull/8)；[dev 设计](dev-webdav-peer-design.md)。

## TD-20261006-01：系统语言切换后特殊目录与上传来源错位

- 日期：2026-10-06
- 状态：已验证来源诊断与重选机制；系统目录变化的完整根因未确认，真实云端端到端未验证。
- 影响版本与环境：本机 Linux/GNOME，本次运行 daemon SHA-256 与此前本地调查已核实的 0.2.10-2 发布产物相同（`2500d0922ede9ffa736ccaa4611c3d670859d4cced59a14bf2533b8de1312332`）；修复版本 0.2.11-1。
- 关联历史故障：无相同已记录故障；[重构记录](refactor-2026-09-16.md) 涉及 known-folder 模块。本次提示缺口在重构前实现中也存在，未认定为重构回归。

### 症状与影响

切换中文到英文后，`~/Pictures` 仍为指向失效旧 OneDrive 路径的软链接，XDG 图片入口指向主目录，Documents、Videos、Templates 英文入口与旧中文内容目录分离。本机只读检查确认上述状态。人工将“下载”改为 Downloads 后，TwoDrive 仍配置 `~/下载 -> /Downloads`，未覆盖新目录；`~/图片 -> /Pictures` 来源仍存在。未确认图片丢失。

### 触发条件与复现

已失效旧软链接、桌面语言切换和显式上传路径与本地新名称不同。隔离测试创建“下载”并改名为 Downloads 后，旧来源缺失；旧版 status 没有错误提示，设置界面“Recent errors”固定显示 None recorded locally。另用临时目录模拟挂载来源，旧扫描允许把该目录内容上传到 MockBackend。

### 根因与证据

- **已确认的 TwoDrive 缺陷**：`known_folders/scan.rs` 对缺失来源仅在扫描时写后台日志，未启动监视；CLI 只显示映射，GTK Settings 没有来源健康状态或重选入口，却固定显示“没有错误”。因此用户难以发现失效来源或修复映射。扫描原先也未拒绝主目录/根目录/TwoDrive 挂载来源，本次补充一致校验以避免错误重选范围。
- **独立的配置变化**：人工改名造成 Downloads 映射失效，不能归因于 TwoDrive。TwoDrive 的 local 是显式路径，不跟随 XDG 或系统语言自动更名。
- **旧路径迁移检查**：当前安装/卸载脚本及仓库已检索历史没有创建该旧 OneDrive 软链接或改写 `user-dirs.dirs` 的实现；没有其他客户端特殊目录迁移功能。旧链接创建者、最初配置和迁移过程仍未确认，不能证明所有旧路径迁移已完成。
- [xdg-user-dirs-update 文档](https://manpages.debian.org/testing/xdg-user-dirs/xdg-user-dirs-update.1.en.html) 确认不存在的特殊目录可被重置到主目录，GUI 工具可处理语言更名；缺少当时执行日志，故仅作为与现场相符的机制，不能确认完整因果链。

### 解决办法与恢复操作

- 共享来源诊断，区分缺失目录、失效/被跳过的软链接、非目录、不可读取和不适合作为上传来源的路径。CLI status、JSON `known-folders status`、daemon 和 Settings 使用相同检查；识别 XDG 主目录回退与映射差异并解释自定义映射也可能合理。
- Settings 增加 Choose source…；`known-folders set-source <index> <directory>` 验证选中目录后原子保存单项 local，保留其他配置、未知选项和远端目标。界面保存校验原映射，避免使用过时行号覆盖已变化来源；失败保留配置。
- 拒绝主目录、文件系统根目录、挂载内部和包含挂载的来源，不把云端目录改作上传源。配置保存后需关闭挂载文件并重启 TwoDrive；运行中的 daemon 不热加载配置，启动时缺失来源恢复后也需重启。

**本地恢复操作**：本次未改动用户 XDG 设置、特殊目录、软链接、上传映射，未安装新包或重启真实服务。仅创建并卸载隔离测试挂载。Pictures 的系统入口和 Downloads 的实际映射仍需按用户期望选择；新代码不会自动迁移私人内容。

### 验证结果与边界

- `status_exposes_missing_upload_source_after_local_rename` 在未修改 CLI/诊断的旧代码上失败（status 无缺失提示），修复后通过。
- `mount_source_is_not_uploaded_into_itself` 在恢复旧扫描 guard 的代码上失败（MockBackend 收到模拟挂载目录内容），修复后通过；不是实际云端循环上传复现。
- 101 项 Rust 默认测试通过，6 项环境相关 FUSE 测试仍默认忽略；19 项 Nautilus、4 项 Settings 回归通过；cargo fmt、Clippy 警告视错误、git diff --check 通过。
- 回归覆盖 XDG 主目录回退与差异、中文显式路径不被猜测改名、失效/有效软链接、文件、相对路径、主目录与挂载别名、只读诊断不改配置、错误选择/过时来源保持配置、成功选择保留远端目标/未知配置/0600 权限、Settings 按钮映射及子进程错误显示。Settings 测试使用控件替身，不代表完整 GUI 点击验证。
- 使用本机 GTK4 创建了隐藏的真实 Settings 控件树和目录选择器（合成状态），构造通过；未执行完整 GUI 点击、选择与重启。
- 隔离真实 FUSE + MockBackend daemon 冒烟通过：挂载、读取、持久写入、模拟上传、移动/删除、pin/release；只操作临时数据，未使用真实令牌。
- `0.2.11-1` amd64 deb 已从干净的提交导出构建；解包 CLI/daemon 与 release 二进制、Settings 与源码字节一致，CLI 版本为 0.2.11。核对包中无运行令牌、用户配置或数据库。解包 CLI 在隔离配置中将缺失“下载”改为 Downloads 后状态由 missing 变为 ready，远端目标/0600 权限保留，XDG 文件与旧来源不被移动；包内 CLI/daemon 的真实 FUSE + MockBackend 冒烟通过。
- **边界**：未执行 GNOME 登录/语言切换全链路、真实 OneDrive 上传再下载比对、图片清单或完整性核对、用户原会话升级与 GUI 重选全流程。未证明该旧软链接由 TwoDrive 产生，未确认图片丢失，也不保证所有来源变化会自动恢复。

### 防复发措施与后续

保留显式来源语义；检测异常并由用户重选，不基于中文/英文名称猜测目录，不把 XDG 主目录回退当作上传范围。维护来源检查、配置保存与 GTK 回归；语言变更/改名后的用户操作与重启要求已同步写入中英文指南。

### 交付记录

- 修复源码：[6f3c40a](https://github.com/LumiaBlack51/twodrive/commit/6f3c40ada47bda2ad46811115b77ac1a748b6392)；打包验证记录：[9f3ade3](https://github.com/LumiaBlack51/twodrive/commit/9f3ade3aa7058a2c0b10a90c0a9b3a17c20bb466)。
- 已合并 [PR #7](https://github.com/LumiaBlack51/twodrive/pull/7)，合并提交/发布标签指向 [ae50648](https://github.com/LumiaBlack51/twodrive/commit/ae50648a4d08ae14f7a2041d3ab5210ff220430f)。合并树与已验证源代码/文档树一致。
- [Release v0.2.11-1](https://github.com/LumiaBlack51/twodrive/releases/tag/v0.2.11-1) 已发布 amd64 deb 和 .deb.sha256；GitHub 两项资产 SHA-256 均与本地一致。deb SHA-256：`2904803b76277bb4d4305b3f2435b6a8419bdfcb91369da2c9bdf0fba234b7cf`。
- 包内故障记录为构建时验证快照；本次交付补记只更新已核实的发布链接与证据表述，不更改发布二进制。新包未安装到本机，真实服务保持原进程运行。

## TD-20260917-DELTA-PENDING-DELETE：跨刷新删除非空目录可能遗留目录

- 日期：2026-09-17；状态：已修复并完成确定性回归；影响本轮未发布的索引实现。
- 关联：folder-move-sync-2026-09-11.md 涉及目录顺序，但不是本次只读索引根因。
### 症状与触发条件
目录删除标记先到，子项在后续 delta 才删除；首次刷新为保护子项保留目录。
### 根因与证据
审阅发现首次提交后清除了暂存删除标记，后续子项删除时无法识别待删目录。
没有证据表明真实账户发生过这种顺序。
### 解决办法与恢复操作
在持久条目中保留删除标记，每次提交所有移动/删除后清理无子项的已删除条目。
无真实云端修改，无本地账户恢复操作。
### 验证结果与边界
跨两次 delta 的目录/子项删除回归已验证旧代码失败（exit 101）、修复后通过（exit 0）。
游标写入失败事务回滚也通过；证据见 windows-evidence/metadata-cache。
真实 rename/move/delete 未执行，遵守只读验证约束。
### 防复发与交付
保留跨批次删除回归及事务回滚测试；未提交、未发布。

## TD-20260917-RETRY-AFTER：共用 HTTP 重试器提前重试长限流

- 状态：代码审阅确认机制，修复后验证中。
- 触发：Graph/OAuth 返回 Retry-After 大于 30 秒；既有辅助函数将其截为 30 秒。
- 证据：原 `retry_after_seconds_are_honored_with_a_small_cap` 明确断言 120 秒变为 30 秒。没有声称当前真实账户发生过限流。
- 影响：现有刷新复用该重试器，可能早于服务端允许的时间重试；与登录增量无凭据丢失关系。
- 修复：保留完整秒数及 HTTP 日期；超出单次重试预算时停止自动重试而非缩短等待。保留结构化 HTTP 状态/延迟，不包含响应正文；浏览端持有共享等待截止时间，后续查询也不能绕过刷新限流。授权网络故障保持网络/超时分类。
- 测试：合成 HTTP 刷新返回 429/120 秒，两次查询只发出一次刷新请求，保存的凭据仍在；HTTP 日期解析、原 120 秒断言已更新。完整平台结果见 windows-progress.md。
- 本地恢复：无；未改真实权限、云端文件或既有登录。

最终补记：TD-20260916-BROWSE-DEV、TD-20260916-BROWSE-CANCEL、TD-20260917-RETRY-AFTER 已完成本轮修复后重跑；Windows Rust 42、Linux Rust 115、Flutter 11 通过，静态检查通过，6 项 FUSE ignored 未执行。当前真实账户只读根/子目录实测通过；真实异常注入及真实多页未执行。构建与证据见 windows-progress.md 2026-09-17 交付节，不将 mock HTTP 结果称为真实异常恢复验证。


## TD-20260917-INDEX-DOWNLOAD-DEV: Native milestone compilation and verification

- Date: 2026-09-17.
- Status: development checks being rerun; no released or running account data affected.
- Environment: uncommitted Windows 0.2.10 metadata/cache milestone.
- Related history: TD-20260916-BROWSE-DEV (verification workflow only).

### Symptoms and reproduction

Initial cargo check rejected four methods private to the new engine child module.
The first warnings-denied Clippy run rejected two cloned single-element slices in new tests.

### Root cause and evidence

Rust child-module method visibility does not expose private methods to the parent;
compiler E0624 identified the four calls. Clippy identified cloned_ref_to_slice_refs.

### Fix and recovery

Use pub(super) for the four engine entry points and std::slice::from_ref in tests.
No local account recovery, cloud mutation, or existing preview replacement occurred.

### Verification and limits

Native cargo check passed after visibility fixes; deterministic readonly HTTP tests passed.
Full final verification remains in progress and will be recorded in windows-progress.md.
These are development failures, not evidence of real Graph failure or recovery.

### Prevention and delivery

Keep native check, all-target Clippy and deterministic tests in milestone validation.
Uncommitted; no release or remote publication.


## TD-20260917-INDEX-UI-ENCODING: New UI strings lost in PowerShell stdin

- Date: 2026-09-17; status: fixed, widget verification passed.
- Affected environment: uncommitted new index UI only; no cloud data impact.
- Related incident: TD-20260916-W02; a different encoding boundary (PowerShell to Python stdin).

### Symptoms and reproduction

Reading newly generated Dart source showed question marks replacing Chinese labels;
Flutter analyze could not detect this semantic text loss.

### Root cause and evidence

PowerShell's default pipe encoding replaced non-ASCII Python source before execution.
The Dart source itself contained literal question marks.

### Fix and local recovery

Replaced only the new affected strings with UTF-8 patches. Added a widget test asserting
real labels and IPC refresh/download/open commands. No account recovery was needed.

### Verification and limits

The index widget and full Flutter suite passed after correction. Native UI verification
is tracked in windows-progress.md. No real Graph fault is inferred from this issue.

### Prevention and delivery

Use direct UTF-8 patches or ASCII-only helper scripts; inspect visible UI labels.
Uncommitted, no release. Development Clippy also found collapsible_if and test-module
ordering; both were corrected and warnings-denied native Clippy passed.


## TD-20260917-PACKAGING-PS51: Release packaging requires unavailable Path API

- Date: 2026-09-17; status: fixed, new-directory package rerun pending.
- Environment: Windows PowerShell 5.1, native Windows build mirror script.
- Related: TD-20260916-W01; different root cause, packaging after successful compilation.

### Symptoms and reproduction

Full Rust/Flutter release compilation succeeded; manifest generation failed because
System.IO.Path.GetRelativePath does not exist in this PowerShell/.NET Framework.
The failed artifact directory is retained and is not a delivered package.

### Root cause and evidence

build-windows.ps1 called a newer .NET API under the system PowerShell runtime.

### Fix and recovery

Derive each enumerated child file path relative to the known package prefix using
Substring; no external paths are enumerated. Expand packaging verification to full
Flutter analyze/test. Build again in a new output directory, preserving known-good previews.

### Verification, prevention and delivery

Rerun status and per-file manifest/hash verification are recorded in windows-progress.md.
No installer, cloud mutation or publication. Keep native packaging in release validation.
Uncommitted; release link not applicable.


## TD-20260917-INDEX-SIZE: Invalid file size could be treated as zero

- Date: 2026-09-17; status: corrected and deterministic regression passed.
- Environment: new uncommitted read-only metadata parser.
- Related history: folder-move-sync-2026-09-11.md negative size handling; same input
  class in a newly introduced parser, not a regression in the Linux parser.

### Symptoms, trigger and root cause

Code review found as_u64().unwrap_or(0) accepted missing/negative file sizes as zero.
This could create an empty cache candidate. No real account occurrence was observed.

### Fix and recovery

Require a nonnegative integer size for supported live file items; folder aggregate sizes
remain non-authoritative. No local recovery or real cloud operation was needed.

### Verification and prevention

invalid_file_size_is_never_an_empty_cache_candidate passes; final native/Linux suites
pass. Real invalid Graph responses were not induced. Preserve strict file-size checks.

### Delivery

Uncommitted; evidence and limits in windows-progress.md.

## TD-20260917-INDEX-SUMMARY: Full overview used obsolete cache counters

- Date: 2026-09-17; status: fixed and native UI verified.
- Environment: initial new index/cache preview; previous browsing architecture retained.
- Related history: TD-20260916-W03 concerned disconnected state, not these counters.

### Symptoms and trigger

After a real small file reached cached in the index, Full overview still showed 0 B
because it read the legacy mock files collection rather than the new persisted cloud view.

### Root cause and fix

The new file page used cloud state, while overview retained its previous data source.
Add engine-computed file_count/cached_bytes to CloudView and use these for real-mode
summary. Account status explicitly describes read-only indexing/download cache.

### Verification and boundary

Native Full first showed 46 files and 1.1 KiB, then 2.2 KiB after the second 1,132-byte
real download; Lite/IPC returned 46 files and 2,264 cached bytes. Final Full UI bytes match
the inspected build; full Flutter suite/analyze and final release build pass.
No cloud metadata/content mutation, no local account repair, no CFAPI claim.

### Prevention and delivery

Keep summary and file-page state sourced from CloudView; maintain native shared-state
acceptance and widget tests. Uncommitted; no release.

### 2026-09-17 final development verification addendum

INDEX-DOWNLOAD-DEV, INDEX-UI-ENCODING, PACKAGING-PS51 and DELTA-PENDING-DELETE
fixes passed final verification: Windows Rust 85, Linux Rust 127, Flutter 12, Nautilus 19;
Clippy warnings denied, fmt, analyze, Full/Lite release and artifact hash verification pass.
Six ignored FUSE tests remain unexecuted. CRLF copied into WSL initially failed diff
whitespace checks; changed text was normalized to LF and checks rerun successfully.
A helper UNC path inspection failed (WinError 64); no files were changed by that failed
inspection, and subsequent comparison/copy used WSL-local Python with explicit paths.
See windows-progress.md and windows-evidence/metadata-cache/results.json for real-account
results and remaining large-file acceptance blockage. No failures/skips count as passes.


## TD-20260916-W03：文件页没有明确显示断连且重连遗留错误提示

- 日期：2026-09-16
- 状态：修复；3 项 Flutter 状态回归通过；最终原生界面断连/重连复验通过
- 影响版本与环境：本地 Windows Full 开发预览，未发布
- 关联历史故障：无

### 症状与影响

实机终止隔离测试后台后，文件列表被清空且按钮禁用，但页面只说没有文件，
没有明确说明断连。检查客户端发现成功重连的 snapshot 不会清除此前传输错误。
定时读取还会短暂显示“等待后台确认”，引起布局抖动。

### 触发条件与复现

Full 文件页连接隔离 Mock，终止本轮创建的 engine，再启动同一 state。
截图证据显示初版文件页的断连说明不足。未接触日常账户。

### 根因与证据

错误条仅放在概览 status()；files() 对 null snapshot 和空文件列表用同一个提示。
客户端仅在非 snapshot 成功时清 error。busy 同时用于轮询和用户命令。

### 解决办法与恢复操作

所有管理页显示连接状态/排队数与错误；断连使用独立说明并禁用操作；
成功重连清除 transport error；pendingMutation 区分读轮询与写请求。
仅重启一次性测试后台；没有修改实际同步状态。

### 验证结果与边界

Flutter 3 项回归验证断连清快照并禁操作、重连清错误、暂停等待后台确认、
不匹配响应 ID 拒绝应用。实机暂停与 UI->IPC->排队链路已验证；
最终修正版截图见本轮进度追加记录。CFAPI 和真实 Graph 未验证。

### 防复发措施与后续

apps/full/test/engine_client_test.dart 纳入打包脚本；界面不得乐观显示写操作成功。

### 交付记录

本地隔离分支，无 Release；见 [Windows 进度](windows-progress.md)。

## TD-20260916-W01：Windows Full 依赖解析成功但原生编译失败

- 日期：2026-09-16
- 状态：已定位并在本机原生 Release 构建验证修复；CI 未执行
- 影响版本与环境：Windows 新开发预览；Flutter 3.41.7、Dart 3.11.5
- 关联历史故障：无；不是既有 Linux 用户故障

### 症状与影响

flutter analyze lib 通过，但 flutter build windows --release 失败。
未安装、未发布该失败产物，无真实账户或文件受影响。

### 触发条件与复现

pubspec 使用 fluent_ui ^4.12.0，解析到 4.16.1；原生编译报
ScrollCacheExtent 不存在、ignorePointer/onReorderItem 参数不存在。

### 根因与证据

4.16.1 源码使用了本机 Flutter 没有的 API；仅分析应用 lib 不会完整编译
依赖。构建日志明确指向 pub 缓存中的 fluent_ui-4.16.1。
另发现 Windows Pub 联网 TLS 错误，阻碍获取兼容依赖；不是同一根因。

### 解决办法与恢复操作

固定 fluent_ui 4.12.0，提交 pubspec.lock；该版本官方元数据要求 Flutter >=3.32。
本机 Windows Pub TLS 失败时，用 WSL 官方 pub.dev HTTPS 下载归档，
对照官方 archive_sha256 核验后补充开发缓存，使用离线锁定解析。
未关闭 TLS、未升级 Flutter、未修改用户日常应用安装。
缓存是构建工具依赖，非 TwoDrive 用户状态。

### 验证结果与边界

同一 Flutter 上旧依赖原生 build 失败，固定后 release exe 构建成功。
增加双发行版脚本：必须检查 flutter build 和工具退出码，不能以 analyze
成功替代。CI 配置尚未运行，多架构未验证。

### 防复发措施与后续

锁定 Flutter 和依赖，CI 原生编译 Full；跟随未来 Flutter 升级时有意更新。
完整验证见 [Windows 进度](windows-progress.md)。

### 交付记录

本地隔离分支；未推送、未 Release；提交记录待本轮检查结束补记。

## TD-20260916-W02：原生 IPC 测试误用 Windows 默认文本编码

- 日期：2026-09-16
- 状态：修复并原生重跑通过
- 影响版本与环境：新测试脚本、中文 Windows Python 3.13，非引擎数据故障
- 关联历史故障：无

### 症状与影响

测试脚本输出 PASS，同时 stderr reader 线程出现 GBK UnicodeDecodeError。
该次输出不能作为干净验收证据。

### 触发条件与复现

subprocess.run(text=True) 未指定编码，读到 Rust UTF-8 本地化 OS 错误。
连接前/关闭后 IPC 失败属于本测试预期路径。

### 根因与证据

Python traceback 位于 subprocess readerthread；引擎 JSON/错误为 UTF-8，
Windows Python 默认编码为 GBK。并非管道消息截断或同步成功丢失。

### 解决办法与恢复操作

测试子进程显式 encoding="utf-8"；仅修改测试读取层，无本地账户恢复操作。

### 验证结果与边界

重跑原生 IPC 全流程无解码异常，上传/下载字节一致，断连与设置保留通过。
其它地区语言环境未全部测试；协议规范固定 UTF-8。

### 防复发措施与后续

全部 Windows IPC 自动化脚本显式编码；CI 保留原生测试。
没有将旧次带异常的 PASS 计为验收。

### 交付记录

本地隔离分支；未发布；证据见 [Windows 进度](windows-progress.md)。
## TD-20260916-UI：Flutter 默认测试仍引用已移除的 MyApp

- 状态：已修复；完整 Flutter 6 项测试通过，静态分析无问题。
- 触发条件：前端重设计时检查完整 Flutter 测试入口。
- 根因与证据：`apps/full/test/widget_test.dart` 仍为默认计数器测试，构造不存在的 `MyApp`；实际入口为 `TwoDrive`。与此前 FUSE 故障无关。
- 解决办法：替换为真实界面的托盘/管理中心布局、导航、暂停确认与断连禁用测试，测试使用注入的隔离数据。
- 本地恢复：无。验证边界：不涉及真实 OneDrive、原生托盘窗口行为或 CFAPI。
- 防复发：后续前端修改运行完整 `flutter test` 与 `flutter analyze`。

## TD-20260916-WIN-AUTH-TEST：后端测试无条件引用 Unix 权限 API

- 日期：2026-09-16
- 状态：已定位，修复验证中。
- 影响版本与环境：0.2.10 Windows 原生后端测试编译；不影响已有运行程序。
- 关联历史故障：与 Flutter 默认测试故障无相同根因。
- 症状与复现：执行 cargo test -p twodrive-windows -p twodrive-backend --locked，报 std::os::unix 不存在和 Permissions.mode 不存在。
- 根因与证据：onedrive/tests.rs 无条件导入 PermissionsExt 并检查 Unix mode。
- 解决办法：仅在 Unix 上编译 mode 检查，Windows 继续执行实际加密存储和读取往返测试。
- 本地恢复：无；未删除账户或数据。
- 验证边界：测试结果待补；模拟令牌不代表真实 Microsoft 登录成功。
- 防复发：Windows 后端测试加入本次验证命令。

验证补记：Windows 后端 24 项与 Windows 引擎 6 项测试均通过；Clippy 无警告。Unix 权限断言仅在 Unix 编译，Windows 加密存储往返通过。真实 Microsoft 登录与云端同步不属于本条测试结论。

## TD-20260916-BROWSE-DEV：浏览开发验证受文件占用和编码影响

- 状态：原因确认，验证重跑中。
- 触发：真实目录探测从 target/debug 启动引擎后运行 cargo test；另一个编辑辅助脚本使用 Python 默认文本编码。
- 症状/证据：Windows 无法删除正在运行的 twodrive-engine.exe（os error 5）；Python read_text 报 GBK UnicodeDecodeError。
- 根因：验证程序占用重建目标；中文源码与 Windows 默认编码不匹配。编码机制与 TD-20260916-W02 相邻，但此次是编辑辅助脚本，不是 IPC 输出。
- 处理：运行引擎移到独立 browse-development-runtime 副本；编辑脚本显式 UTF-8。保留既有产物/登录凭据，不修改云端。
- 验证边界：失败运行不计通过；随后结果见 windows-progress.md。新增构建验证都从独立产物运行，不再占用 target。

## TD-20260916-BROWSE-CANCEL：空查询身份仍被客户端接收

- 状态：确定性回归发现并修复，完整重跑中。
- 触发：取消查询/注销立即清除 queryId 后，传输返回 query_id=null 的目录对象。
- 根因/证据：原客户端仅判断 incoming.query_id != queryId，null == null 时仍接收；新增 cloud_browse_test 的取消/注销测试首次失败，出现不该恢复的条目。
- 修复：没有非空活动 queryId 时不接收任何目录数据。引擎另以取消标记、登录状态、查询 ID 三重检查阻止迟到 worker 发布。
- 验证：新增快速切换、取消、注销、分页去重测试；结果补入 windows-progress.md。测试用合成目录，未退出当前真实账户。
- 恢复操作：无。防复发：保留该回归；完整 Flutter 测试与 Rust 迟到完成测试纳入本阶段检查。

验证补记（2026-09-17）：取消空查询修复后客户端回归通过。新增 widget 测试首次因测试 fixture 的 Map.addAll 泛型不匹配失败，改为显式 Map<String,dynamic> 后完整 11 项通过；这不是生产 JSON 解析故障。Dart format 暴露三个缺花括号的 lint（两处新代码、一处已有测试），首轮构建因此被静态检查拦截；已加花括号，重新打包，首次失败日志独立保留。

## TD-20260916-01：C 编译产物在挂载中无法执行

- 日期：2026-09-16
- 状态：已验证本地权限机制修复（真实 FUSE 挂载 + MockBackend）；未验证真实 OneDrive 端到端。
- 影响版本与环境：修复前源码 `2f2607a`（基于 v0.2.9-1）；用户运行版本及具体产物未核实。修复版本 0.2.10-1。
- 关联历史故障：[早期可靠性修复](reliability-fix-2026-07-24.md)，其 setattr 兼容处理仅确认请求，没有保存 POSIX mode。本次补齐该处理遗漏的本地权限功能；与 TD-20260915-01 的本地创建记录丢失机制不同。

### 症状与影响

用户报告在 TwoDrive 创建 .c 文件后，编译结果无法执行。隔离复现中编译成功，但运行报 Permission denied；chmod 调用成功却不改变可见权限。

### 触发条件与复现

在隔离 MockBackend FUSE 挂载中创建 `hello.c`，内容为 `int main(void) { return 0; }`，执行 `cc hello.c -o hello`，再执行产物并尝试 `chmod 755 hello`。修复前产物及 chmod 后权限均为 0644，执行报 errno 13。将同一源文件编译输出到本地普通目录后可运行。

### 根因与证据

修复前 `crates/twodrive-fs/src/lib.rs` 的 `attr_for_record` 把普通文件权限固定为 0644；`create` 忽略 mode/umask；`setattr` 忽略 mode。因此无法保留编译器或 chmod 设置的执行位。初次使用现有 debug 二进制复现（其构建提交未核实）后，又将新增编译/挂载回归测试放到未修改的源码上重新构建：测试稳定失败，实际权限为 `0o100644`。修复后同一编译执行流程通过。本机原挂载选项未见 noexec。

### 解决办法与恢复操作

- 为 SQLite files 增加可空的 local_mode 字段，按稳定本地身份保存普通 rwx 位。迁移可重复执行；旧条目及新导入的云端条目仍默认文件 0644、目录 0755。
- 创建文件和目录时，把 `mode & !umask & 0777` 与本地创建记录一起持久化；chmod 更新数据库及 inode 视图，不产生内容上传任务。启用 FUSE default_permissions，让内核检查本地权限。
- 上传确认、单条/批量云端更新、释放缓存及重命名保留本地权限；原子替换使用来源文件权限，不能错误继承目标文件权限。重挂载从数据库恢复。
- 已有编译产物升级并重挂载后可用 `chmod +x 文件名` 恢复执行，也可重新编译。权限只保存在本机，不作为云端元数据同步。

本地恢复操作：仅创建并卸载隔离测试挂载；未安装到用户运行目录、重启用户服务或修改用户的云端文件。deb 更新需另行安装并重挂载后生效。

### 验证结果与边界

- 88 项默认 Rust 测试通过；19 项 Nautilus 测试通过；cargo fmt、Clippy（警告视为错误）及 git diff --check 通过。
- 6 项通常忽略的实际 FUSE 测试已显式执行并通过。首次批量运行中释放下载测试因未设置 TWODRIVE_TEST_CLI 而未执行到测试主体；构建新 CLI 并设置该变量后单独重跑通过，其余 5 项首次通过。
- 新增挂载测试验证直接创建权限、编译执行、umask、移除执行位后拒绝执行、恢复执行位、移除读位后拒绝读取、目录权限及文件重命名后重挂载执行。
- 数据库回归覆盖本地创建、chmod 不改变内容状态、上传确认、两种云端导入、释放缓存、重复初始化、原子替换权限以及删除后重新导入恢复默认策略。既有旧数据库迁移测试增加权限字段兼容及重复迁移保留设置的断言。
- 从生成的 0.2.10-1 deb 解包，CLI/daemon 与 release 构建字节一致，CLI 报告 0.2.10。使用解包 CLI 在隔离 mock 挂载中编译、执行、chmod 拒绝/恢复执行及重挂载执行均通过。
- **验证边界**：实际挂载测试使用 MockBackend，不代表真实 OneDrive 上传下载端到端验证；重挂载执行使用本地缓存。没有核对用户具体编译产物；未安装系统 deb 或验证用户原会话。完整 uid/gid、特殊模式位、已解除链接句柄的 chmod 及目录时间戳不属于此次实现；挂载根目录 mode 固定，chmod 返回不支持。

### 防复发措施与后续

新增 `compiled_program_and_chmod_survive_remount`（需 FUSE、fusermount3、python3、cc，默认忽略）及 `local_permissions_survive_cloud_updates_release_and_replacement`。修改创建、setattr、数据库导入、替换或迁移时须保留“本地权限不被云端元数据覆盖”约束，发布时显式运行挂载测试。实际云端验证仍待执行，不承诺所有编译或执行问题均已消除。

### 交付记录

- 修复提交：[b9fa018](https://github.com/LumiaBlack51/twodrive/commit/b9fa01888c91a6a6696c76aa74d52819985e40a6)。
- 已合并 [PR #3](https://github.com/LumiaBlack51/twodrive/pull/3)，合并提交：[37234e4](https://github.com/LumiaBlack51/twodrive/commit/37234e49a982190dbe01f72608c69c783ee8e864)。合并树与已测试构建输入一致。
- Release：[TwoDrive 0.2.10-1](https://github.com/LumiaBlack51/twodrive/releases/tag/v0.2.10-1)，已发布 amd64 deb 及 sha256 文件；GitHub 报告的两项资产 SHA-256 均与本地一致。
- 安装包中的故障记录为发布前验证快照；本次补记仅更新交付链接，不修改发布二进制。

## TD-20260916-08：peer 升版暴露更新测试固定候选版本

- 日期：2026-09-16
- 状态：已验证本地及 Windows 原生完整测试和更新进程夹具。
- 影响版本与环境：0.1.1 构建期间的更新单测/健康夹具；不是生产反回滚逻辑故障。
- 关联历史故障：TD-20260916-07 升版验证中发现；与 Graph 故障机制不同。

### 症状与影响

完整 Rust 测试有两项失败：update must be a newer stable version。

### 触发条件与复现

把 peer 版本从 0.1.0 升至 0.1.1 后执行 cargo test --workspace --locked。

### 根因与证据

测试固定以 0.1.1 作为新版本，已不高于当前版本；生产反回滚检查正确拒绝。原生健康夹具同样写死版本。

### 解决办法与恢复操作

测试候选及原生夹具按编译版本 patch + 1 构造，第二次升级为 patch + 2。生产更新检查不改。无本地恢复操作。

### 验证结果与边界

旧测试在 0.1.1 实际失败；修复后完整 Rust 测试通过；本地及 Windows 原生夹具验证通过。此项不属于真实 GitHub 自动升级验证。

### 防复发措施与后续

候选版本应相对编译版本生成；保留不递增版本拒绝及回滚水位断言。

### 交付记录

随本次控制通道修复提交 c2776f1；见 TD-20260916-07 的交付链接。

## TD-20260916-07：真实 Graph permanentDelete 拒绝导致 peer 轮询失败

- 日期：2026-09-16
- 状态：已验证 Linux 真实 Graph 控制对象读写删除修复；Windows/Linux 双机握手待重新验证。
- 影响版本与环境：codex/peer-control cc472ea，peer 0.1.0，用户 Windows/Linux 同账号真实联调；修复版本 0.1.1。
- 关联历史故障：TD-20260916-06 为分页编码问题。本次已观测失败在删除阶段，为另一根因；之前 mock 和原生 CI 没有覆盖真实 Graph 删除能力。

### 症状与影响

用户 Windows 登录成功后持续 poll failed，Linux ping 仅入本地队列，未完成真实握手。Linux 同账号真实探针复现删除失败；原程序隐藏错误阶段。

### 触发条件与复现

使用既有隔离 peer 登录状态访问 AppFolder。真实 approot、namespace、devices/list 均 200；新建 diagnostics bucket 201，探针 PUT 201，GET 200 且内容一致；旧空 POST permanentDelete 返回 411。显式 Content-Length: 0 后返回 400 invalidRequest，脱敏消息为固定文本 API not found。轮询实际收件箱也复现 400。没有读取或删除用户普通文件。

### 根因与证据

1. reqwest 空 POST 不自动发送 Content-Length；真实 Graph 前端拒绝为 411。新增回环 HTTP 测试在旧构造上失败（header 为 None），修复后为 0，通过。
2. 本账号使用文档 drive-ID 路由仍返回 API not found。不能把微软文档列出接口当成当前账号支持证据；具体服务端原因/Windows 原始失败阶段不能仅由 Linux 结果推断。
3. 原 runtime 把读取失败当成无效信封并继续删除，网络故障时可能丢消息。新增回归在旧逻辑上失败（delete 被调用），修复后保留对象重试。

### 解决办法与恢复操作

- 控制通道空 POST 显式携带 Content-Length: 0。仅 permanentDelete 的 400 + invalidRequest + 精确 API not found 回退到普通 DELETE；本进程记忆不可用能力并提示进入回收站。401/403、其它 400、限流及 5xx 不回退，不清空回收站。
- 控制 HTTP 错误使用类型化诊断，操作名、数字状态、固定允许列表错误码和固定 hint；不输出 URL、ID、响应正文、token 或任意服务端文本。可开启逐操作成功日志。doctor 报配置 AppFolder/过期状态/refresh 是否存在（不是已授权 scope 的证明），只校验自建唯一探针。
- 发现阶段记录读取错误；收件箱读取失败保留消息，下次重试，不计为协议拒绝。

本地恢复：暂停下载目录中的 Linux peer PID 344856/344857；稳定 daemon PID 204605 及挂载未改动。使用原隔离 peer auth 做真实探针，未重新登录或更换用户设备身份。最初两次失败探针遗留的两项对象已校验测试内容后普通删除；未清空回收站。

### 验证结果与边界

- 真实账号 doctor 修复后：PUT 201、GET 200 内容一致、permanentDelete 400 触发明确能力回退、DELETE 204、随后 LIST 200 且本次对象不存在。
- 空 POST 和读取失败保留消息两项已证明旧代码失败、修复后通过。固定错误码脱敏及严格回退条件单测通过；本地 114 项默认 Rust 测试、19 项 Python 测试通过。格式与 Clippy 通过；Windows 原生 72 项默认测试及额外更新进程测试通过，Linux CI 114 项默认及额外更新进程测试通过。
- Python 初次读取重定向内容曾发生网络 timeout；Rust doctor 读回成功。不据此推断 Windows 网络状况。
- 本机两个临时隔离身份通过真实 Graph 完成互相 Authenticated、双向 PING/PONG round trip verified；临时云端 presence/收件箱及本地测试身份/token 已清理。它们运行于同一 Linux 主机，不是 Windows/Linux 双机实测。
- 下载目录 0.1.1 发布版再次运行 doctor 全部通过，其中 permanentDelete 直接返回 204。**更正边界**：400 API not found 是当时的真实响应，不能推断该账号永久不支持此 API；程序仍优先永久删除，仅对精确能力响应作本进程回退。
- **边界**：以上是 Linux 对真实 Microsoft Graph，不是 mock；仍不等于 Windows/Linux 真机握手与 ping/pong。Windows 需替换新 exe 运行 doctor、互信后验证。普通删除进入回收站，不保证永久清除。

### 防复发措施与后续

保留 empty_permanent_delete_post_has_explicit_zero_content_length、only_explicit_api_not_found_allows_recycle_fallback、diagnostic_never_echoes_untrusted_error_fields、failed_mailbox_read_is_not_deleted_or_counted_as_rejected。新云端功能须增加真实账号探针，不能仅以 mock/CI 宣称支持。说明和重测命令见 [双机指南](peer-quickstart.zh-CN.md)。

### 交付记录

修复提交：[c2776f1](https://github.com/LumiaBlack51/twodrive/commit/c2776f10148ee6b90dd65b13023281e04537ece6)。[原生 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35074280918) Windows/Linux 均成功；Windows release exe 原生版本、健康、自身份持久化及系统 DLL 依赖检查通过。Linux 发布版 SHA-256：`8f03fba31b94a3a59f7f58e7e3466b4a07d88f6e91be49775319b21057786d2e`，已替换本地下载目录 peer，并保留 0.1.0 备份。未发布 GitHub Release；稳定 TwoDrive 不发布、不安装、不重启。


- Windows exe：7662080 bytes，SHA-256 `84b33e056bc417c574b6016ec3d72376f264e38680ce9f36c8170fe5b14187b0`；与下载 CI SHA256SUMS 一致，PE AMD64 已核对。CI artifact ID 10436929068。
- 本地手动替换包：`dist/peer-0.1.1/twodrive-peer-0.1.1-windows-x86_64.zip`，亦已复制到本地下载目录。ZIP SHA-256 `c50ec246e014018964182a75546fbe9bf42b8c06363554596fa4bc6829b3ec30`。包内 exe 是上述未修改的 CI 原件；没有自动更新签名 manifest、令牌或私钥。
- 原用户 Linux 身份短时运行 40 秒：poll/read 错误均 0，6 次 permanentDelete 204；未观察到原 Windows 对端认证。已停止测试 peer 进程，等待两端替换/重启；稳定 daemon PID 204605、启动时间 14:18:36 CST 不变。
- 后续记录提交仅补充验证/交付，不改变已构建可执行文件输入。尚未执行 Windows 用户设备真实 doctor/双机 ping；不得把原生 CI 或本机双进程当作这项完成。

## TD-20260916-06：控制通道分页校验拒绝等价的 OneDrive ID 编码

- 日期：2026-09-16
- 状态：已验证单元机制：旧代码失败、新代码通过，Linux/Windows 最终 CI 通过；真实 Graph 分页未验证。
- 影响版本与环境：peer 控制 provider，至 fb05bcb；既有文件系统分页未改动。
- 关联历史故障：TD-20260916-02 的不可信 URL 约束新增时边界遗漏，不是同一信任根因。

### 症状与影响

构造合法等价 nextLink 时，包含 `!` 的 ID 与 `%21` 编码的预期路径不相等，严格字符串校验会拒绝后续页。实际账号返回形式未核实。

### 触发条件与复现

预期路径包含 `ABC%21123`，nextLink 同一位置为 `ABC!123`。新增 pagination_accepts_equivalent_onedrive_id_encoding 在修复前失败。

### 根因与证据

校验要求正确的 HTTPS Graph 来源和相同资源，但直接比较转义后的路径字符串，混淆编码差异与跨资源访问。

### 解决办法与恢复操作

已逐路径段解码后比较，同时保留段边界、HTTPS、主机、端口、凭据、fragment 限制；不放宽到任意 Graph URL。无需用户数据恢复。

### 验证结果与边界

旧代码测试失败；修复后等价编码、错误来源及资源逃逸测试通过，Clippy 和格式检查通过。模拟 URL，不是实际 Graph 分页抓包。

### 防复发措施与后续

保留等价编码和不同资源拒绝用例，不能只验证恶意 nextLink。

### 交付记录

[修复提交 31ca30e](https://github.com/LumiaBlack51/twodrive/commit/31ca30e0433fbbf733930e14a88315058d472ec1)；[最终 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35068850290)。

## TD-20260916-05：peer 篡改测试可能没有实际改变随机密文

- 日期：2026-09-16
- 状态：已修复测试反例构造；这是审计发现的测试缺陷，不是已确认的协议绕过。
- 影响版本与环境：peer 实验分支 0.1.0 测试代码，至 389bed8。
- 关联历史故障：审查 TD-20260916-04 的 CI 失败期间发现；其实际失败来自 daemon 时间窗，两者根因不同。

### 症状与影响

原篡改测试把密文首字节替换为 ff；随机首字节本来为 ff 时没有篡改，约 1/256 概率错误要求合法消息被拒绝。

### 触发条件与复现

静态反例：首字节为 ff 时 replace_range(..2, "ff") 保持输入不变。没有将此前任何 CI 失败归因于这一机制。

### 根因与证据

替换为固定值不能保证改变随机输入。协议正常接收未改动的有效消息并非安全漏洞。

### 解决办法与恢复操作

把首个十六进制字符改成与原值不同的字符，并断言新旧密文确实不相等。没有生产代码或本地恢复操作。

### 验证结果与边界

本地篡改、会话、目标和过期测试重跑通过，Clippy/格式检查通过；Windows 原生同项测试也已通过。没有真实云端攻击测试。

### 防复发措施与后续

保留 assert_ne!，篡改类测试必须先确保构造的输入不同于有效原件。

### 交付记录

[修复提交 fb05bcb](https://github.com/LumiaBlack51/twodrive/commit/fb05bcb943b086fc2413ab049fc5cbd17c2e94c3)；[最终 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35068850290)。

## TD-20260916-04：CI 中既有 known-folder 持久化测试的短观察窗失败

- 日期：2026-09-16
- 状态：已验证测试调度依赖修复：本地与最终 Linux CI 通过；生产回归尚无证据。
- 影响版本与环境：重构前继承的 daemon 测试；peer CI 35068052022 的 Linux runner。
- 关联历史故障：[响应与恢复](responsiveness-recovery-2026-09-08.md)描述需要保留的逐任务持久化行为；[历史调查](background-download-investigation-2026-09-08.md)曾记录另一项并发测试波动，不能据此认定相同根因。

### 症状与影响

completed_known_folder_job_is_persisted_while_slower_job_runs 在 CI 断言失败，本地完整套件此前通过。失败发生在旧 daemon 测试，不是 Windows peer 编译或握手失败。

### 触发条件与复现

测试只轮询 25 次、每次 20ms，模拟快/慢上传分别休眠 100/800ms。注入额外 600ms worker 启动延迟后，旧测试稳定在 0.51 秒失败，尚未进入生产上传处理。

### 根因与证据

原 CI 日志只证明 500ms 内未观察到目标状态，不能分辨启动调度慢与中间状态被漏采样。生产上传实现本阶段没有修改；具体 CI 调度轨迹无法还原。应使用可控的慢任务完成屏障，而非依赖两次定时休眠之间恰好采样。

### 解决办法与恢复操作

仅改测试：慢上传等待释放信号，主线程观察快结果持久化后再允许慢任务结束，并始终释放/回收工作线程。生产 daemon 不变，无本地恢复操作。

### 验证结果与边界

同样的 600ms 模拟启动延迟下，旧测试失败；新屏障版本在 1.55 秒内通过，且断言仍要求慢任务未完成时快结果已经持久化。最终原生 CI 通过。属于测试确定性验证，不代表真实 OneDrive 上传性能。

### 防复发措施与后续

保留“慢任务仍未结束时快结果已持久化”的断言，避免简单放宽断言或只重跑掩盖问题。

### 交付记录

[CI 35068052022](https://github.com/LumiaBlack51/twodrive/actions/runs/35068052022)，[修复提交 fb05bcb](https://github.com/LumiaBlack51/twodrive/commit/fb05bcb943b086fc2413ab049fc5cbd17c2e94c3)；[最终 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35068850290)。

## TD-20260916-03：peer 健康检查无法写入相对输出文件

- 日期：2026-09-16
- 状态：已验证：本地与 Windows 原生 CLI 回归及 release 自检通过。
- 影响版本与环境：实验 peer 0.1.0，截至 7f357bd；不影响现用 TwoDrive 实例。
- 关联历史故障：无；不是 TD-20260916-02 的协议信任问题。

### 症状与影响

原生 CI 在运行 release exe 的 `health-check --output health.json` 时退出失败；构建本身和绝对路径的原生更新子进程测试通过。阻断 artifact 交付检查，未造成已有文件丢失。

### 触发条件与复现

在空的隔离当前目录执行上述命令，输出使用不带父目录的相对文件名。本地新增 CLI 回归测试在旧函数上稳定失败，退出码 1。

### 根因与证据

Path::parent 对单文件名返回空路径；原子写入尝试在空路径创建临时文件，而不是当前目录。core 新增的私密文件写入辅助函数存在相同边界，虽然 peer 的实际 token 路径使用绝对父目录。

### 解决办法与恢复操作

已将空父目录规范为 `.`，保持同目录原子替换和 Unix 目录 fsync。没有用户数据恢复操作。

### 验证结果与边界

`release_health_command_writes_relative_output_without_state_side_effects` 在修复前失败，修复后通过；格式、Clippy 和 diff 检查通过。Windows 原生同一 CLI 回归及 release 自检通过。测试不访问 OneDrive，不代表真实登录通过。

### 防复发措施与后续

保留真实 CLI 子进程测试，验证生成预期健康结果且不创建 peer 运行状态。原生 CI 在上传产物前执行 release 健康检查。

### 交付记录

触发 CI：[35067546462](https://github.com/LumiaBlack51/twodrive/actions/runs/35067546462)。[修复提交 69fb3d9](https://github.com/LumiaBlack51/twodrive/commit/69fb3d94c797867f8d702daf7801d6f73e04000f)；[最终 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35068850290)。artifact 摘要见 [交付记录](peer-implementation.md#verified-delivery)。

## TD-20260916-02：relay-lab 原型的云端信任与控制重放边界不足

- 日期：2026-09-16
- 状态：已验证新 peer 的本地及原生 CI 安全机制；真实 OneDrive 端到端未验证。
- 影响版本与环境：用户提供的独立 twodrive-relay-lab 0.1.0 原型；不据此认定稳定 TwoDrive 存在相同攻击面。
- 关联历史故障：无；与 FUSE 权限和本地创建记录故障无关。

### 症状与影响

原型把同一 OneDrive 中自签名 manifest 列表作为 trusted_senders；可写云端的主体能添加自有密钥，再发送可被接受的加密消息。加密不等于验证物理设备归属。原型没有设备控制握手与会话重放状态，不能直接当作控制协议使用。

### 触发条件与复现

审计 identity.rs 的 manifest 自签名验证、graph.rs::list_manifests 和 crypto.rs::verify_payload 调用链，确认信任列表完全来源于云端。新协议用未知自签名设备、重复信封、过期信封、错误会话/目标作为合成回归输入。旧原型复制到隔离临时目录后，新增两项安全要求测试；均可重复失败（云端注入身份被接收、同一信封两次解密均被接收）。原始工作区原型未修改，也未在真实云端进行攻击测试。

### 根因与证据

云端发现与本地授权未区分；消息签名未覆盖目标、信封 ID 和有效期等外层路由字段。详细分析见 [peer audit](peer-implementation.md)。Windows 兼容性检查还发现已有 core credential 和 provider upload persistence 导入 Unix 专用 API，原样依赖无法编译 Windows peer；这属于此前 Linux-only 代码的移植边界。

### 解决办法与恢复操作

复用 provider/OAuth，不导入原型的重复认证实现。新协议增加本地指纹信任、完整上下文签名、随机会话、挑战握手、重放窗和输入上限。Windows 使用 DPAPI，Unix 保持 token 格式兼容并原子写入。更新发布密钥独立于设备身份和云端。

本地恢复操作：无。仅在独立 worktree 和测试临时目录构建运行，未操作稳定服务和已有用户数据。

### 验证结果与边界

初轮 peer 7 项、core 31 项 Linux 测试通过，包括未知设备拒绝、双向握手与 ping/pong、重放/过期/会话拒绝，以及正常更新、错误签名、损坏文件、健康检查失败回退。最终 110 项默认 Rust 测试、19 项 Python 测试、6 项实际隔离 FUSE 测试、独立 daemon smoke 通过。Windows 原生 CI 的 68 项默认测试及额外原生更新进程测试通过。测试为模拟，非真实 OneDrive 端到端。已在原型副本运行 audited_cloud_manifest_is_not_local_authorization 和 audited_control_replay_must_be_rejected，两项均因预期的安全边界缺失而失败；新 peer 的对应未知身份和重放拒绝测试通过。两套实现协议不同，这不是将同一测试直接移植到旧生产版本。

### 防复发措施与后续

协议层测试固定 cloud discovery != local trust；后续传输仅提供不可信字节。保持独立发布根、反回滚检查和不覆盖当前 exe 的更新事务。更多故障及最终验证将在交付前补记。

### 交付记录

实验分支 codex/peer-control；[构建提交 31ca30e](https://github.com/LumiaBlack51/twodrive/commit/31ca30e0433fbbf733930e14a88315058d472ec1)；[最终 CI](https://github.com/LumiaBlack51/twodrive/actions/runs/35068850290)。产物摘要见 [交付记录](peer-implementation.md#verified-delivery)。未发布或替换稳定 TwoDrive。

## TD-20260915-01：新建目录中解压文件报 I/O 错误

- 日期：2026-09-15
- 状态：已验证本次故障机制修复；真实 OneDrive 完整上传、下载比对尚未执行。
- 影响版本与环境：修复前源码包含于 0.2.8；故障发生在 Linux/Nautilus + TwoDrive FUSE 挂载。本机旧程序早于本次发布，具体包版本未核实；不据此断言最早受影响版本。
- 关联历史故障：[父目录移动与子项同步顺序](folder-move-sync-2026-09-11.md)。关联在于保护待处理目录的逻辑；没有证据证明此前所有 I/O 错误都是本次根因。

### 症状与影响

向 OneDrive 挂载中新建的目录解压 ZIP，创建 `__MACOSX/._…` 文件时出现输入/输出错误。守护进程记录：`twodrive create upload error: created upload record disappeared`。

失败会中断解压并留下部分结果。普通文件也受影响，`._` 文件名只是本次最先暴露问题的位置。原 ZIP 的 48 个条目通过 CRC 检查；未发现原压缩包损坏的证据。

### 触发条件与复现

1. 在本地建立目录，使其云端创建操作仍处于待处理状态。
2. 在其中建立嵌套目录并创建文件，例如解压 ZIP。
3. 在父目录同步完成前执行本地文件创建。

隔离测试中保留父目录待处理状态，可稳定触发旧代码错误。待移动父目录使用同一过滤机制；新增解压测试直接覆盖的是待创建目录。

### 根因与证据

`TwoDriveFs::create_upload` 将本地新建文件交给 `Database::upsert_metadata`，随后再分别登记缓存和 Writing 状态。

`upsert_metadata` 用于云端元数据导入。为防止过时云端数据破坏尚未完成的本地目录操作，它会跳过待创建或待移动目录下的条目。此次本地写入也被跳过，函数却正常返回；后续查询找不到记录，最终向 FUSE 返回 EIO。

将新增回归测试放回旧 `create_upload`，稳定得到与生产日志完全相同的错误。修复后同一测试通过。原来的分步写入还暴露了中间状态；本次一起消除了这个窗口，但没有把它当成已证实的另一条生产故障原因。

### 解决办法与恢复操作

新增 `Database::create_local_file`：通过单条 SQLite INSERT 同时登记本地身份、路径、缓存路径、访问时间和 Writing 状态。本地创建不再经过云端导入过滤；唯一约束防止覆盖同名记录。

保留云端旧元数据过滤和父目录操作完成前暂停上传的规则。新建内容由写入句柄持有缓存锁，后续按原有恢复流程上传。

本地恢复：备份数据库和旧程序后更新用户目录下的 CLI、daemon，并重启服务；挂载恢复，真实 OneDrive 元数据同步成功。没有自动合并或删除历史部分解压结果和无关云端冲突。

### 验证结果与边界

- 87 项默认 Rust 测试、19 项 Nautilus 测试通过；格式检查及 Clippy（警告视为错误）通过。
- 5 项通常忽略的实际 FUSE 挂载测试全部显式执行并通过，包含待处理目录中的 ZIP 解压。
- 普通文件和 AppleDouble 文件在父目录待处理时保留正确字节，父目录恢复后向模拟云端正确上传；过时元数据未替换本地记录。
- 验证了待删除路径重用及同名创建冲突时保护现有记录。
- 安装包内容和构建输入一致，GitHub 安装包摘要与本地 SHA-256 一致。
- **验证边界**：挂载测试的云端为 MockBackend。真实 OneDrive 只验证了元数据同步与原 ZIP 读取校验，未完成原 ZIP 的“解压 → 上传 → 重新下载逐文件比对”。不能据此保证任何来源的 I/O 错误都不会再发生。

### 防复发措施与后续

已提交的回归测试：

- `archive_files_in_pending_nested_folders_survive_until_upload`
- `zip_extraction_into_pending_folder_on_mount`（需要 FUSE，默认忽略，相关发布验证应显式运行）
- `local_create_is_atomic_and_can_reuse_a_pending_delete_path`

后续修改本地创建、云端导入过滤或父子同步顺序时，应运行上述相关测试，保留“本地创建不能被云端过滤跳过”和“子项上传等待父目录完成”两项约束。

未完成的验证：真实 OneDrive 原 ZIP 端到端比对。若再次出现相同报错，应先核实运行程序版本、触发状态和日志，再判断是否属于本次机制的回归。

### 交付记录

- 根因修复：[c84714f](https://github.com/LumiaBlack51/twodrive/commit/c84714f)
- 同批发布的既有 Nautilus 修复：[030132f](https://github.com/LumiaBlack51/twodrive/commit/030132f)，与解压故障根因不同。
- Release：[TwoDrive 0.2.9-1](https://github.com/LumiaBlack51/twodrive/releases/tag/v0.2.9-1)
- 详细调查：[Archive extraction I/O error](archive-extraction-2026-09-15.md)
