# Agent Taskboard

[![最新版本](https://img.shields.io/github/v/release/youjiaxing/agent-taskboard?display_name=tag&label=release)](https://github.com/youjiaxing/agent-taskboard/releases)
[![许可证：MIT](https://img.shields.io/github/license/youjiaxing/agent-taskboard)](./LICENSE)

Agent Taskboard 是一个**本地优先的桌面效率工具**：它把多个工作项目中的 GitHub Issue、依赖关系和执行状态集中起来，并把 Issue 交给本机安装的编码 Agent 命令行工具执行。

项目名称是 **Agent Taskboard**，桌面应用名称为 **Agent Taskboard**。当前版本为 `v0.1.0`，可从 [GitHub Releases](https://github.com/youjiaxing/agent-taskboard/releases) 下载。

> 项目仍处于早期阶段。数据保存在本机，项目路径和 Agent CLI 登录状态也依赖当前电脑的环境。使用前请先阅读下方的限制和安全提示。

## 目录

- [功能](#功能)
- [工作方式](#工作方式)
- [下载与安装](#下载与安装)
- [从源码运行](#从源码运行)
- [项目结构](#项目结构)
- [配置、凭据与数据](#配置凭据与数据)
- [更新与发布](#更新与发布)
- [当前范围与限制](#当前范围与限制)
- [参与贡献](#参与贡献)
- [许可证](#许可证)

## 功能

- **Issue 看板**：查看 Frontier、阻塞中的 Issue、进行中的 Issue 和最近完成的 Issue。
- **依赖与父子关系**：区分阻塞关系和父子归属关系，支持依赖图查看。
- **GitHub Issue 集成**：从 GitHub 读取 Issue、标签、认领状态和依赖信息，并保留最近一次成功读取的数据供离线查看。
- **本机 Agent 执行**：在真实的 Embedded Terminal 中运行官方交互式 CLI，而不是在应用里模拟聊天窗口。
- **多种 Agent Adapter**：内置支持 Grok Build、Codex、Claude Code 和 Antigravity CLI；应用会探测本机是否安装对应命令。
- **Run 管理**：启动、停止、继续和查看 Run，支持绑定 Issue 或启动不绑定 Issue 的 Run。
- **隔离执行目录**：在 Agent CLI 声明支持时，可使用基于 Git worktree 的独立工作目录。
- **改动与用量观察**：查看 Run 的本地改动、Host 用量和单次 Run 的 token/速率遥测。
- **自动推进**：可选地在当前 Issue 正常结束并经过短暂确认等待后继续处理下一张 Issue；默认关闭。
- **多种 Client**：桌面应用和浏览器都可以作为 Client 连接 Host；手机只作为浏览器 Client。
- **中英文界面与主题**：桌面应用和浏览器 Client 支持简体中文、英语，以及多套界面主题。

## 工作方式

Agent Taskboard 分为 Host 和 Client 两部分：

- **Host**：运行在某台电脑上的常驻核心，负责访问 Issue Tracker、启动 Agent CLI、管理 PTY 和保存 Host 数据。
- **Client**：连接 Host 的界面。桌面应用内置一个 Client，浏览器也可以作为 Client；一个 Client 可以连接本机 Host 或通过配对连接另一台电脑上的 Host。
- **Project**：Host 上登记的一个本地项目目录，以及该项目对应的 Issue Tracker 配置。
- **Run**：一次可观察的 Agent CLI 执行会话。Run 的生命周期与 GitHub Issue 的 open/closed 状态相互独立。

桌面应用启动 Host 时，本机浏览器入口固定为：

```text
http://127.0.0.1:10529/
```

桌面应用、浏览器和未来的远程 Client 走同一套 Host 协议。远程连接使用用户自己的局域网、VPN 或 Tailscale 网络；项目不提供云端中继或产品账号。

更完整的领域定义和架构决策见：

- [`CONTEXT.md`](./CONTEXT.md)：领域词汇和产品边界。
- [`docs/adr/0006-host-resident-pairing-remote-client.md`](./docs/adr/0006-host-resident-pairing-remote-client.md)：Host 常驻、配对和远程 Client。
- [`docs/adr/0007-tauri2-desktop-shell.md`](./docs/adr/0007-tauri2-desktop-shell.md)：Tauri 2 桌面壳与 Host 内核的职责边界。
- [`docs/adr/0012-settings-and-data-location.md`](./docs/adr/0012-settings-and-data-location.md)：设置、凭据和本地数据的位置。

## 下载与安装

正式安装包发布在 [GitHub Releases](https://github.com/youjiaxing/agent-taskboard/releases)。当前 v1 交付以下安装包：

| 系统 | 安装包 | 适用设备 |
| --- | --- | --- |
| macOS | `.dmg` | Apple Silicon（`aarch64`）和 Intel（`x86_64`）各一份 |
| Windows | NSIS `setup.exe` | `x86_64`，Windows 10 版本 1809 或更高版本 |

### macOS

1. 根据芯片架构下载对应的 `.dmg`。
2. 打开磁盘映像，将 **Agent Taskboard** 拖入 `Applications`。
3. 首次启动时，由于安装包采用 ad-hoc 签名且没有苹果公证，macOS 可能显示安全提醒；请在确认来源可信后通过系统提供的“打开”操作启动。

### Windows

1. 下载 Windows x64 的 `-setup.exe`。
2. 运行安装程序并按向导完成安装。
3. 安装程序要求 Windows 10 版本 1809 或更高版本。安装包没有 Windows 代码签名，首次启动时可能显示 SmartScreen 提示；请在确认来源可信后再继续。

项目不发布 Linux 桌面安装包。Linux 用户可以运行 Host 或使用浏览器 Client，但 Linux 桌面包不属于当前 v1 的正式交付范围。

## 从源码运行

### 开发环境

- Node.js 22
- Rust stable toolchain
- Git
- Tauri 2 对应的系统依赖：请先阅读 [Tauri 官方环境要求](https://v2.tauri.app/start/prerequisites/)
- 至少安装一个受支持的 Agent CLI；项目不会替用户安装或登录 Agent

当前内置 Agent CLI 及其探测命令如下：

| Agent | 默认探测命令 |
| --- | --- |
| Grok Build | `grok` |
| Codex | `codex` |
| Claude Code | `claude` |
| Antigravity CLI | `agy` |

### 安装依赖并启动开发版

```sh
git clone https://github.com/youjiaxing/agent-taskboard.git
cd agent-taskboard
npm ci --prefix apps/desktop
npm run dev
```

`npm run dev` 会启动 Tauri 桌面应用和本地 Vite 开发服务。只运行前端构建或直接调用 Tauri CLI 时，也可以使用：

```sh
npm --prefix apps/desktop run build
npm --prefix apps/desktop run tauri dev
```

### 构建、测试和发布校验

```sh
# 构建桌面安装包
npm run build

# 运行 Rust workspace 测试
npm test

# 检查所有 workspace target
cargo check --workspace --all-targets

# 校验版本号、打包目标和更新器配置
npm --prefix apps/desktop run verify:release
```

发布工作流还会安装 Playwright Chromium、构建前端并运行 `cargo test`。Windows 安装包另有 [安装冒烟工作流](./.github/workflows/windows-package-smoke.yml)。

## 项目结构

```text
crates/host-kernel/       Host 核心：Issue、Project、Run、Agent 和 Tracker
apps/desktop/src/         TypeScript/Vite Client 界面
apps/desktop/src-tauri/   Tauri 2 桌面壳和跨平台打包配置
apps/desktop/e2e/         浏览器端到端测试
.github/workflows/        测试、发布和 Windows 安装验证
CONTEXT.md                领域词汇和产品边界
docs/adr/                 架构决策记录
```

## 配置、凭据与数据

### GitHub 凭据

项目没有产品账号、云端同步或统一登录。GitHub Issue 访问凭据按以下优先级解析：

1. `AGENT_TASKBOARD_GITHUB_TOKEN`
2. Host 秘密文件中的 GitHub PAT
3. 本机 `gh auth token`
4. `GH_TOKEN` 或 `GITHUB_TOKEN`

日常使用推荐先安装并登录 [GitHub CLI](https://cli.github.com/)，然后执行：

```sh
gh auth login
```

Agent CLI 的账号、API key 和登录状态由各 Agent 自己管理，Agent Taskboard 不会把它们与 GitHub 凭据合并。

### 本地数据

桌面应用使用 Tauri 的本地应用数据目录，并将 Host 数据与桌面 Client 设置分开保存：

- macOS：`~/Library/Application Support/com.youjiaxing.agent-taskboard/`
- Windows：`%LOCALAPPDATA%\\com.youjiaxing.agent-taskboard\\`
- Host 数据：`host/`
- 桌面 Client 设置：`desktop-client/`
- 日志：系统应用日志目录下的对应目录

项目 v1 不提供备份/导出按钮。迁移前请退出应用，并按 [`docs/adr/0012-settings-and-data-location.md`](./docs/adr/0012-settings-and-data-location.md) 复制对应目录；其中 Project 路径是本机绝对路径，换电脑后可能需要重新配置。

远程配对依赖用户自己的网络。请不要把 Host 的端口直接暴露到公网，也不要在不信任的电脑上保存配对信息。

## 更新与发布

桌面应用通过 GitHub Releases 的 `latest.json` 使用 Tauri updater 检查更新，并使用更新器公钥验签：

- 发现更新后只提示，不会后台静默替换。
- 只有用户确认后才会下载并安装。
- 有活跃 Run 时禁止安装更新，避免中断 Agent 执行。
- 安装完成后按更新前的运行模式重新启动。
- 更新失败或跨大版本更新不会替换 Host 数据和 Client 设置。
- 浏览器 Client 不提供替另一台 Host 更换安装包的操作。

发布约束和平台资产说明见 [`docs/adr/0013-packaging-and-auto-update.md`](./docs/adr/0013-packaging-and-auto-update.md)。维护者发布新版本时，向 `v*` 标签推送即可触发 GitHub Actions 发布工作流；更新私钥只应保存在 CI secret 中。

## 当前范围与限制

当前 v1 明确包含：

- GitHub 作为 Issue Tracker 的首个实现。
- macOS Apple Silicon、macOS Intel 和 Windows x64 桌面安装包。
- Windows 10 版本 1809 及以上版本支持。
- 本机优先的数据存储和用户自有网络下的远程配对。

当前 v1 明确不包含：

- Linux 桌面安装包。
- App Store、Microsoft Store、苹果公证或 Windows 代码签名。
- 产品账号、云端中继、云同步和集中式凭据管理。
- 默认开启的无人值守自动推进。
- 用 Agent Taskboard 替代 Agent 官方 CLI 的聊天界面。

## 参与贡献

项目使用 [GitHub Issues](https://github.com/youjiaxing/agent-taskboard/issues) 管理需求、规格和缺陷。提交较大的代码改动前，请先创建 Issue 或在相关 Issue 中讨论范围和方案；当前项目以维护者确认后的 Issue 为主要工作入口，外部 PR 请先取得维护者确认。

提交代码时请至少确保：

- 变更范围与 Issue 或现有架构决策一致。
- Rust 测试和必要的前端构建检查通过。
- 不提交凭据、配对令牌、构建产物或本机数据。
- 涉及界面、路由或状态的变更，补充对应的浏览器或桌面交互验证。

仓库内的 Issue 操作约定见 [`docs/agents/issue-tracker.md`](./docs/agents/issue-tracker.md)。

## 许可证

本项目以 [MIT License](./LICENSE) 发布。版权归 **YouJiaXing** 所有，版权年份为 2026。

项目依赖的第三方库仍受各自许可证约束；使用、再分发或修改时请同时遵守这些依赖的许可条款。
