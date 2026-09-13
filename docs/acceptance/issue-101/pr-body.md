## Summary

- 将 v1 桌面版本提升到 0.1.1，完成 macOS Apple Silicon/Intel 与 Windows x64 的 Release matrix、安装启动和 updater 资产闭环。
- 接入 Tauri 原生通知、Codex 等待态兜底、真实窗口/托盘/启动项验证，并稳定浏览器验收竞态。
- [完整 156 条矩阵](../v1.md) 与 [真实桌面证据](README.md) 已更新。

## Scope Host kernel/Client

### Host kernel

- 每个 Run 使用隔离 completion hook sink；覆盖 PermissionRequest/UserPromptSubmit 等待生命周期和 Codex PTY Action Required 标题兜底。
- 保持 Host/Client 数据树隔离、配对 token 撤销/隔离、端口占用解释和 5 MiB × 5 日志轮转。

### Client

- 桌面壳使用 Tauri notification plugin 请求权限、发送原生通知并跳回 Host/Project/Issue/Run；浏览器保留 Web Notification 回退，桌面通知与声音独立。
- Release workflow 在发布后修正 latest.json 的三平台公开下载 URL；Windows smoke 覆盖 NSIS 静默安装、10529、窗口关闭/托盘/Quit Host 与 HKCU Run。

## Explicitly out of scope

- 不加入苹果公证、Mac App Store、Microsoft Store 或商业代码签名。
- 不发布 Linux 桌面包。
- 不自动关闭 #45；其 proposer 明确接受仍是独立收尾门。

Closes #101
