# Issue #111 Local Markdown tracker 能力与元数据验收

验收日期：2026-08-29。

## 结论

自动化结果：**PASS**。Local Markdown 的读取、状态/Type/Assignee 语义、父子关系、Dependency、认领/释放、创建、编辑、评论、重启后重读和非法 metadata fail-closed 均有 HostKernel 回归证据。

真实 Tauri 桌面壳结果：**BLOCKED**。本轮未启动源码 Tauri bundle 执行真实文件目录验收，因此不把 RPC、fixture 或单元测试结果升级为桌面壳 PASS。

## 正常链路

- 读取 `Status`、`Type`、`Assignee`，并将状态/类型带入看板卡片与 Issue 详情。
- 认领写回 `Status: claimed`，释放恢复 `Status: ready-for-agent`；关闭写 `Status: resolved`。
- 创建、编辑、追加 `## Comments`、设置 `Part of` 与 `Blocked by` 后，重新读取仍保持一致。
- 依赖图只使用解析后的 Dependency 边；已完成 blocker 不继续阻塞 Frontier。

证据：`crates/host-kernel/tests/projects.rs` 中的 `local_markdown_*` 场景，以及 `cargo test --workspace --all-targets`。

## 非法 / 缺失 metadata 链路

`Status: done`、缺失 blocker 或非法引用会返回 `RefreshStatus::Incomplete`，隐藏 Frontier 和依赖图；不会将不确定数据当成真实状态，也不会显示 GitHub credential 修复文案。`Status`/`Type`/`Closed` 冲突、父 Issue 缺失、自依赖和 Dependency cycle 同样 fail-closed。

## 证据边界

- 自动化使用真实 HostKernel、生产 Client build 和临时本地 Markdown 文件；不调用 GitHub 写接口。
- 直接 RPC、fixture、单元测试只证明逻辑与协议，不计真实 Tauri 桌面体验 PASS。
- 桌面壳、真实用户目录与重新打开 App 的人工验收仍需在可用环境中执行。

## 验证命令

```sh
npm --prefix apps/desktop run build
cargo test -p host-kernel --test projects -- --nocapture
cargo test --workspace --all-targets
```
