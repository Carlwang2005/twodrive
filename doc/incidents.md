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

## TD-20261007-01：仅云端文件选择始终保留后缺少下载等待标志

- 日期：2026-10-07
- 状态：已验证本地状态图标与原生异步命令机制；真实 Nautilus GUI 与 OneDrive 端到端未验证。
- 影响版本与环境：本次调查基于 main 源码 `bdacfb9`；用户当前运行版本未核实。修复包版本为 0.2.11-2，Rust 二进制版本继续为 0.2.11。
- 关联历史故障：[目录浏览与延迟释放图标](directory-browsing-2026-09-08.md)、[固定策略可靠性](reliability-fix-2026-07-24.md)、[后台下载与恢复](responsiveness-recovery-2026-09-08.md)。本次补齐现有图标映射对“已固定但尚无缓存”的展示遗漏；未证明是历史修复回归，也未把它等同于 TD-20260924 的 Nautilus 卡顿。

### 症状与影响

用户报告对只有云图标的文件选择 Always keep on this device 后没有可见反馈。期望策略生效、下载尚未完成时显示云和 pin，下载完成后仅显示 pin。未确认下载本身失败或数据损坏。

### 触发条件与复现

仅云端文件，选择始终保留，观察下载期间及完成后的 Nautilus 图标。隔离 SQLite 回归复现了显式/继承固定、hydrating 与 pinned 尚无缓存的阶段；原生 Gio 冒烟在临时 mock 数据库中人为延迟 350 ms，再执行实际 CLI 的 MockBackend 下载，验证菜单请求到缓存落盘的转换。不使用真实账户。

### 根因与证据

- `run_action` 原先只启动 Python 工作线程，没有在菜单接受请求时设置图标反馈；扩展环境已有原生主循环可能持有 GIL 的历史限制，状态查询已改用 Gio 子进程，菜单命令却仍依赖线程。本次移除这项线程依赖；未取得用户现场调用栈，不能断言其原操作一定因 GIL 完全未启动。
- Nautilus 查询只读取 state/release_pending，未结合显式或继承 pin 与 cache_path。`recompute_pin_inheritance` 对固定无缓存文件设为 hydrating，而 `begin_hydration` 又会在下载完成前设为 pinned 并清空 cache_path。因此旧展示可能只有同步图标，或过早只显示 pin，无法区分固定策略与缓存完成。
- 新回归在未修改的 `bdacfb9` 扩展上失败，固定无缓存阶段得到单独 cloud/syncing/pin；修复后通过。并未证明下载失败、数据损坏或整个 Nautilus GUI 卡住。

### 解决办法与恢复操作

- 菜单选择仅云端文件时立即提交临时 `pin_pending` 展示，顺序添加 cloud、pin 两个已有图标。持久状态查询使用有效固定策略与缓存字段；在缓存未发布期间持续双图标，发布后只显示 pin，不依赖临时标记的 60 秒期限。
- 数据库状态确认工作或完成后接管临时反馈；每个命令完成/失败时单独清除其临时状态，重新读取真实状态，不把失败请求永久当作已固定。多选仍按顺序执行，单项失败后继续处理其余项。
- 菜单命令改用原生 Gio 异步子进程与完成回调，保留输出、失败通知及一小时超时；上传、错误/冲突与延迟释放的优先级保留。目录汇总尚无缓存的固定子项；新发现的未固定云端子项不被误认为等待固定。
- 中英文首页与使用指南补充双图标含义。未改动数据库迁移、固定策略、下载算法或缓存发布机制。

**本地恢复操作**：只使用临时 SQLite、临时命令包装器及 MockBackend 下载；未安装扩展或新包，未重启 Nautilus/用户服务，未修改用户配置或云端文件。实际会话加载新扩展仍需更新安装并重启 Nautilus。

### 验证结果与边界

- 25 项 Nautilus 回归通过，覆盖点击立即反馈、固定无缓存的显式/继承策略、缓存完成、目录与相邻路径边界、错误/上传/释放优先级、单项失败反馈清除、多选顺序与逐项清除，以及 sync/status/unpin/release 命令参数和通知。新增 pin 回归放入临时目录搭配旧扩展运行，确认旧代码失败；文件信息与菜单回调使用替身，不能称为完整 GUI 验证。
- 真实 Gio/GLib + Nautilus 4 模块冒烟通过：接受菜单请求后立即双图标；模拟延迟阶段原生主循环继续响应；实际 mock CLI 发布缓存后只剩 pin，缓存字节与公开 mock 样本一致。另验证子进程非零退出及 stderr 合并、启动失败、强制超时均只回调一次。超时测试将一小时定时器缩短到 50 ms，未实际等待一小时。
- 101 项 Rust 默认测试、4 项 Settings 回归通过；cargo fmt、Clippy 警告视错误、git diff --check 通过。6 项环境相关 FUSE 测试默认忽略，本次未额外执行（未修改文件系统实现）。
- 0.2.11-2 amd64 deb 从源码提交 `a749e430acde26a4364c5ee2671b472281b17f9a` 的干净 Git 导出构建；解包 CLI/daemon 与 release 产物、扩展/托盘/Settings 与该提交源码字节一致。包内 CLI 版本为 0.2.11；扩展的 25 项回归重新通过。核对软件包无运行令牌、用户配置、数据库或私人构建路径；原有未提交调查不进入包内记录。
- 使用包内 CLI/daemon 运行隔离真实 FUSE + MockBackend 冒烟，挂载、读取、持久写入、模拟上传、移动/删除、pin/unpin/release 均通过并卸载临时挂载。此项是包内容验证，不是 6 项默认忽略测试的补跑，也不代表真实 OneDrive。
- 本机已安装 CLI、daemon、Nautilus 扩展摘要与构建前一致；用户服务保持 MainPID=3051、active、NRestarts=0。未安装 deb 或重启实际服务/Nautilus。
- **边界**：未验证实际 Nautilus 窗口中图标布局、完整右键操作、多扩展交互、用户原会话升级或真实 OneDrive 下载。原生冒烟使用文件信息替身、合成等待状态和 MockBackend，不代表真实 Graph 端到端，也不能证明用户全部“没反应”现象来自同一根因。

### 防复发措施与后续

保留 `PinEmblemTests`、`ActionSequenceTests` 回归，特别是不能仅凭 state=pinned 就宣布本地内容下载完成。菜单动作和状态查询均避免依赖扩展中的 Python 工作线程。后续若再次出现无反馈，需核对已加载扩展版本、持久策略/缓存状态及原生命令执行结果；实际 GUI 和真实云端验收仍待执行。

### 交付记录

- 用户授权将修复与 deb 提交到新分支，明确不更新本机。已推送 [codex/pin-download-feedback](https://github.com/LumiaBlack51/twodrive/tree/codex/pin-download-feedback)；修复源码提交为 [a749e43](https://github.com/LumiaBlack51/twodrive/commit/a749e430acde26a4364c5ee2671b472281b17f9a)，软件包及验证记录提交为 [18120fc](https://github.com/LumiaBlack51/twodrive/commit/18120fc6286e797f57e487c58b5c57a019b54d1d)。远端分支提交及 deb/校验文件的 Git blob 与本地一致；未合并或修改 main。
- 已构建并验证 [0.2.11-2 amd64 deb](../dist/twodrive_0.2.11-2_amd64.deb) 与 [SHA-256 校验文件](../dist/twodrive_0.2.11-2_amd64.deb.sha256)，随本分支提交，不创建 Release，也不安装本机。deb 为 6,419,576 字节，SHA-256：`e2da4946333cdecdd7f67cb7b1481acb8de93a0675c807cd0a682f7c412a230f`。
- 包内故障记录为源码构建时快照；本次包验证与交付补记不更改包内二进制或扩展。原有未提交调查记录和无关文件保留在工作区，不纳入本次提交或软件包。

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

## TD-20260924-02：Nautilus 偶发等待对话框的 48 小时监测

- 日期：2026-09-24
- 状态：调查中 / 根因未确认，已启用本机滚动诊断。
- 影响版本与环境：本机 Nautilus 46.4、TwoDrive 0.2.10-2；本次未发现具体卡顿时间点，运行 daemon 与 TD-20260924-01 所核实版本相同。
- 关联历史故障：TD-20260924-01、[Nautilus 启动](nautilus-startup-2026-09-11.md)、[目录浏览](directory-browsing-2026-09-08.md)。症状相近，不推断是旧修复回归。

### 症状与影响

用户报告 Nautilus 偶发无响应，桌面显示“等待或退出”。影响目录浏览；本次没有确认进程退出或数据损坏。

### 触发条件与复现

发生时间、所在目录、操作顺序均未确定。当前主动启动的 Nautilus 服务 D-Bus Ping 正常，未复现卡顿。

### 根因与证据

根因未确认。TD-20260924-01 未取得卡顿时 Nautilus 堆栈；本次也尚无现场证据，不能仅凭相同提示认定 FUSE、扩展、GIO 或网络为根因。

### 解决办法与恢复操作

新增本机用户级 `twodrive-nautilus-watch.service`，每五秒在 Nautilus 存在时测量 D-Bus Ping。响应达到 700 ms、失败或超时，记录 Nautilus 和 TwoDrive daemon 线程状态与内核等待点；持续异常每 30 秒再采样，并记录恢复时刻。私有 SQLite 逐次删除超过 48 小时的记录，不收集私人路径、文件名、令牌或原始 journal 消息。脚本、查看方式及解释边界见 [Nautilus 卡顿监测说明](nautilus-hang-monitor-2026-09-24.md)。未修改同步或 Nautilus 扩展机制，未重启 TwoDrive 服务。

### 验证结果与边界

- 临时数据库分别测到 Nautilus 不存在和存在时正常 Ping（约 8 ms）。将阈值临时设为 1 ms 后成功生成 Nautilus/daemon 线程快照；注入超过 48 小时的记录后，下一次探测将其删除。
- 用户服务已启用并运行；实际数据库目录为 0700、文件为 0600，已写入 Nautilus 存在时 17 ms 的正常响应记录。原 TwoDrive daemon 保持 MainPID 2455、active、NRestarts=0。
- **边界**：尚未观察到真实卡顿；内核等待点只能缩小调查范围，不能单凭它确认应用层根因。未对真实 OneDrive 做端到端验证，也未证明卡顿已修复。

### 防复发措施与后续

卡顿时尽量选择“等待”并记下时间、目录类型；根据监测时间戳关联脱敏系统日志，再针对确认机制补有意义的回归测试。当前缺乏稳定复现，不能证明旧代码失败与修复后通过。

### 交付记录

本地诊断脚本及说明已保存于仓库工作区，服务安装在当前用户目录；尚未提交、发布或生成安装包。

2026-10-13 提交前补记：按用户要求将此前未提交的调查、监测脚本与说明纳入本仓库。脚本原样保留，本轮8项隔离检查通过：SQLite目录/文件权限、拒绝公开目录/数据库、48小时清理与进程不存在状态、注入慢响应后的恢复记录、30秒异常采样限流、非法间隔在创建状态前拒绝，以及只写临时数据库的本机只读探测。慢响应/错误/恢复用注入结果验证，未复现实际卡顿；不改原有“根因未确认”结论。本轮未安装或重启监测服务、未打开用户诊断数据库、未操作真实云端。提交和 dev 合并链接待实际产生后补充。

## TD-20260924-01：弱网时 Nautilus 无响应及 PDF 保存结果不明确

- 日期：2026-09-24
- 状态：调查中 / Nautilus 卡顿及此前 PDF 保存延迟根因未确认；用户确认 PDF 目前显示已保存。
- 影响版本与环境：本机 Linux，Nautilus 46.4；运行 TwoDrive daemon SHA-256 与本地已发布 0.2.10-2 deb 提取产物一致（`2500d0922ede9ffa736ccaa4611c3d670859d4cced59a14bf2533b8de1312332`）；Papers 50.2 inplace-save，安装提交 `16573ca27c3e72932f9d7190c3686711c5d489d8c63f80658e6a74267e77e95e`。
- 关联历史故障：TD-20260922-01、[PDF 保存可靠性](pdf-save-reliability-2026-09-09.md)、[Nautilus 启动](nautilus-startup-2026-09-11.md)、[目录浏览](directory-browsing-2026-09-08.md)。未证实为其中任何机制的复发。

### 症状与影响

用户初报网络不佳时进入 TwoDrive 导致 Nautilus “崩溃”；后续明确为点击目录后无响应，桌面提示“等待或退出”，因此更正为卡顿，不能记录为已确认进程崩溃。已打开课程 PDF 有批注，Ctrl+S 和另存下载目录一度看不到成功结果。调查期间用户确认 PDF 已显示保存。

### 触发条件与复现

原现场未保留卡住的 Nautilus。当前打开 TwoDrive 父目录、挂载根目录、目标课程目录，各执行五次 D-Bus Ping，响应约 3–13 ms，未复现挂起。已请求用户重现时保留窗口并选择等待，以便捕获阻塞位置。

### 根因与证据

**根因未确认，不将弱网与两个症状直接等同为因果。**

- TwoDrive 服务自本次开机连续运行，active、NRestarts=0；Papers 原进程仍在，无本次断言退出证据。
- 日志确有 PDF 上传重复超时，09:53:53、09:56:54、09:59:56、10:02:57（Asia/Shanghai）均报告 `dirty upload remains queued for <document>: <remote request>: operation timed out`。说明云端请求失败并保留重试，不证明本地保存失败或 Nautilus 主线程正在等待该上传。
- 本地下载目录已有 10:47 写入的 PDF。它与从当前 Papers 文件描述符复制出的备份 SHA-256 相同（不公开私人文档摘要），大小均为 14,668,221 字节。只读数据库查询对应文件为 uploading、cache 存在、release_pending=0；本地持久化与云端完成应分开说明。
- Papers 仍持有三个已被原子替换的旧文件描述符，fstat 可读到各自旧属性；通过 /proc 重新打开旧描述符因旧 FUSE inode 不再支持 open 而返回 ENOENT。此操作不等价于原进程原描述符 read 失败，不能据此推断批注丢失或认定原子替换修复回归。
- Nautilus 当前进程的调试器附加被 ptrace 限制拒绝；未取得原故障堆栈，也未修改系统 ptrace 安全设置。

### 解决办法与恢复操作

没有足够证据支持特定程序修复，尚未修改 TwoDrive/Papers 代码。未重启挂载或关闭用户 PDF 窗口。

本地保护操作：将当前 PDF 只读复制到用户私有状态目录 `~/.local/state/twodrive/recovery-20260924/papers-open-fd-47.pdf`，目录权限 0700；该副本仅包含已落盘内容，不宣称提取了未保存的内存编辑。下载目录已有副本不是调查程序生成的。为诊断打开过 Nautilus；首次命令使用 15 秒 timeout，会终止该次诊断进程，该退出不算故障复现。随后无 timeout 启动并完成目录响应检查。

### 验证结果与边界

- pypdf strict=True 解析全部 92 页并解码内容流，统计到 3 个 FreeText 和 8 个 Ink 批注；仅有对象头多余空白警告。pdftotext 全文到 /dev/null 成功。未读取或记录批注正文，未逐页渲染核对，也不能证明所有预期批注均在。
- 当前源码隔离真实 FUSE + MockBackend：`mounted_large_listing_and_copy_remain_responsive_during_download` 通过；后台下载延迟 3 秒时，10,000 条目录枚举与 16 MiB 写入约 140 ms。
- `gio_directory_metadata_does_not_download_unknown_large_files` 通过：30 个虚拟 4 GB 未知格式文件的完整 GIO 元数据查询约 28 ms、零下载；显式复制仍可读取正确字节。
- 19 项 Nautilus Python 回归通过。
- **边界**：上述测试未重现原 GUI 主线程卡顿，未覆盖所有扩展、桌面和网络状态；未进行真实 OneDrive 上传完成后重新下载比对。PDF 当前能保存不是已证明修复原机制。没有确认任何新代码修复，也不承诺再次弱网时不会卡顿。

### 防复发措施与后续

下一步需要故障仍存在时的 Nautilus 主线程及 TwoDrive 请求阻塞证据，区分 FUSE、扩展、GIO 和桌面自身阻塞。若附加仍受限，可在受控复现场景从调试器启动新的 Nautilus；不能仅靠缩短网络超时或重启挂载来宣称根治。确认可复现缺陷后补充旧失败/新通过回归。

### 交付记录

仅保存本次本地调查记录；尚未确认 TwoDrive 新缺陷，因此未生成或发布声称修复本问题的 deb、未推送代码。此前 TD-20260922-01 的本地未提交补记保持原样，未纳入无关提交。

2026-10-13 归档补记：本条与 TD-20260922-01 的既有调查现按用户要求一并提交留档；保留原证据、Papers 独立项目的修复归属及全部验证边界。本轮只归档，不重新执行此前安装、文件恢复或云端操作。

## TD-20260922-01：PDF 保存时 Papers 批注处理断言退出

- 日期：2026-09-22
- 状态：已修复并安装 Papers 批注生命周期与中文字体回退问题；已验证隔离回归和本地保存，用户完整 GUI 操作链及真实云端端到端尚未验证。
- 影响版本与环境：本机默认阅读器 org.gnome.Papers，Flatpak 用户安装，版本 50.2、分支 inplace-save、提交 `8f269aca6d702f7d38685903ea47936222ae5af7a749ec73097a4fe24944f7de`；本地 CLI 报告 TwoDrive 0.2.10，运行 daemon 位于用户安装目录，未核实其构建提交。文件实际位于 TwoDrive FUSE 挂载。
- 关联历史故障：[PDF 保存可靠性调查](pdf-save-reliability-2026-09-09.md)。此前为上传完成修改 mtime 导致 Poppler 拒绝保存；本次明确出现 Papers 批注代码断言，未证明与旧机制有关，不能认定旧修复回归。

### 症状与影响

用户报告保存某课程 PDF 时阅读器疑似退出。日志确认阅读器于 20:27:04（Asia/Shanghai）断言退出。当前文件可解析，但不能确认崩溃前所有编辑均已保存。私人路径不录入此记录。

### 触发条件与复现

用户日常编辑保存；具体点击顺序及稳定复现条件待核实。未在原文件上重演编辑或删除批注操作。

### 根因与证据

直接退出机制已确认：Papers 在批注移除函数中到达不可达分支，主动中止。20:27:04 的脱敏日志：

```text
papers: Duplicate custom ID: palette
PapersView:ERROR:../libview/context/pps-annotations-context.c:792:pps_annotations_context_remove_annotation: code should not be reached
Bail out! PapersView:ERROR:../libview/context/pps-annotations-context.c:792:pps_annotations_context_remove_annotation: code should not be reached
```

第一条警告与后续断言的因果关系未确认。没有堆栈或稳定复现，不能从函数名推断用户一定主动删除了批注，也不能确定具体哪种批注触发。

实际服务为 `twodrive-daemon.service`，自 18:58:53 连续运行，检查时 active、NRestarts=0；20:15–20:28 日志可见正常 delta，20:26–20:29 未见 error/fail/panic/upload 匹配。最初查询的 `twodrive.service` 不是本机运行单元，其 inactive 不代表守护进程停止。未发现同期 TwoDrive 错误证据，但日志缺失错误不等价于排除所有文件系统交互因素。

目标文件 mtime 为 20:27:04，大小 2,074,594 字节；只读数据库查询状态为 cached，存在缓存路径及云端身份。这不是重新下载验证云端内容的证据。

### 解决办法与恢复操作

未修改 TwoDrive 或 Papers 程序，未重启服务，未更改原 PDF 或云端数据。当前优先调查 Papers 批注处理及 inplace-save 构建；无证据支持通过重启 TwoDrive 修复本次断言。

### 验证结果与边界

- pypdf strict=True 成功解析当前 PDF 的全部 23 页，所有页面内容流解码成功，存在 2 个 FreeText 批注；未读取或记录批注正文。
- 系统 pdftotext 全文解析到 /dev/null，退出码 0、无诊断。
- 初次 Python GI Poppler 检查因本机缺少 Poppler typelib 未执行，随后采用上述独立解析检查。
- 未进行逐页视觉检查、阅读器保存复现、旧新代码回归对比或真实 OneDrive 重新下载比对；不能保证最后一次编辑全部保存，也不能将文件可解析等同于内容完全正确。
- /var/crash 未见此次报告，未取得调用栈。

### 防复发措施与后续

下一步需在隔离副本上记录具体批注操作并捕获 Papers 调用栈，对比普通本地目录与挂载路径；确认触发条件后为对应项目增加回归测试。本次无代码修改，不新增推测性的 TwoDrive 回归测试。保留日志中的函数及行号供后续定位。

### 交付记录

仅新增本调查记录；提交及发布未产生。

### 2026-09-22 后续定位、修复与验证

用户补充：输入文字后点击右下角编辑区域的关闭按钮，文字消失；重新进入编辑模式文字出现，随后 Ctrl+S 闪退。用户明确授权修改 Papers，并要求保留此前 Ctrl+S 原地保存改动。

确认两个可独立复现的机制：

1. 在普通本地 PDF 上创建批注、切换为重新加载的文档后删除旧批注，旧 Papers 源码在与日志相同的函数第 792 行断言退出，不需要 TwoDrive。重复删除也失败。空 FreeText 失焦时原代码将控件裸指针排队到 idle 回调，控件销毁、已删除或重新输入文字时没有撤销或重检。原故障的完整 GUI 调用栈未取得，因此只确认这些机制与日志匹配，不声称已经证明用户每一步的内部调用链。
2. 使用相同 PDF 后端在独立空白测试页上绘制中文：Liberation Sans 得到 0 个非白像素，换为覆盖中文的字体后得到可见文字。GTK 编辑框具有字体回退，而 Poppler FreeText 使用单一字体，解释了编辑可见而普通阅读不可见的现象；源 PDF 只读检查发现中文 FreeText。检查过程中用户仍可能编辑原文件，早期两项与后期一项批注的读数不能据此判断丢失原因。

Papers 修复：在查询确认批注属于当前列表后才修改后端/撤销栈；取消已销毁控件的延迟删除，执行前重新核对内容和已删除状态；若当前字体缺字且 Pango 回退字体覆盖全文，则把回退字体写入批注字体属性，保持字号。新输入及已有文字进入编辑均执行检查；没有单一字体覆盖全文时保留原字体，不保证所有混合文字脚本。

验证：

- 8 个生命周期场景中正常删除/撤销/重做、空框自动清理旧新均通过；旧代码在重载后旧对象删除、重复删除、idle 前销毁、重新输入、先删除、重载这 6 个场景退出码 134，修复均为 0。
- 中文渲染/编辑/保存再打开回归旧失败、修复通过，确认英文继续保留 Liberation Sans、字号保持，保存重开中文可见；人工检查了独立生成的中文测试 PNG。
- Meson 共 3 个测试程序通过，包含上述 8 项、三轮原地 JobSave/重开、中文渲染保存。用户 PDF 仅复制到临时普通目录，再验证三轮原地保存与批注计数；原文件未被测试修改。
- 安装后通过运行时库搜索路径指定 /app/lib，再执行三轮保存重开成功；安装库 SHA-256 与测试产物一致，导出符号无增减，运行时无缺失库/未定义符号，Papers --version 正常。
- **验证边界**：未执行完整 GUI 点击/快捷键端到端、所有字体和排版验证、真实 OneDrive 上传再下载对比。当前用户已打开的旧进程未强制退出。不能保证最后一次未保存编辑可恢复。

交付与本地操作：

- 源码、原地保存补丁、修复补丁、测试及回滚包保存在 `/home/lumia/software/papers-fix`，详细说明见该目录 README.md。三个 Rust 源码改动从旧 Debug 扩展恢复，shell 二进制保持原样；仅重建 libppsview，保留原 PDF 后端。
- 安装 Flatpak 提交 `16573ca27c3e72932f9d7190c3686711c5d489d8c63f80658e6a74267e77e95e`；libppsview SHA-256 `cfaa998944d324bcd13fb2e916e3642501fe470d84d5b471cbd41d017e66eaca`。
- 恢复原本缺失的 papers-origin 配置，指向本地 fixed-repo；重新导出 desktop 原始模板，修正复制已部署导出文件造成的 @@u 重复包装，安装成功。备份包已生成，未实际回滚覆盖修复。
- 未修改 TwoDrive 程序或重启服务；无 Git 修复提交、无上游发布。后续完全退出旧 Papers 并重新打开后使用修复版本。

### GitHub 交付补记（2026-09-22）

用户授权建立独立 Papers 修复仓库，并发布“deb 安装器部署修复版 Flatpak”。源码与公开脱敏故障记录已提交至 [papers-inplace-save](https://github.com/LumiaBlack51/papers-inplace-save)，发布源码提交为 [058765d](https://github.com/LumiaBlack51/papers-inplace-save/commit/058765d2b8959c8b490787beb5764b4fb44de896)。

[Release v50.2-fix20260922-1](https://github.com/LumiaBlack51/papers-inplace-save/releases/tag/v50.2-fix20260922-1) 已发布 amd64 deb、Flatpak、完整 Papers 源码归档及 SHA256SUMS；四个远端资产 SHA-256 与本地一致。deb 为当前用户 Flatpak 安装器，非原生 GTK deb；保留 Ctrl+S 原地保存。

新增打包验证：5 项安装器检查通过；独立临时 Flatpak 安装成功从原始备份包升级到修复包并核实提交（该测试使用 --no-deps，不代表首次运行时下载验证）。解包 deb 的 Flatpak 与安装器字节匹配；当前用户已是目标提交时安装器跳过重复安装。完整公开验证与边界见 [Papers 故障记录](https://github.com/LumiaBlack51/papers-inplace-save/blob/main/doc/incident-20260922.md)。本 TwoDrive 条目仍为本地调查记录，未修改其程序或发布 TwoDrive 安装包。

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
