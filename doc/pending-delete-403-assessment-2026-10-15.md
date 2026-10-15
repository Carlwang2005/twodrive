# 持续 403 删除待办：main / dev 存续与修改必要性评估

日期：2026-10-15（Asia/Shanghai）。范围：代码审查、公开脱敏历史证据和隔离模拟验证；没有生产修复或真实账号检查。

## 结论

附件指出的两项工程缺口在 main 和 dev 的普通同步删除链路中都仍存在：失败诊断不足，持续拒绝的待办没有持久化逐项退避。它们值得改进，但不能把这些改进说成已经解决原对象的 403。

原对象为什么被拒绝仍未确认，本轮也没有核实该生产任务今天是否还存在。对象权限/服务端限制、共享来源、历史身份错配仍是候选原因，无法仅凭公开材料可靠排序为已证实根因。

建议先补安全诊断，再做有界、可恢复的逐项退避；账号/Drive/Item 模型和删除目标的改变应等待对象证据。当前没有必须立即发布紧急修复的证据，也不宜把长期拒绝当作正常瞬时网络失败而无限期维持现状。

## 锁定版本与历史区分

本轮执行 `git fetch origin main dev` 后核实：

| 分支 | 提交 | 共享 core/backend/fs 版本 |
| --- | --- | --- |
| main | `e62942798cc6ab4b221a31d6d766b16dd4be8edf` | 0.2.13 |
| dev | `ecb05392324275f3e0c4317f6e5937d3a4b52ed7` | 0.2.12 |

main 与附件审查提交相同；稳定标签 `v0.2.13-1` 解引用为 `cfa1ffaefbd5877dc281025085cdaefac1ab2164`，到本轮 main 的差异只有 CI、CONTRIBUTING、故障记录与验证 JSON，没有删除源代码变动。

已读 main 的 TD-20261014-16、12、15、[local-first 设计](local-first-sync-2026-09-03.md)、[可靠性调查](reliability-fix-2026-07-24.md)。main 的历史原文可从[锁定版本故障记录](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/doc/incidents.md)核对；当前 dev 没有完整合入这些 main 补记。

- TD-20261014-12：关闭后的 Dirty/Uploading 文件被本地 unlink 拒绝。main 已有 `4d4f7ae` 及后续修复，dev 仍保留 `cannot delete a file while it is changing` 条件。这是另一项尚未同步的修复，不能与云端 403 混为一谈。本轮没有重新验证其真实 FUSE 场景。
- TD-20261014-16：升级前旧任务持续 403。历史记录的 17 次错误、新测试对象成功、剩余任务属于升级前集合，是此前记录的证据，不是本轮重新测得。
- [验证 JSON](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/doc/unlink-live-validation-2026-10-14.json) 的 `remaining_pending_deletes=0` 属于隔离探针，不能证明生产旧任务已完成。

## 分支与调用链判断

| 项目 | main | dev | 判断 |
| --- | --- | --- | --- |
| 普通 Graph DELETE 的错误信息 | `HttpFailure` 只保留 status / retry_after | 同一文件，同一 Git blob | 两分支都缺少操作来源、允许的细分码和请求关联信息 |
| 持续 403 的挂载待办调度 | 失败保留；恢复扫描重排 | 失败保留；恢复扫描重排 | 两分支都没有 attempt_count / next_retry / blocked_reason |
| 普通删除目标 | `/me/drive/items/{id}` | 同样路由 | 路由本身受官方支持，不能仅据此判错 |
| 普通队列的账号/Drive/remoteItem 身份 | 没有逐项保存；新增上传声明字段不补足这些身份 | 没有逐项保存 | 存在模型边界，但尚未证明导致本案 |
| dev 桌面额外保护 | 无对应独立 provider 桌面实现 | 连接/恢复核对账号和 Drive，挂载状态检查 binding；浅层列表过滤 remoteItem/package | 附件关于普通队列的判断不能扩大成“dev 完全没有身份保护” |

关键代码：

- [main 普通 HTTP helper](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/crates/twodrive-backend/src/onedrive/http.rs#L89)：Graph DELETE 真正返回 403 时立即报错；404 幂等成功；429/5xx 才进入 HTTP 重试。main/dev 的该文件和普通 Graph 模型分别完全相同。
- [main 删除恢复](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/crates/twodrive-fs/src/recovery.rs#L253)：`backend.delete(...)` 失败提前返回，后面的缓存清理、移除待办不会执行。保留待办是正确的持久性行为。
- [main 恢复扫描](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/crates/twodrive-fs/src/recovery.rs#L407)无到期筛选；[挂载循环](https://github.com/LumiaBlack51/twodrive/blob/e62942798cc6ab4b221a31d6d766b16dd4be8edf/crates/twodrive-fs/src/mount.rs#L77)扫描、同步后等待 60 秒。队列去重只覆盖排队/运行期间，不形成持久退避；17 次不能换算成精确 17 分钟。
- 普通 DELETE 构建请求前会 `access_token()`；token 刷新使用同一个 HTTP helper，失败可在添加 OAuth label 前传播为相同 `HTTP 403`。因此历史聚合文字不足以独立证明每次都是 Graph DELETE 响应。
- main 的 `is_pending_delete` 还保护同路径上传；持续阻塞可能使同名新文件的云端上传继续等待。它属于防止旧任务伤及新对象的必要排序约束，不能为消除报错而绕过。原生产任务是否实际阻塞了新文件，本轮未检查。
- 普通恢复日志仍有 path / ID，reqwest 错误也可能包含 URL；“没有保存正文”不等于原始日志适合公开。

dev 需要区分两种入口：

1. Linux 可写挂载经 `experiments/twodrive-dev/src/mount.rs` 调用共享 `mount_backend`，会继承上述持久队列和周期恢复缺口。
2. 桌面文件页的 `delete` 经 `desktop.rs` → `Backend::delete_path` → `OneDrive::delete` → `GraphBackend::delete` 同步返回结果。这条直接删除调用没有加入 FUSE 的 pending_deletes；可以确认它继承诊断不足，不能说它也自行每分钟重排任务。

dev 的 binding 和浅层过滤降低部分错配风险，但不会把普通队列改造成逐对象 Drive 绑定，也未证明普通 list_all/delta/按路径查找覆盖所有 remoteItem 语义。

## 修改必要性与风险

| 候选改动 | 建议 | 收益与边界 |
| --- | --- | --- |
| 安全诊断与来源分类 | 优先安排，范围可控制 | 区分 token.refresh / driveitem.delete；限定大小/深度解析白名单错误码和校验后的关联字段，避免正文、私人路径、真实 ID、token、会话 URL 泄漏。可借鉴 control 通道模式，但不能认为它已覆盖普通同步 |
| 持久化逐任务退避和受阻状态 | 有必要，建议随后做 | 降低无效请求、日志噪声、任务规模增大时的 worker 占用；保留任务与必要缓存，权限/认证变化或用户重试后可重新评估。需要 schema、调度、状态显示、重启和同路径屏障联合设计，风险高于纯诊断 |
| 账号/Drive/Item 绑定、remoteItem 删除语义 | 先取证再决定范围 | 原旧任务可能是权限限制而非寻址错误；历史身份缺失不能凭空补齐。删除快捷方式与删除共享源对象不能互换 |
| 盲目扩权、按旧路径兜底、换源 Drive 删除、清空待办 | 当前无依据 | 可能隐藏未完成操作或误删同名/共享对象，不能算修复 |

官方列明 `/me/drive/items/{item-id}` 是合法删除路由，委托个人/工作账号最低权限为 Files.ReadWrite；不支持仅凭本案直接改成 Files.ReadWrite.All。403 也可能包含 insufficient_claims，需要细分信息。[删除 API](https://learn.microsoft.com/en-us/graph/api/driveitem-delete?view=graph-rest-1.0)、[错误模型](https://learn.microsoft.com/en-us/graph/errors)。remoteItem 表示另一 Drive 来源的对象，应保留外层与来源身份的区别。[remoteItem 模型](https://learn.microsoft.com/en-us/graph/api/resources/remoteitem?view=graph-rest-1.0)。这些公开规范不证明本案的实际权限或对象类型。

## 本轮实际验证

从两分支锁定提交各导出独立临时源码副本，只追加评估测试；生产实现原字节不改，仓库内未增加测试/实现补丁。测试使用 loopback HTTP、合成凭据、临时 SQLite，无真实 OneDrive。

每个分支分别运行以下 5 项，全部通过（backend 4 项、fs 1 项）：

1. `assessment_403_stops_http_retry_and_discards_diagnostic_fields`：返回带 accessDenied、嵌套码和关联 header 的模拟 403；只构造/发送一次 DELETE，错误最终仅为 HTTP 403，细分信息未保留。
2. `assessment_404_stays_idempotent_and_long_429_is_not_slept`：404 返回 None；429 + Retry-After 120 保留等待时长并退出该调用，不短等后立即重试。
3. `assessment_transient_503_retries_then_succeeds`：503、503、204 的模拟序列恰好三次请求并成功。
4. `assessment_20261015_refresh_403_is_reported_as_delete_failure_without_source`：实际调用 `GraphBackend::delete`，合成过期凭据的 refresh endpoint 三次返回 403；没有进入 Graph DELETE 发送，最终错误仍为无来源的 HTTP 403。证明附件指出的诊断歧义在两分支可复现；不证明原生产 17 次源于刷新。
5. `assessment_denied_task_survives_restart_and_each_scan_requeues_without_backoff`：连续三轮重新打开/初始化临时 DB，每次扫描立即排队并调用模拟失败 backend；每次保留同一任务及路径屏障，拒绝解除后正常清理。该测试没有实际等待 60 秒；周期由挂载代码审查确认，也没有模拟 HTTP 客户端与队列贯通的真实端到端调用。

测试根目录：`/tmp/twodrive-403-assessment-20261015-veqw2qtz`；临时副本/探针可能被系统清理，不是已纳入仓库的长期回归。命令在各副本执行：

```bash
cargo test --locked --offline -p twodrive-backend -p twodrive-fs assessment_20261015 -- --test-threads=1
```

dev 测试复用 main 副本的 CARGO_TARGET_DIR，仅共享构建产物；两套路径依赖分别从各自副本编译，版本分别为 0.2.13 / 0.2.12。上述通过表示现有行为已被探针确认，不表示缺陷已修复。没有修复后版本，未做旧失败/新通过对照；正式修改时应把安全诊断、退避持久性及恢复约束转为仓库回归测试。

未运行全工作区测试、真实 FUSE 或 Windows 原生测试；未读取私人数据库/凭据、启动挂载/recover/doctor、重放生产 DELETE、改权限、改服务、安装或发布。

## 后续决策门槛

若后续处理原对象，先以沿用既有授权的有限只读证据确认账号/默认 Drive、队列指向、对象类别与共享来源；读取成功也不证明可删除。若仍不能定位，优先利用增强诊断观察正常既有后台尝试的来源和安全错误字段。是否执行真实检查需由后续任务确定，本轮未执行附件内建议步骤。

本轮确认修复的机制：无。已确认仍存在的工程缺口：普通删除诊断与持久退避不足。原对象拒绝原因、当前生产队列状态及任何改进能否解除该对象的 403：尚未验证。

## 同日实施补记

用户在评估完成后授权实施并同步 main/dev 与 Release。安全诊断、持久化逐任务退避、受阻状态和手动重试已进入修复分支；上文“未修改/无修复版本”描述评估阶段。具体实施、旧实现失败对照和最终交付证据继续追加到 [TD-20261015-01](incidents.md#td-20261015-01持续-403-删除待办的-maindev-存续与修改必要性评估)。未扩大权限或改变删除目标，原对象服务端拒绝根因仍未确认。

交付完成补记：main 的 0.2.14-1 与 dev 的 Development 0.1.3 已发布；安全诊断、持久退避及状态/手动重试的验证和边界见 [故障记录](incidents.md)。原对象的服务端拒绝原因仍未确认，未修改或检查生产待办。
