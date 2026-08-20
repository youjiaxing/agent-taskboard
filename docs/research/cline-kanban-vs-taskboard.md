# 调研：Cline Kanban 与 Agent Taskboard 的对照

- **Ticket**: [#35](https://github.com/youjiaxing/agent-taskboard/issues/35)
- **Branch**: `research/cline-kanban-vs-taskboard`
- **Date**: 2026-08-20
- **Skill**: `research`
- **Scope**: 专评 Cline Kanban（https://github.com/cline/kanban ）与已钉的 Agent Taskboard v1 是否同一类产品。以三份旧文稿为起点，全部跟到 **当前** 主源（README / 官方文档 / 源码），标出相对旧文稿「变了什么、没变什么」。不写 ADR；不替 [#36](https://github.com/youjiaxing/agent-taskboard/issues/36)（是否改写 v1 规格）拍板。
- **起点文稿**（不重做全市场）:
  - [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) `research/agent-kanban-models` → `docs/research/agent-kanban-models.md`（Cline §2.3）
  - [#18](https://github.com/youjiaxing/agent-taskboard/issues/18) `research/agent-worktree-isolation` → `docs/research/agent-worktree-isolation.md`（Cline §3.1）
  - [#30](https://github.com/youjiaxing/agent-taskboard/issues/30) `research/comparable-features` → `docs/research/comparable-features.md`
- **词表**: 根目录 `CONTEXT.md`（Issue / Project / Run / Frontier / Host / Client / Embedded Terminal / Dependency / 认领 / 父 Issue / 上次态势 / 隔离执行目录 / 等待操作 / 执行已停 / 自动推进 / 待确认 / 自检 / 查看改动 / 改动备注 / 配对）。不用同义替换。
- **取证时间**: 2026-08-20。Cline 仓库 `pushed_at` 2026-08-13，版本仍为 0.1.70（[repo API](https://api.github.com/repos/cline/kanban)）。

---

## 1. Cline Kanban 现状（对到当前主源）

### 1.1 形态与定位

- `npx kanban` 本地 Node runtime + 浏览器控制面，无账号开箱即用（README：*"No account or setup required, it works right out of the box"*）。[README](https://github.com/cline/kanban/blob/main/README.md)
- **已并入 Cline CLI 产品族**：`cline --kanban` → `http://localhost:3484`，官方称 *"Cline Kanban is the orchestration surface for the same agent runtime… It also speaks Claude Code and Codex, so you can run a mixed fleet"*。[cline.bot/cli](https://cline.bot/cli)
- 官方文档站新增 Kanban 专页：[usage/kanban](https://docs.cline.bot/usage/kanban)（入门）、[core-workflow](https://docs.cline.bot/kanban/core-workflow)（端到端流程）、[remote-access](https://docs.cline.bot/kanban/remote-access)（远程访问）。
- 仍是 **Research Preview**；README 置顶警告使用 bypass permissions / runtime hooks 等实验能力。[README](https://github.com/cline/kanban/blob/main/README.md)
- 许可 Apache-2.0 © 2026 Cline Bot Inc.；~1.26k★，非归档。[README](https://github.com/cline/kanban/blob/main/README.md)、[repo](https://github.com/cline/kanban)

### 1.2 工作单元与板存储（不变）

- 工作单元仍是**自建 Task card**（prompt + baseRef + review 设置），不是外部 Issue。[README](https://github.com/cline/kanban/blob/main/README.md)
- 板状态仍落 `~/.cline/kanban/workspaces/<id>/board.json`（另有 index.json / sessions.json / meta.json，文件锁防并发写）。[workspace-state.ts](https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts)
- 列仍是 Backlog → In Progress → Review → Done（列 id 为 `trash`、显示名 "Done"，0.1.67 从 "Trash" 改名）。[workspace-state.ts](https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts)、[CHANGELOG 0.1.67](https://github.com/cline/kanban/blob/main/CHANGELOG.md)

### 1.3 派活面（不变）

- 双执行路径不变：**多数 Agent 走 PTY 跑官方 CLI**（`src/terminal/`，node-pty，0.1.51 起 PTY 全服务端）；**Cline 引擎走 SDK 原生聊天**（`src/cline-sdk/`，用发布包 `@clinebot/core` / `@clinebot/agents` / `@clinebot/llms`，自有 provider/OAuth/会话持久化）。[architecture.md](https://github.com/cline/kanban/blob/main/docs/architecture.md)
- 人点卡片 play 启动；另有 Start All（0.1.10、0.1.30 快捷键）。[README](https://github.com/cline/kanban/blob/main/README.md)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- **侧栏 Agent 可代管看板**：把工作拆成卡片、建链、直接开跑；看板管理经 `kanban task list|create|update|link|unlink|start` CLI（MCP 已移除，改为 skill + CLI 指令注入）。[README](https://github.com/cline/kanban/blob/main/README.md)、[man/kanban.1](https://github.com/cline/kanban/blob/main/man/kanban.1)、[.plan/docs/kanban-mcp-removal-handoff.md](https://github.com/cline/kanban/blob/main/.plan/docs/kanban-mcp-removal-handoff.md)
- Agent 目录现为 7 家：claude / codex / gemini / opencode / droid / kiro / cline。[api-contract.ts](https://github.com/cline/kanban/blob/main/src/core/api-contract.ts)
- 高自主形态仍在：Claude Code 自动任务用 `--permission-mode auto`（0.1.69 起不再完全 bypass）、Codex `--dangerously-bypass-approvals-and-sandbox`、Gemini `--yolo`、Codex hooks pre-trusted（0.1.68）。[agent-session-adapters.ts](https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)

### 1.4 Tracker 关系（不变）

- **无外部 Issue Tracker**；GitHub 只作 Commit / Open PR 出口（由 agent 在 TUI 里执行 git 动作，动态 prompt 驱动）。[README](https://github.com/cline/kanban/blob/main/README.md)、[core-workflow](https://docs.cline.bot/kanban/core-workflow)
- GitHub / Linear 只以 **MCP 服务器**形式出现在 agent 的工具面（如 Linear MCP 把票变成卡片），板上真源仍是 board.json。[README](https://github.com/cline/kanban/blob/main/README.md)、[cline.bot/cli](https://cline.bot/cli)

### 1.5 依赖与自动编排（不变）

- `⌘+click` 建链；规则不变：至少一端在 Backlog（`non_backlog`）、不能链自己 / 重复 / 链 Done 卡（`trash_task`）。[task-board-mutations.ts](https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts)、[use-linked-backlog-task-actions.ts](https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-linked-backlog-task-actions.ts)
- **依赖完成即自动开跑**：卡片移入 Done 后，已解锁的 Backlog 链接卡自动 kickoff（`trashTaskAndGetReadyLinkedTaskIds`）。[task-board-mutations.ts](https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts)、[core-workflow](https://docs.cline.bot/kanban/core-workflow)
- 0.1.28 补了启动门：**有未完成依赖的卡不能再启动**。[CHANGELOG 0.1.28](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- auto-commit / auto-PR 仍在（`autoReviewEnabled` + `autoReviewMode: commit|pr`，自动 git 动作后移入 Done）。[api-contract.ts](https://github.com/cline/kanban/blob/main/src/core/api-contract.ts)、[use-review-auto-actions.ts](https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-review-auto-actions.ts)、[README](https://github.com/cline/kanban/blob/main/README.md)

### 1.6 隔离（不变）

- 每卡**一律 ephemeral worktree**，由壳（Kanban runtime）创建，不归 Agent CLI：`~/.cline/worktrees/<taskId>/<仓库目录名>`，detached HEAD 停在 baseRef；gitignored 文件（如 `node_modules`）symlink（Turbopack 项目改为复制）。[README](https://github.com/cline/kanban/blob/main/README.md)、[architecture.md](https://github.com/cline/kanban/blob/main/docs/architecture.md)、[CHANGELOG 0.1.32](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- 非 git Project 要求初始化（`requiresGitInitialization`）；resume 语义不变（`claude --continue`、`codex resume`、`gemini --resume latest` 等「最近一次」）。[agent-session-adapters.ts](https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts)

### 1.7 审阅（增强）

- 详情 = Agent TUI + worktree diff；行级评论回投 agent。[README](https://github.com/cline/kanban/blob/main/README.md)
- **新增 checkpoint 系统**：可按「消息区间」看 diff（`src/workspace/turn-checkpoints.ts`），0.1.12 起有 last-turn changes。[README](https://github.com/cline/kanban/blob/main/README.md)、[core-workflow](https://docs.cline.bot/kanban/core-workflow)、[CHANGELOG 0.1.12](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- 0.1.64 多行 diff 评论；0.1.63 起任务标题可编辑。[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- Script Shortcut（如 `npm run dev` 一键跑）仍在。[README](https://github.com/cline/kanban/blob/main/README.md)

### 1.8 产品形态（新出现）

- 本机 Web UI 可装成 **PWA**（0.1.52）；**手机响应式**布局（0.1.60）。[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- **Electron 桌面壳开发中**：`packages/desktop`（`@kanban/desktop` 0.0.1 private，"Electron shell for the Kanban runtime"，electron-builder + node-pty + notarize 脚本），0.1.67 注明 "not yet available"。[packages/desktop/package.json](https://github.com/cline/kanban/blob/main/packages/desktop/package.json)、[CHANGELOG 0.1.67](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- **远程访问一等化**：`--host` 绑定（0.1.25）、HTTPS + passcode（0.1.60）、远程设备码登录 Cline（0.1.61）、企业级远程配置门（0.1.43/45）；官方文档推荐 Tailscale / SSH 隧道 / Docker / ngrok / Cloudflare，默认只绑 `127.0.0.1:3484`。[remote-access](https://docs.cline.bot/kanban/remote-access)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- 无配对协议：LAN 裸绑默认无鉴权，passcode 是可选增强；也没有 Client 能力矩阵概念。[remote-access](https://docs.cline.bot/kanban/remote-access)
- Cline 路径在看板内管 **Cline 账号**（OAuth、org 切换、积分余额与用量通知，0.1.60）；`cline` CLI 要求 "a free Cline account or an API key"。[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)、[cline.bot/cli](https://cline.bot/cli)
- 更新通道：`kanban --update`（0.1.57）、Web UI 内新版通知一键更新（0.1.67）。[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
- 遥测 / 反馈：Sentry（0.1.36）、Featurebase 反馈组件带 Cline 账号数据（0.1.37）。[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)

### 1.9 多项目

- 多 workspace（每个已登记 git 仓一个）各自一张板，侧栏切换，无跨板聚合。[README](https://github.com/cline/kanban/blob/main/README.md)、[workspace-state.ts](https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts)

---

## 2. 对照表：Cline 现状 vs Taskboard 已钉决策

已钉出处：[Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1) 的 Notes / Decisions so far / Out of scope，含 [#9](https://github.com/youjiaxing/agent-taskboard/issues/9)、[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#16](https://github.com/youjiaxing/agent-taskboard/issues/16)、[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)、[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)、[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)。

| 维度 | Cline Kanban 现状 | Taskboard v1 已钉 | 关系 |
| --- | --- | --- | --- |
| 工作单元 | 自建 Task card，落 board.json（[§1.2](#12-工作单元与板存储不变)） | Issue（Tracker 上的工作项）为展示与分派基本单位；不自建第二套库（#11、#17） | **冲突** |
| Tracker | 无；GitHub 只作 Commit/Open PR 出口；GitHub/Linear 仅作 agent 的 MCP 工具（[§1.4](#14-tracker-关系不变)） | Tracker Adapter（v1 GitHub）为单一真源；上次态势只是只读缓存（#11、#29） | **冲突**（出口 vs 真源） |
| 派活 | 人点 play；Start All；依赖完成自动 kickoff；侧栏 Agent 代建/链/开跑（[§1.3](#13-派活面不变)、[§1.5](#15-依赖与自动编排不变)） | 人认领 + 启动 Run；自动推进默认关、待确认 60s、自检（#20）；Dependency 只解锁 Frontier 不自动开跑（#17） | **部分覆盖**（play 启动）+ **冲突**（自动 kickoff / 代管板） |
| 依赖 | 卡片链接：至少一端 Backlog、Done 不可入链、未完成依赖阻止启动、完成即自动开跑（[§1.5](#15-依赖与自动编排不变)） | Dependency 是 Tracker 上的阻塞关系；被阻塞方不进 Frontier；父 Issue 是另一回事（#11、CONTEXT.md） | **部分覆盖**（链接语义基础）+ **冲突**（自动开跑） |
| 隔离 | 每卡一律 worktree，由壳建树，detached HEAD + gitignored symlink；非 git 要初始化（[§1.6](#16-隔离不变)） | 默认 Run 在 Project 主目录；隔离执行目录只走 Agent 原生 worktree；看板不替 CLI 建树（#16 / ADR 0004） | **覆盖目标**（并行不互踩）+ **冲突**（一律 worktree、壳建树） |
| 审阅 | 详情 = TUI + diff（checkpoint 按消息区间）；行级评论即时回投在跑的 agent；Commit/Open PR 由 agent 执行；auto-commit/auto-PR 可选（[§1.7](#17-审阅增强)） | 查看改动只读、相对启动 commit、多仓现场现算；改动备注只进下一轮开场白；v1 无 PR 入口、PR 不当完成证据（#22 / ADR 0009） | **覆盖**（diff + 行级评论素材）+ **冲突**（评论回投在跑的 Run；PR/auto-commit 出口） |
| 完成信号 | 卡片移入 Done 即「完成」语义（触发依赖 kickoff）；auto-review 可自动 commit/PR 后进 Done；agent 可代 trash 卡（0.1.25） | Run 结束 ≠ Issue 完成；完成 = Issue 关闭（看板不代关）；SessionEnd / StopFailure / 退出码只影响 Run 结束态；自动推进默认关（#9、#20） | **冲突** |
| 产品形态 | 本机 Node runtime + 浏览器（PWA）；Electron 壳未发布；无账号（看板本体）；远程 = 用户网络绑定 + 可选 passcode，无配对（[§1.8](#18-产品形态新出现)） | Tauri 2 桌面 + 浏览器（含手机）Client；Host 常驻；配对 = 一次性码 + 长期令牌 + 可撤销；本机回环页免配对（#12、#21 / ADR 0006、0007） | **覆盖**（本机无账号、用户网络远程）+ **正交**（常驻 Host、配对协议、能力矩阵） |
| 多项目 | 多 workspace 各自板，无跨板聚合（[§1.9](#19-多项目)） | 多 Project 各自 Frontier；中间四列只跟当前选中 Project；不做跨 Project/Host 聚合（#11、#14、#15） | **覆盖**（同构） |

---

## 3. 三列清单

### 3.1 Cline 覆盖了 Taskboard

| # | Taskboard 需求 | Cline 证据 |
| --- | --- | --- |
| 1 | 人点启动 → 官方 CLI 进 Embedded Terminal（派活主路径） | play 按钮 → 每卡 PTY 跑官方 CLI（[README](https://github.com/cline/kanban/blob/main/README.md)、[architecture.md](https://github.com/cline/kanban/blob/main/docs/architecture.md)） |
| 2 | 并行 Run 不互踩（隔离目标） | 每卡独立 worktree（[README](https://github.com/cline/kanban/blob/main/README.md)） |
| 3 | 列表上的 Run 忙闲可见性（在跑/等/停三态素材，[#32](https://github.com/youjiaxing/agent-taskboard/issues/32)） | hooks 把最新消息/工具调用显示在卡片上（[README](https://github.com/cline/kanban/blob/main/README.md)） |
| 4 | 查看改动（diff 素材） | 详情里 TUI + worktree diff + checkpoint（[core-workflow](https://docs.cline.bot/kanban/core-workflow)） |
| 5 | Dependency 的链接语义基础（至少一端可执行、Done 不可入链） | `non_backlog` / `trash_task` 链接规则（[task-board-mutations.ts](https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts)） |
| 6 | 多 Project 各自看板、不做跨板聚合 | 多 workspace 各自板（[README](https://github.com/cline/kanban/blob/main/README.md)） |
| 7 | 远程访问走用户自己的网络、无产品中继 | 官方文档推荐 Tailscale / SSH 隧道 / 自建隧道（[remote-access](https://docs.cline.bot/kanban/remote-access)） |
| 8 | 看板本体无账号（个人本地优先） | "No account or setup required"（[README](https://github.com/cline/kanban/blob/main/README.md)） |

### 3.2 正交（Cline 不做、Taskboard 要做）

| # | Taskboard 已钉 | 说明 |
| --- | --- | --- |
| 1 | Issue Tracker 接驳（GitHub 读写、评论、写回） | Cline 完全没有 Tracker 层（[§1.4](#14-tracker-关系不变)） |
| 2 | Frontier 定义与筛选（未关闭 ∧ 无未完成阻塞 ∧ 未被认领） | Cline 无 Frontier；Backlog 卡都可点 play（词表见根目录 `CONTEXT.md`） |
| 3 | 认领（Tracker 上的 assignee 钉子） | Cline 无认领概念，只有卡列（词表见根目录 `CONTEXT.md`） |
| 4 | 父 Issue 层次（拆分与归属，非阻塞） | Cline 无父/子概念（词表见根目录 `CONTEXT.md`） |
| 5 | Triage Role / Label Mapping / skills 只读透镜 | Cline 无（[#10](https://github.com/youjiaxing/agent-taskboard/issues/10)、词表见根目录 `CONTEXT.md`） |
| 6 | 完成信号体系：SessionEnd / StopFailure / 待确认 60s / 自检 / 自动推进开关 | Cline 无完成信号概念，Done 列即完成（[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)，ADR 0005） |
| 7 | 上次态势 / 离线（只读副本，不拿旧数据认领） | Cline 的 board.json 是本地 SoT，无离线/缓存语义（[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)、[workspace-state.ts](https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts)） |
| 8 | Host 常驻进程 + 配对协议（一次性码、长期令牌、撤销）+ Client 能力矩阵 | Cline 是启动即用的本地 server；LAN 裸绑、passcode 可选、无配对、手机是全量响应式（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[remote-access](https://docs.cline.bot/kanban/remote-access)） |
| 9 | 启动配置表单 + 启动环境快照（Adapter 声明字段、按 Project×Agent 记默认） | Cline 在设置里选 agent/model，但无「目标目录用户默认壳整环境快照」概念（[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#23](https://github.com/youjiaxing/agent-taskboard/issues/23)） |
| 10 | 改动备注只进下一轮开场白、不灌在跑的 Run | Cline 的评论是即时回投在跑的 agent（[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)、[core-workflow](https://docs.cline.bot/kanban/core-workflow)） |
| 11 | 通知走本机系统通道、点击跳转对应 Run/Issue | Cline 只有浏览器通知（0.1.13/0.1.56），无本机系统通道（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#32](https://github.com/youjiaxing/agent-taskboard/issues/32)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)） |

### 3.3 冲突（Cline 做了、Taskboard 已拒）

| # | Cline 现状 | Taskboard 已拒 |
| --- | --- | --- |
| 1 | 自建卡片当工作单元（board.json） | 自建第二套任务库当 SoT（#11、#17 §4.2） |
| 2 | 依赖完成自动开跑（linked tasks auto-start） | Dependency 只解锁 Frontier，不自动开跑（#17 §4.1、[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)） |
| 3 | auto-commit / auto-PR，agent 跑完自己 ship | v1 产品化「开 PR」入口、PR 当完成证据（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope、#22） |
| 4 | 侧栏 Agent 代管看板（建卡/链/开跑；`kanban task` CLI） | 观察 ≠ 编排；v1 不做默认无人值守编排器（#17 §4.1、[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope） |
| 5 | 一律 worktree 且由壳建树 | 默认 Project 主目录；隔离执行目录只走 Agent 原生 worktree；看板不替 CLI 建树（#16 / ADR 0004） |
| 6 | 看板内完整 git 管理（commit 历史/切分支/push/可视化） | 看板内完整 git 管理应拒；查看改动只读（#30 §15.5、#22） |
| 7 | 高自主权限（`--permission-mode auto`、`--yolo`、bypass approvals、hooks pre-trusted） | 权限询问留在官方 TUI；yolo/skip-permissions 当默认应拒（#17 §4.5、[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)） |
| 8 | Cline 引擎走 SDK 原生聊天（非官方 TUI PTY） | 自研聊天式 Agent UI 替代官方 CLI TUI（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope）；Agent 定义要求官方交互 TUI（词表见根目录 `CONTEXT.md`）。注：Cline 是自家 SDK 聊天，且 v1 Agent 名单无 Cline，冲突程度按此打折 |
| 9 | Cline 路径在看板内管 Cline 账号（OAuth、org、积分） | 看板不管各家 Agent 登录态 / API key（#13、[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope） |
| 10 | 遥测 / 反馈组件带账号数据（Sentry、Featurebase） | 无账号、本地优先（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)）；轻冲突 |

---

## 4. 相对三份旧文稿：变了什么、没变什么

### 4.1 没变的（[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) / [#18](https://github.com/youjiaxing/agent-taskboard/issues/18) / [#30](https://github.com/youjiaxing/agent-taskboard/issues/30) 的核心结论仍然成立）

- 工作单元仍是自建卡片；`~/.cline/kanban/workspaces/*/board.json` 路径不变；列结构不变（Backlog / In Progress / Review / Done，id 为 trash）。[workspace-state.ts](https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts)
- 无外部 Tracker 作为板上真源；GitHub 只作 Commit / Open PR 出口。[README](https://github.com/cline/kanban/blob/main/README.md)
- 双执行路径：多数 Agent PTY 跑官方 CLI、Cline 引擎 SDK 原生聊天。[architecture.md](https://github.com/cline/kanban/blob/main/docs/architecture.md)
- 依赖完成自动 kickoff、auto-commit/auto-PR、链接规则（至少一端 Backlog、Done 不可链）全在。[task-board-mutations.ts](https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts)、[use-linked-backlog-task-actions.ts](https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-linked-backlog-task-actions.ts)、[use-review-auto-actions.ts](https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-review-auto-actions.ts)
- 每卡一律 worktree、壳建树、detached HEAD + gitignored symlink、非 git 需初始化、resume 是「最近一次」（`--continue` / `resume --last` 等）。[README](https://github.com/cline/kanban/blob/main/README.md)、[agent-session-adapters.ts](https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts)
- 审阅 = TUI + diff + 行级评论回投；Script Shortcut 在。[README](https://github.com/cline/kanban/blob/main/README.md)
- 看板本体无账号开箱即用。[README](https://github.com/cline/kanban/blob/main/README.md)
- 版本仍是 0.1.70（仓库 2026-08-13 后无新 release）——**产品本体在旧文稿之后没有发布过新版本**。

### 4.2 变了 / 新出现的（相对三份文稿）

| # | 变化 | 证据 |
| --- | --- | --- |
| 1 | **产品定位**：Kanban 并入 Cline CLI 产品族，`cline --kanban` 成为一等入口；官方文档站新增 usage/kanban、core-workflow、remote-access 三页。旧文稿只把 Kanban 当独立 `npx kanban` 包 | [cline.bot/cli](https://cline.bot/cli)、[usage/kanban](https://docs.cline.bot/usage/kanban) |
| 2 | **远程访问一等化**：`--host` 绑定、HTTPS + passcode（0.1.60）、远程设备码登录（0.1.61）、官方 remote-access 文档（Tailscale/SSH/Docker/ngrok/Cloudflare）、企业远程配置门（0.1.43/45）。三份文稿均未涉及 | [remote-access](https://docs.cline.bot/kanban/remote-access)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |
| 3 | **桌面端**：Electron 壳（`packages/desktop`，0.0.1 private，"not yet available"）。旧文稿只写「本机 Web」 | [packages/desktop/package.json](https://github.com/cline/kanban/blob/main/packages/desktop/package.json)、[CHANGELOG 0.1.67](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |
| 4 | **审阅增强**：checkpoint 系统（diff 按消息区间）、0.1.12 last-turn changes、0.1.64 多行 diff 评论 | [README](https://github.com/cline/kanban/blob/main/README.md)、[core-workflow](https://docs.cline.bot/kanban/core-workflow)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |
| 5 | **板管理 CLI**：`kanban task list/create/update/link/unlink/start`（MCP 移除，改 skill + CLI） | [man/kanban.1](https://github.com/cline/kanban/blob/main/man/kanban.1)、[.plan/docs/kanban-mcp-removal-handoff.md](https://github.com/cline/kanban/blob/main/.plan/docs/kanban-mcp-removal-handoff.md) |
| 6 | **依赖启动门**：0.1.28 起有未完成依赖的卡不能启动。旧文稿只记了建链规则，未记这条 | [CHANGELOG 0.1.28](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |
| 7 | **自主性形态微调**：0.1.69 Claude Code 自动任务改用官方 `auto` 权限模式（不再完全 bypass）；0.1.68 Codex hooks pre-trusted。README 的 bypass 警告仍在 | [CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md)、[agent-session-adapters.ts](https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts) |
| 8 | **杂项新功能**：PWA 安装（0.1.52）、手机响应式（0.1.60）、每卡可选 agent/model（0.1.60）、任务标题（0.1.60/64）、图片附件（0.1.46）、`kanban --update` 与新版通知（0.1.57/67）、Cline 账号 org/积分（0.1.60）、Sentry/Featurebase（0.1.36/37） | [CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |
| 9 | **Agent 目录扩到 7 家**：claude/codex/gemini/opencode/droid/kiro/cline（0.1.28 砍过 OpenAI/Gemini/Droid，0.1.58 起加回 Droid、0.1.60 加 Kiro） | [api-contract.ts](https://github.com/cline/kanban/blob/main/src/core/api-contract.ts)、[CHANGELOG](https://github.com/cline/kanban/blob/main/CHANGELOG.md) |

### 4.3 旧文稿需要修正 / 补充的两处

1. [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) §2.3 说「依赖至少一端必须在 Backlog；Done 中的卡不能再链」——对，但要补：**有未完成依赖的卡也不能启动**（0.1.28）。[CHANGELOG 0.1.28](https://github.com/cline/kanban/blob/main/CHANGELOG.md)
2. [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) / [#18](https://github.com/youjiaxing/agent-taskboard/issues/18) 都把 Cline 写成「本机 Web」——要补：Electron 桌面壳已在仓库内（未发布），远程访问已官方文档化。[packages/desktop/package.json](https://github.com/cline/kanban/blob/main/packages/desktop/package.json)、[remote-access](https://docs.cline.bot/kanban/remote-access)

---

## 5. 结论短答（研究意见，不替 #36 拍板）

**是不是同一类产品**：局部同类、整体不同类。Cline Kanban 与 Taskboard 共享「人点启动 → 官方 CLI 进真实终端、每任务一隔离目录、diff + 行级评论、多项目各自板、无账号、远程走用户自己的网络」这一片**执行配套面**（§3.1 覆盖项），这是旧文稿判断的延续。但 Cline 在三个根上不同：① 工作单元是自建卡片、无 Tracker 真源（Taskboard 的 Issue / Frontier / 认领 / 父 Issue / triage 全是它没有的维度，§3.2）；② 完成语义是「卡片进 Done 列」且依赖自动开跑 + auto-commit/auto-PR + 侧栏代管板，是**默认无人值守编排器**的完整形态（§3.3，Taskboard 明确应拒）；③ 产品形态是「启动即用的本地 Web server」，无常驻 Host、无配对协议、无 Client 能力矩阵（§3.2，Taskboard 已钉 #21 正与之相反）。

**给 #36 的事实面**：Cline 相对旧文稿「产品本体没变」（仍 0.1.70），变的是定位（并入 Cline CLI 产品族）、远程访问文档化、桌面壳在途、审阅 checkpoint 与板管理 CLI 等增量（§4.2）；旧文稿「派活/审阅可抄、编排应拒」的结论在当前主源下依然成立。

---

## 6. 证据索引

| 主题 | 主源 |
| --- | --- |
| 仓库形态 / 许可 / 维护 | https://github.com/cline/kanban 、https://api.github.com/repos/cline/kanban（star 1262、pushed 2026-08-13、Apache-2.0、未归档） |
| 产品叙事 / 派活 / 依赖 / 审阅 / 隔离 | https://github.com/cline/kanban/blob/main/README.md |
| 架构（双执行路径、壳建树、Cline SDK 边界、侧栏 Home Agent） | https://github.com/cline/kanban/blob/main/docs/architecture.md |
| 版本史（0.1.4–0.1.70 全部条目） | https://github.com/cline/kanban/blob/main/CHANGELOG.md |
| 板存储 / 列 / 依赖规则 / 自动开跑 | https://github.com/cline/kanban/blob/main/src/state/workspace-state.ts 、https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts 、https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-linked-backlog-task-actions.ts |
| auto-review / agent 目录 / 列枚举 | https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-review-auto-actions.ts 、https://github.com/cline/kanban/blob/main/src/core/api-contract.ts |
| PTY / resume / 高自主参数 | https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts |
| 桌面壳 | https://github.com/cline/kanban/blob/main/packages/desktop/package.json |
| 板管理 CLI | https://github.com/cline/kanban/blob/main/man/kanban.1 、https://github.com/cline/kanban/blob/main/.plan/docs/kanban-mcp-removal-handoff.md |
| 官方文档 | https://docs.cline.bot/usage/kanban 、https://docs.cline.bot/kanban/core-workflow 、https://docs.cline.bot/kanban/remote-access |
| Cline CLI 定位 | https://cline.bot/cli |
| Taskboard 已钉 | [Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1)、`CONTEXT.md`、[#9](https://github.com/youjiaxing/agent-taskboard/issues/9)、[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#16](https://github.com/youjiaxing/agent-taskboard/issues/16)、[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)、[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)、[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)、[#32](https://github.com/youjiaxing/agent-taskboard/issues/32) |

- 旧文稿三份：`docs/research/agent-kanban-models.md`（[#17](https://github.com/youjiaxing/agent-taskboard/issues/17)）、`docs/research/agent-worktree-isolation.md`（[#18](https://github.com/youjiaxing/agent-taskboard/issues/18)）、`docs/research/comparable-features.md`（[#30](https://github.com/youjiaxing/agent-taskboard/issues/30)）。
- 本票未写 ADR；未改 CONTEXT.md；未关 [#35](https://github.com/youjiaxing/agent-taskboard/issues/35)；未替 [#36](https://github.com/youjiaxing/agent-taskboard/issues/36) 拍板。
