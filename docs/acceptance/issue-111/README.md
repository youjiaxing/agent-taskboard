# Issue #111 Local Markdown tracker 能力与元数据验收

验收日期：2026-08-30

## 结论

Issue #111：**PASS**。

- 自动化：HostKernel debug/release workspace tests、桌面生产 build、Issue #111 Browser E2E 和全量 board/project E2E 均通过。
- 真实桌面体验：使用 Release Tauri `.app` 与实际 `.scratch/*/issues/*.md` 文件，完成正常读写、重启保持、非法 metadata fail-closed，以及真实文件系统写失败后的草稿恢复与重试。
- 证据边界：RPC、fixture 和单元测试只计自动化证据；下列“真实 Release Tauri 壳”场景才计桌面体验 PASS。

真实 bundle：

`target/release/bundle/macos/Agent Taskboard.app`

实际 Markdown 数据源：

- `.scratch/issue111-real-e2e-release/project/.scratch/feature/issues/`
- `.scratch/issue111-real-e2e/project/.scratch/feature/issues/`

## 正常链路（真实 Release Tauri 壳）：PASS

1. 登记实际 Local Markdown Project，读取 `Status`、`Type`、`Assignee`、父 Issue 和 Dependency；看板、Issue 详情与依赖图使用同一份关系语义，父子边没有被画成 Dependency。
2. 在桌面 UI 创建 `#3 Release UI Created`，文件实际写入 Markdown 目录。
3. 编辑 `#2` 的标题与正文；文件编号前缀 `02` 保持不变。
4. 在 `#2` 追加评论，磁盘文件生成并保留 `## Comments` 内容。
5. 为 `#2` 设置和清除父 Issue；设置和清除 Dependency，磁盘中的 `Part of` / `Blocked by` 与 UI 同步。
6. 关闭 `#2` 后 UI 显示 `Status: resolved`；重开后显示 `Status: open`。
7. 退出并重新启动同一个 Release bundle，创建、编辑、评论、状态和关系写入均从实际 Markdown 文件重新读回并保持一致。

## 非法 metadata 链路（真实 Release Tauri 壳）：PASS

临时加入包含不支持值 `Status: done` 的 Markdown Issue 并刷新：

- 桌面壳显示“数据不完整”并指出 `invalid Status: done`；
- Frontier 和依赖图被隐藏，不基于不确定数据继续计算；
- 没有显示 GitHub-only credential 修复文案。

删除临时文件并再次刷新后，正常看板恢复。

自动化还覆盖以下 fail-closed 关系错误：缺失/歧义引用、自依赖、Dependency 环、父 Issue 环，以及跨 `.scratch/*/issues/` 的重复 Issue 编号。Dependency 和父子环在写入前被拒绝，不修改原 Markdown 文件。

兼容性回归还覆盖：清除旧 `Parent:` / `## Parent` 父关系格式；`Closed: true` 只兼容缺失 Status，不掩盖显式非法 Status；`Closed: false` 的 legacy Issue 可以正常关闭并迁移到 `Status: resolved`。

## 写失败与恢复链路（真实 Release Tauri 壳）：PASS

1. 将实际 Issue 目录权限改为 `0555`。
2. 在编辑器输入标题 `Release write failure draft`，正文输入 `Draft body must survive the real filesystem failure.`。
3. 保存返回真实文件系统错误 `Permission denied (os error 13)`；标题和正文草稿均留在编辑器中，没有静默成功。
4. 恢复目录权限为 `0755`，直接重试保存成功；磁盘文件随后包含同一标题和正文。

Dependency 集合写入会先验证完整候选图，再对单个 Markdown 文件执行一次 atomic temp-file + rename；无效引用或成环时不会产生半写入。

## Acceptance matrix

| Acceptance criterion | 结果 | 证据 |
| --- | --- | --- |
| 解析并展示 Status、Type、Assignee、父子、Dependency | **PASS** | 真实 Release 看板、详情、依赖图；HostKernel 回归 |
| 非法/缺失 metadata fail-closed | **PASS** | 真实 `Status: done` 场景；缺失/歧义/成环/重复编号自动化 |
| 读取、状态筛选、认领、释放、编辑、评论 | **PASS** | 真实 Release UI 与实际 Markdown 文件 |
| 创建/维护父子关系和 Dependency，重启后保持 | **PASS** | 真实 Release UI 设置/清除关系并重启重读 |
| 写失败保留输入、明确反馈、无半写入/静默成功 | **PASS** | 真实 `0555` 权限失败、草稿保留和恢复后重试；atomic-write 回归 |
| 使用真实 Tauri 壳和实际 Markdown 文件 | **PASS** | Release `.app` 与上述两个实际目录 |
| 至少一条正常和一条非法/缺失 metadata 记录 | **PASS** | 本文“正常链路”和“非法 metadata 链路” |

## 自动化验证

```sh
npm --prefix apps/desktop run build
cargo test --workspace
cargo test --workspace --release
```

Issue #111 Browser E2E 及全量 board/project E2E 也已通过。Release Tauri build 已生成 `.app`；`codesign --verify --deep --strict` 通过。构建命令末尾的非零状态仅来自本机未设置 updater 所需的 `TAURI_SIGNING_PRIVATE_KEY`，不影响 `.app` 的生成和本地验收。

Local Markdown Project 的文件内容 revision 会由 Host tick 检测；即使窗口不在前台，外部新增或修改实际 Markdown 文件也会触发该 Project 刷新。当前打开的 Issue document cache 会随之失效，并由 Client tick 自动重新读取新正文；Issue #111 的 headless Browser E2E 直接改写实际 Markdown 文件后，已验证详情恢复为 ready、展示新正文且编辑器读到同一内容。

## 运行稳定性备注

本轮 Release App 的创建、编辑、评论、状态/关系写入、失败恢复、刷新和重启没有产生新的 crash report。系统中此前已有的 `agent-taskboard-2026-08-30-081626.ips` 不由本轮新增操作产生。
