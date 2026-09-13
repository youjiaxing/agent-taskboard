# Issue #101：v1 真实桌面与发布验收证据

本目录记录 Issue #101 的可复核证据；验收矩阵索引为 [`../v1.md`](../v1.md)，实现 PR 为 [#125](https://github.com/youjiaxing/agent-taskboard/pull/125)。证据按层级区分：Host 内核/壳层自动化、Release 产物、浏览器与移动降级、开发安装包，以及真实 macOS/Windows 系统行为。

## 当前结论

- v0.1.1 版本号在桌面 package、Rust crate、Tauri bundle 三处一致。
- 本机 macOS Apple Silicon 已从 `/Applications/Agent Taskboard.app` 的 v0.1.0 通过 updater 确认安装到 v0.1.1；更新后 Host 仍监听 `10529`，看板和两个 Project 均恢复。
- 更新弹窗先展示版本说明，只有点击“下载并安装”才开始下载；有活跃 Run 时由 `update_install_requires_every_run_to_end` 门禁止安装。
- Host 与 Client 数据树独立：更新只替换应用包，不替换 `host/`、`desktop-client/`。更新前后以 SHA-256 记录 secrets 与 Client 设置；Host 重新启动后仍可服务。
- 真实 Windows 证据由 Release matrix 的 NSIS 安装、静默安装、原生启动、托盘常驻和 HKCU Run 验收提供；真实 Intel macOS 由 `macos-15-intel` matrix 提供。

## 真实桌面证据

| 证据 | 内容 |
|---|---|
| [`02-v0.1.0-dmg-cold-launch.png`](real/02-v0.1.0-dmg-cold-launch.png) | 从 Apple Silicon DMG 复制安装后冷启动；原生窗口与 10529 Host 同时可用 |
| [`03-window-close-host-alive.png`](real/03-window-close-host-alive.png) | 关闭窗口路径的系统屏幕记录；Host 存活与窗口/托盘路径在 Release smoke 中再次核对 |
| [`04-v0.1.1-update-found.png`](real/04-v0.1.1-update-found.png) | v0.1.0 设置页真实发现 v0.1.1，弹窗包含版本说明及确认按钮 |
| [`05-active-run-quit-gate.png`](real/05-active-run-quit-gate.png) | 存在活跃 Run 时选择退出 Host，真实弹出“返回/停掉全部”安全门 |
| [`06-v0.1.1-updated.png`](real/06-v0.1.1-updated.png) | 点击确认后 updater 完成替换并重启到 v0.1.1；看板、Project、Host 均恢复 |

`00-dev-before-release.png` 与 `01-dev-settings.png` 是开发安装基线，仅用于对照，不代替 Release 产物证据。

## 自动化与发布证据

```sh
node scripts/verify-v1-acceptance.mjs
cargo test --workspace --all-targets -- --test-threads=1
npm --prefix apps/desktop run verify:release
npm --prefix apps/desktop run build
cargo test --test host_kernel pairing:: -- --test-threads=1 --nocapture
```

`verify-v1-acceptance.mjs` 要求 `docs/acceptance/v1/*.md` 恰好包含 #1–#156 各一次，并且每条为“状态：通过”；当前输出为 `156 unique stories, all PASS`。Pairing 真实 TCP 门覆盖 offer/code、token 复用、撤销即时失效、Host/Project 隔离与远端 Host 存活（10 tests passed）。

Release workflow 的 matrix 是 macOS Apple Silicon、macOS Intel、Windows x64；Windows 步骤执行 `npm ci`、NSIS `/S` 安装、原生启动、10529 readiness、关闭窗口保留 Host、托盘重开、Quit Host 退出及开关 Start at login 后核对 HKCU Run。最后的 `finalize-updater` 将 `latest.json` 的三平台 URL 固定为公开 Release 下载地址，避免 API asset URL 被 updater 拒绝。

日志由 `tauri-plugin-log` 配置为单文件 5 MiB、保留 5 个轮转文件；本机日志目录中已观察到 5 个约 5 MiB 的历史文件。端口被其他进程占用时，第二个原生实例显示“端口 10529 已被占用；桌面窗口可以继续用”，而现有 Host 保持服务。

## 结果边界

本票完成 #101 要求的实现与真实桌面/发布验收后，PR #125 使用 `Closes #101` 合并并关闭 #101。#45 的 156 条技术故事矩阵可全部 PASS，但其总票仍需 proposer 明确接受；在获得该明确接受前，#45 保持 OPEN，不以本票代签。
