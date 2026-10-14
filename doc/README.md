# TwoDrive documentation · 文档

[English homepage](../README.md) · [中文首页](../README.zh-CN.md)

Start with installation, then read the everyday-use guide before trusting the mount with important files. 请先完成安装，再阅读使用与安全说明。

| Guide / 指南 | English | 简体中文 |
| --- | --- | --- |
| Install and sign in / 安装与登录 | [Get started](getting-started.md) | [开始使用](getting-started.zh-CN.md) |
| Files On-Demand, desktop controls, safety / 按需文件、桌面操作与安全 | [Everyday use](usage.md) | [日常使用](usage.zh-CN.md) |
| Release history / 版本历史 | [Changelog](../CHANGELOG.md) | [版本历史](../CHANGELOG.zh-CN.md) |

Shared references / 共用参考：[Configuration / 配置示例](../config.example.toml) · [Contributing / 开发与贡献（英文）](../CONTRIBUTING.md) · [Assets / 图片来源](assets/README.md)

## 故障记录与维护流程

从 2026-09-15 起，每次故障须记录根因、解决办法和验证边界：[故障记录](incidents.md) · [维护指导](../AGENTS.md)。

## Engineering notes

These are historical investigations/design records, not the current user manual. The project description includes intended features that may not be implemented. 以下为历史调查与设计，包含尚未实现的设想，不应当作当前功能清单。

| Topic | Record |
| --- | --- |
| Local workspace cleanup and recovery | [2026-10-07](workspace-cleanup-2026-10-07.md) |
| Nautilus responsiveness monitor | [2026-09-24](nautilus-hang-monitor-2026-09-24.md) |
| Archived original relay-lab reference | [Source and audit boundary](../TwoDrive-relay-lab/README.md) |
| Archive extraction I/O error | [2026-09-15](archive-extraction-2026-09-15.md) |
| Nautilus startup investigation | [2026-09-11](nautilus-startup-2026-09-11.md) |
| Folder-move ordering | [2026-09-11](folder-move-sync-2026-09-11.md) |
| Download/placeholder race | [2026-09-10](download-placeholder-race-2026-09-10.md) |
| PDF save reliability | [2026-09-09](pdf-save-reliability-2026-09-09.md) |
| Download cancellation | [2026-09-08](download-cancellation-2026-09-08.md) |
| Directory browsing | [2026-09-08](directory-browsing-2026-09-08.md) |
| Responsiveness and recovery | [2026-09-08](responsiveness-recovery-2026-09-08.md) |
| Background download investigation | [2026-09-08](background-download-investigation-2026-09-08.md) |
| Local-first synchronization | [2026-09-03](local-first-sync-2026-09-03.md) |
| Earlier reliability fixes | [2026-07-24](reliability-fix-2026-07-24.md) |
| Original description and goals | [Design document](twodrive_project_description.md) |

Existing investigation paths are retained so earlier release links continue to work. 原有路径不变，避免破坏历史发布说明链接。

- [Experimental device peer: implementation and audit](peer-implementation.md)
- [Peer 第一次 Windows/Linux 联调](peer-quickstart.zh-CN.md)

- [TwoDrive Development — Linux Flutter client / Linux 桌面客户端](linux-dav-desktop.md) · [2026-10-13 验证报告](linux-desktop-validation-2026-10-13.json)

- [Desktop provider boundaries and verification (2026-10-13)](provider-boundaries-2026-10-13.md)

- [Provider delivery verification (2026-10-13)](provider-validation-2026-10-13.json)

- [Third-party provider/auth/upload design specification / 第三方网盘与配套组件设计规范](provider-integration-spec.md)
- [Tray positioning / 托盘居中与 X11、Wayland 验证](tray-positioning-2026-10-14.md)
- [Storage connections / 存储连接与设备发现](storage-connections-dev.md)
- [Live OneDrive desktop verification / 真实 OneDrive 脱敏验证](onedrive-desktop-validation-2026-10-14.json)
- [Google Drive Development setup / 接入与验证](google-drive-dev.md)
