# WKD：本账号 TwoDrive 实验材料

本目录集中存放 GitHub 账号 **Carlwang2005** 在本次 TwoDrive 实验中新增的脚本、报告、日志与交接文档。
整理日期：2026-10-29。历史证据未重跑或改写；整理目录不表示实验已全部完成。

## 从哪里开始

- **另一台笔记本的助手**：复制 [交接指令](docs/windows-two-device-agent-prompt.zh-CN.md)。
- **双机操作**：阅读 [Windows 低流量实验手册](docs/windows-two-device-handoff.zh-CN.md)。本机 A 用热点，笔记本 B 用家庭网络；下载与构建放在 B。
- **已有结果**：阅读 [测试报告](validation/udp-2026-10-25/README.zh-CN.md)及 [测试矩阵](validation/udp-2026-10-25/matrix.csv)。
- **已知问题**：阅读 [实验故障记录](incidents.md)。首次大文件超时根因未确认；干净网络、双机跨 NAT、全程路径、两小时及桌面验收尚未全部完成。
- **脚本回归**：从仓库根目录运行 `python wkd/scripts/test-reliability.py`。

## 目录

```text
wkd/
  README.md
  incidents.md
  docs/
    windows-two-device-agent-prompt.zh-CN.md
    windows-two-device-handoff.zh-CN.md
  scripts/
    reliability.py
    test-reliability.py
  validation/
    udp-2026-10-25/
      README.zh-CN.md
      matrix.csv
      test-log.txt
      final-runner-check.json
      local-corpus-first.json
      local-corpus-retry.json
      local-faults.json
      local-lan.json
      local-large-retry.json
      local-public.json
      local-relay.json
```

## 本账号提交与文件清单

以两次已提交的实验工作为范围：

| 原提交 | 内容 | 原新增文件数 |
| --- | --- | ---: |
| `4a7b77f3027e3dfd85d3089c2b1adc16bdeb5f7e` | 测试工具、回归、脱敏证据和故障记录 | 13 |
| `9965f67b3cf352a3685b4e2a6f28ee35c27d0d9e` | 双 Windows 低流量交接 | 2 |

这 15 个独立新增文件已全部归入本目录，另增加本索引并将本账号的完整故障条目集中到 incidents.md。
历史提交保留原路径；当前分支以本目录为准。

| 原路径 | 当前位置 | 内容 |
| --- | --- | --- |
| `doc/windows-two-device-agent-prompt.zh-CN.md` | [docs/windows-two-device-agent-prompt.zh-CN.md](docs/windows-two-device-agent-prompt.zh-CN.md) | 发给另一台助手的指令 |
| `doc/windows-two-device-handoff.zh-CN.md` | [docs/windows-two-device-handoff.zh-CN.md](docs/windows-two-device-handoff.zh-CN.md) | 克隆、构建、配对、流量预算与实验步骤 |
| `experiments/twodrive-dev/scripts/reliability.py` | [scripts/reliability.py](scripts/reliability.py) | 可丢弃文件的 CLI/网关测试工具 |
| `experiments/twodrive-dev/scripts/test-reliability.py` | [scripts/test-reliability.py](scripts/test-reliability.py) | 五项工具回归 |
| `doc/validation/udp-2026-10-25/README.zh-CN.md` | [validation/udp-2026-10-25/README.zh-CN.md](validation/udp-2026-10-25/README.zh-CN.md) | 版本、结果和限制 |
| `doc/validation/udp-2026-10-25/matrix.csv` | [validation/udp-2026-10-25/matrix.csv](validation/udp-2026-10-25/matrix.csv) | 可机读测试矩阵 |
| `doc/validation/udp-2026-10-25/test-log.txt` | [validation/udp-2026-10-25/test-log.txt](validation/udp-2026-10-25/test-log.txt) | 脱敏日志摘录 |
| `doc/validation/udp-2026-10-25/*.json`（8 份） | [validation/udp-2026-10-25/](validation/udp-2026-10-25/) | 连接、逐文件校验、失败与资源采样原始结果 |
| `doc/incidents.md` 中 TD-20261025-01、TD-20261026-01 的详细内容 | [incidents.md](incidents.md) | 保留完整调查及验证边界 |

两处仓库集成文件仍需放在标准位置：

- [.github/workflows/dev-webdav-peer.yml](../.github/workflows/dev-webdav-peer.yml)：原有 CI 工作流；仅调整测试入口及 wkd 路径触发条件。默认工作目录为 experiments/twodrive-dev，因此使用 `../../wkd/scripts/test-reliability.py`。
- [doc/incidents.md](../doc/incidents.md)：按 AGENTS.md 保留两条中央故障摘要，详细内容链接到本目录。其他作者的历史记录未移动。

未把 exe、Cargo target、旧补丁导出、下载 ZIP、邀请、配对状态或任何个人凭据加入本目录。既有被 Git 忽略的本地产物仍留原位。

## 使用约束

热点阶段只用小语料：`--small-count 0 --attempts 1`；不用 --full/--faults/--soak-seconds。达到 50 MiB 暂停、100 MiB 停止，协议及背景流量另计。
完整命令、两端角色及路径证据缺口以操作手册为准。Python 脚本已迁移，原路径不再作为入口。
整理提交只推个人 fork 的当前工作分支，不合并 PR，不重写历史分支。
