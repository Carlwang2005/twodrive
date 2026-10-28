# 发给家庭网络笔记本上编码助手的任务

可把以下内容复制给另一台 Windows 笔记本上的助手。助手只能操作自己所在设备，不假定有权控制另一台电脑。

---

请协助我完成 TwoDrive 双 Windows、低流量跨 NAT 实验。你所在的高性能笔记本叫 B，使用家庭网络；另一台 Windows 电脑叫 A，使用手机蜂窝热点。我一个人可以在两台设备间配合操作。

请先在 B 的家庭网络上执行浅克隆，不覆盖现有目录：

```powershell
git -c core.longpaths=true clone --depth 1 --single-branch --branch codex/windows-two-device-handoff-20261028 https://github.com/Carlwang2005/twodrive.git C:\TwoDriveLab\repo
```

进入仓库后阅读 AGENTS.md，并严格按 `doc/windows-two-device-handoff.zh-CN.md` 操作；先报告当前提交、环境和准备情况。源码基于上游 dev，实验后端为 experiments/twodrive-dev，桌面为 apps/dav。不要使用旧 Full/Lite 测试流程。

所有工具链、依赖、源码和构建下载都放在 B 的家庭网络上完成。先构建 CLI 并跑独立回归；通过 U 盘把同一 exe 和必要脚本交给 A，双方核对 SHA-256。邀请和私钥不得上传 Git、公开聊天或 PR。

先指导我检查两端代理/TUN/VPN、系统代理/PAC、路由、实际出口、防火墙。原机曾存在 PulseVPN 无法确认完全退出的问题，前置条件不满足就停止并记录，不假报通过。

B 首先 serve，A connect；只共享新建可丢弃目录，服务端限制单次上传 1 MiB。热点阶段每次使用 `--small-count 0 --attempts 1 --timeout 30`，不要用 --full/--faults/--soak-seconds，不要在热点下载依赖。auto 与 relay-only 各建连五次，再交换角色；按交接文档记录每轮。

热点累计 50 MiB 暂停检查、100 MiB 停止；每轮都提醒我检查接口与手机流量。失败不要无限重试，两次失败停止该组。不要现在自动启动两小时任务或大文件传输。

现有 CLI 只打印初始路径；必须把“初始 direct-udp”“全程路径证据”“物理出口”和“relay 成功”分开。需要改进时仅在后端增加脱敏、有限频率的路径遥测及独立回归，不在 Flutter 重写恢复逻辑。未实现的遥测写未测，不能宣称已经具备。双机测试实际未执行前只写准备完成。

大文件、千文件、异常组合、两小时稳定性和桌面验收安排到两端同一家庭局域网后，仍需分别保留验证边界。所有故障按 AGENTS.md 留档。

只在我的 fork 工作分支提交并交回分支/提交链接；不要合并 PR #46 或任何其他 PR，不要直接修改 LumiaBlack51/twodrive 的 dev/main。先推进 B 上的准备工作；需要 A 的操作时给我一次一个阶段的明确指令，不要假定已经远程操作了 A。
