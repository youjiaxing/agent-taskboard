# 调研：并行 Agent 的 git worktree 与执行目录隔离

- **Ticket**: [#18](https://github.com/youjiaxing/agent-taskboard/issues/18)
- **服务决策票**: [#16](https://github.com/youjiaxing/agent-taskboard/issues/16)（本票只交事实与选项，不代替 grilling、不关 #16、不写 ADR）
- **Branch**: `research/agent-worktree-isolation`
- **Date**: 2026-08-14
- **Skill**: `research`
- **Scope**: 同类看板 / 桌面控制台如何为并行 Agent 选择执行目录；以及 Grok Build / Codex / Claude Code 官方 CLI 是否自带 worktree / 隔离开关
- **资料原则**: 官方 README / 官方文档 / 仓库源码与本机 CLI `--help`；不采二手榜单
- **已决约束**（不重开）: [#9](https://github.com/youjiaxing/agent-taskboard/issues/9) — Run 通常绑定一个 Issue；同一 Issue 同时最多一个活跃 Run；不同 Issue 的并行 Run 不受全局数量限制；Issue 关闭不终止活跃 Run。领域词见根目录 `CONTEXT.md`（Project / Issue / Run / Agent / Agent Adapter / Embedded Terminal）

---

## 1. 问题

在已决规则「同一 Issue 同时最多一个活跃 Run、不同 Issue 可不受全局数量限制地并行」之下，同类产品如何为并行 Agent 选择和管理**执行目录**？

须钉清事实（不是替 #16 拍板）：

1. 何时复用 Project 主目录，何时创建 git worktree
2. 隔离由壳（看板 / 桌面应用）负责，还是由 Agent CLI / Adapter 负责
3. gitignored 目录（如 `node_modules`）如何共享或复制
4. 非 git Project 如何降级
5. 分支命名、创建 / 清理时机、失败恢复、可恢复的 resume id
6. 目录外副作用（端口、本地数据库、锁文件）它们是否处理、如何处理
7. 官方 CLI 是否已有原生 worktree / 隔离开关，避免壳与 CLI 双层打架
8. 哪些实践可直接借给 Agent Taskboard，哪些与「Run 绑定 Issue、v1 不做自动编排」冲突

---

## 2. 结论摘要

同类产品对「并行执行目录」大致分成四档，**没有一家同时解决了目录隔离 + 端口 / 本地 DB / 锁文件**：

| 档 | 代表 | 默认行为 |
| --- | --- | --- |
| 可选 worktree，默认复用主目录 | Zed、Nimbalyst、Routa | 用户显式开隔离；日常会话仍在主 checkout |
| 任务 / workspace 一律 worktree | cline/kanban、Vibe Kanban、KanVibe（有分支名时） | 壳在启动前 `git worktree add` |
| 会话默认 worktree，可降级复用目录 | OpenHands Agent Server | `worktree=true` 时在 `/tmp/conversation-worktrees/...` 切分支；非 git 或未出生 HEAD 则留在原目录 |
| 不做 git 目录隔离，做 OS sandbox | Codex CLI | 无 `--worktree`；用 `-s/--sandbox` 限制写盘与网络 |

对 **Agent Taskboard v1** 最要紧的三条事实：

1. **Grok Build 与 Claude Code 已自带 `--worktree`**。壳若再包一层 worktree，再把 `--worktree` 传进去，会双层打架。Codex **没有** 对等开关，隔离若要做，只能由壳或 `-s/--sandbox` 承担。
2. **「一律 worktree」产品几乎都要求 Project 是 git 仓，且由壳（不是 Agent CLI）创建 / 清理。** 非 git 要么拒收，要么先 `git init`，要么静默降级回原目录。
3. **目录外副作用普遍不处理。** 共享 `node_modules` 用 symlink（cline）或按清单复制（Vibe / Claude `.worktreeinclude`）；端口与本地 DB 最多给用户自己写 setup / dev-server 脚本。v1 不应假装能自动分端口。

**给 #16 的推荐倾向（研究意见，不是决策）**：默认复用 Project 主目录；用户为某次 Run 显式要求隔离时，**二选一**——要么壳建 worktree 且启动 CLI 时**不要**再传 `--worktree`，要么把 Grok / Claude 的原生 `--worktree` 交给 Adapter 透传且壳**不要**再包一层。Codex 只能走壳 worktree 或它自己的 sandbox。非 git Project 降级为复用主目录，不要学 cline 强制 `git init`。

---

## 3. 项目卡片

### 3.1 cline/kanban — 每张任务卡一张 ephemeral worktree + gitignored symlink

| 项 | 内容 |
| --- | --- |
| 形态 | 本机 `npx kanban`：浏览器控制面 + 本地 Node runtime；多数 Agent 走嵌入 PTY |
| 仓库 | https://github.com/cline/kanban （~1.3k★，Apache-2.0，Research Preview） |
| 隔离策略 | **一律 worktree**：点 Play 后为该任务卡建 ephemeral git worktree，detached HEAD 停在任务 `baseRef` |
| 谁创建 / 清理 | **壳（Kanban runtime）**。架构文档写明 worktree 生命周期归 Kanban，不归 Agent SDK / CLI |
| 路径 | `~/.cline/worktrees/<taskId>/<仓库目录名>` |
| 分支 | **不建命名分支**；`git worktree add --detach <path> <baseCommit>` |
| 非 git | 添加 Project 时若无 `.git`，返回 `requiresGitInitialization`；可勾选初始化。无初始 commit 则无法从 `baseRef` 建 worktree |
| gitignored 共享 | 从主仓 `git ls-files --others --ignored` 列出后 **symlink** 进 worktree；跳过 `.git` / 系统垃圾文件；Next/Turbopack 的 `node_modules` 刻意不链（避免扫描串味） |
| resume | 丢进 Trash 前尽量打 patch（`~/.cline/kanban/trashed-task-patches/<taskId>.<commit>.patch`）；再启动时按原 commit 重建并 `git apply`。README 说「Kanban tracks the resume ID」；Adapter 层实际是 `claude --continue`、`codex resume --last`、`gemini --resume latest` 这类「最近一次」而不是强绑 UUID |
| 目录外副作用 | **不处理**。README 注明 symlink 适合「日常不改 gitignored 文件」；要改依赖目录就别用 Kanban |
| 已知坑 | 旧实现会在 base 分支前进时重建 worktree、毁掉进度，现已改为「已有 worktree 即权威」；`git worktree add` 前必须 `prune`，否则会报 `missing but already registered`；setup lock 在关机时要清 |

主源：

- https://github.com/cline/kanban/blob/main/README.md
- https://github.com/cline/kanban/blob/main/docs/architecture.md
- https://github.com/cline/kanban/blob/main/src/workspace/task-worktree.ts
- https://github.com/cline/kanban/blob/main/src/workspace/task-worktree-path.ts
- https://github.com/cline/kanban/blob/main/src/workspace/task-worktree-turbopack.ts
- https://github.com/cline/kanban/blob/main/src/trpc/projects-api.ts
- https://github.com/cline/kanban/blob/main/src/terminal/agent-session-adapters.ts

**对 Taskboard**：PTY 跑官方 CLI、每张卡一个 cwd，和 Embedded Terminal + Run 很像。但「点 Play 必建 worktree + 自动 commit / 链式开下一张卡」是自动编排，和 v1 人工分派冲突。侧栏 Home Agent **故意不建** worktree，说明他们也承认「不是所有会话都该隔离」。

---

### 3.2 BloopAI/vibe-kanban — Workspace = 多仓 worktree 容器

| 项 | 内容 |
| --- | --- |
| 形态 | 本机 `npx vibe-kanban` + 可选 Cloud；**产品正在 sunsetting** |
| 仓库 | https://github.com/BloopAI/vibe-kanban （~28k★，Apache-2.0） |
| 隔离策略 | **一律 worktree**。一个 Workspace 是目录容器，里面每个 repo 一份 worktree；同一 Workspace 可挂多个 Session |
| 谁创建 / 清理 | **壳**（`workspace-manager` + `worktree-manager`）。Agent CLI 只被丢进已建好的目录 |
| 路径 | 默认 `get_vibe_kanban_temp_dir()/worktrees`（macOS 为系统 temp 下的 `vibe-kanban/worktrees`；Linux 用 `/var/tmp/vibe-kanban/worktrees`）。用户覆盖目录时强制落在 `<自定义路径>/.vibe-kanban-workspaces`，避免误扫用户原有文件夹。官方文档写「通常在 home 下的 `.vibe-kanban-workspaces`」，与源码默认 temp 路径不完全一致 |
| 分支 | `{prefix}/{shortUuid4}-{slug16}`，例如 `vk/1a2b-implement-auth`。`prefix` 来自全局设置 `git_branch_prefix`；空前缀则无 `/` |
| 非 git | Project / Repo **必须是 git 仓**。文档「从空白项目创建」会先生成新 git 仓。路径「不是 git repository」直接 400 |
| gitignored 共享 | **不 symlink**。Repo 设置 `copy_files`：逗号分隔 glob，worktree 建好后、setup script 之前从主仓 **复制** 文件（已存在则跳过）。文档点名 `.env` |
| 创建 / 清理时机 | 用户创建并启动 Workspace 时建仓；删除 / 归档走后台清理（先停进程含 dev server，再 `git worktree remove` + 删目录，可选删分支）。启动时可 `cleanup_orphan_workspaces`；`DISABLE_WORKTREE_CLEANUP` 可关。创建失败会回滚已建 worktree |
| resume | Workspace / Session UUID 持久在 SQLite；冷启动 `ensure_workspace_exists` 按记录重建缺失 worktree |
| 目录外副作用 | **用户脚本**：`setup_script`、`cleanup_script`、`archive_script`、`dev_server_script`。预览靠用户自己的 dev server 命令，壳不分配端口。失败常见原因文档写「主仓有未提交变更导致 worktree 创建失败」 |
| 已知坑 | worktree 创建用路径级锁防竞态；存在则校验 git 元数据，坏了就 comprehensive cleanup 再重建；旧单仓布局会 migrate 到 `workspace_dir/{repo_name}` |

主源：

- https://github.com/BloopAI/vibe-kanban/blob/main/README.md
- https://vibekanban.com/docs/workspaces/creating-workspaces
- https://vibekanban.com/docs/core-features/creating-projects
- https://vibekanban.com/docs/settings/general
- https://github.com/BloopAI/vibe-kanban/blob/main/crates/worktree-manager/src/worktree_manager.rs
- https://github.com/BloopAI/vibe-kanban/blob/main/crates/workspace-manager/src/workspace_manager.rs
- https://github.com/BloopAI/vibe-kanban/blob/main/crates/local-deployment/src/container.rs
- https://github.com/BloopAI/vibe-kanban/blob/main/crates/local-deployment/src/copy.rs
- https://github.com/BloopAI/vibe-kanban/blob/main/crates/utils/src/text.rs

**对 Taskboard**：`copy_files` + setup script 是「gitignored 怎么办」里最完整的壳侧方案。但 Workspace 是比 Run 更重的对象（可多仓、多 Session、自带 dev server），且产品在收摊，不宜当领域模型模板。旧文档「Monitoring Task Execution」仍写 attempt 结束后自动清 worktree，与现行 Workspace 持久模型不一致，以源码与 Workspaces 文档为准。

---

### 3.3 phodal/routa — 按需手动 worktree，默认仍共享 cwd

| 项 | 内容 |
| --- | --- |
| 形态 | Workspace-first 看板；Web（Next）+ Desktop（Tauri / Axum）双后端，语义对齐 |
| 仓库 | https://github.com/phodal/routa （~1.8k★，MIT） |
| 隔离策略 | **默认不隔离**。规格写明：多 Agent 原本共享同一 `cwd` 会互相踩文件；worktree 是 **on-demand 手动创建**，不是开 Session 就建 |
| 谁创建 / 清理 | **壳**。REST：`POST /workspaces/{id}/codebases/{id}/worktrees`；删 `DELETE /worktrees/{id}?deleteBranch=`。ACP Session 可带 `worktreeId`，用该路径当 cwd |
| 路径 | `~/.routa/worktrees/{workspaceId}/{codebaseId}/{branch-safe-name}/` |
| 分支 | 默认 `wt/{label 或 uuid 前 8 位}`；也可传入已有分支。同 codebase 同分支冲突返回 409 |
| 非 git | worktree API 建立在 codebase 的 `repo_path` 上，走 `git worktree add`；没有单独的「非 git 目录复制」降级 |
| gitignored 共享 | **未做** symlink / copy 清单 |
| 创建 / 清理 | 先写 DB（`status=creating`），再 `git worktree prune` + `add`；失败标 `error` 并保留记录。删除：`removing` → `worktree remove --force` → prune → 可选 `branch -D`。codebase 删除时级联清 worktree。提供 `POST /worktrees/{id}/validate`（目录与 `.git` 在不在） |
| resume | worktree 可挂 `sessionId`；会话结束应清空。失败恢复靠 status + validate，不是自动重建 |
| 目录外副作用 | 另有 sandbox policy / Docker sandbox，与 worktree 正交，不是「并行写文件」的解法 |
| 已知坑 | 同仓操作有 per-repo mutex，防 `.git/worktrees` 损坏；创建失败会留下 `error` 记录，需人工删 |

主源：

- https://github.com/phodal/routa/blob/main/README.md
- https://github.com/phodal/routa/blob/main/.qoder/specs/routa-worktree-feature.md
- https://github.com/phodal/routa/blob/main/crates/routa-server/src/api/worktrees.rs
- https://github.com/phodal/routa/blob/main/crates/routa-core/src/models/worktree.rs
- https://github.com/phodal/routa/blob/main/docs/adr/0003-workspace-first-scope.md

**对 Taskboard**：和「默认复用主目录、隔离是显式动作」最接近。Kanban 自动化 / 多 specialist 车道是自动编排，v1 不要搬。

---

### 3.4 OpenHands — 控制面不管隔离；Agent Server / Cloud sandbox 管

| 项 | 内容 |
| --- | --- |
| 形态 | Agent Canvas（浏览器控制面）+ 可切换 Backend（本机 / Docker / VM / Cloud） |
| 仓库 | https://github.com/OpenHands/OpenHands （~84k★，MIT）；worktree 实现在 https://github.com/OpenHands/software-agent-sdk |
| 隔离策略 | **两层**：① 本机会话可选 `local_repo`（直接用用户目录，**即使不是 git**）或 `new_worktree`（每会话一个 worktree）；② Cloud / Docker 是容器级沙箱。架构文档明确：**Canvas 不提供 sandbox / workspace 隔离** |
| 谁创建 / 清理 | **Agent Server**。Canvas 只传 `worktree: bool` 与绝对 `working_dir`。子会话工具 `launch_child_conversation`：`target=local` 可选 `isolation=worktree\|shared`（默认 worktree）；`target=cloud` 始终独立 sandbox，禁止再传 isolation |
| 路径 | `/tmp/conversation-worktrees/<conversation_id>/<repo_name>`（可用 `conversation_worktree_root` 改） |
| 分支 | `openhands/<conversation_uuid>`；起点优先 `origin/<default>`（先 `fetch`），否则本地 `main` / `master`，再退 `HEAD` |
| 非 git | `validate_git_repository` 失败则 **worktree=None，继续用原目录**。未出生 HEAD / 纯 scratch 目录不能 `worktree add`：子会话会 **降级 shared** 并写 `isolation_note` |
| gitignored 共享 | **未做** 自动 symlink / copy |
| 创建 / 清理 | 创建前若路径已在则 `worktree remove --force` + prune，并 `branch -D` 同名分支。用户未选本地文件夹时默认 `new_worktree`；显式选了文件夹则默认 `local_repo` |
| resume | conversation id 即恢复键；工作目录由 Agent Server 按会话重绑。Canvas 把 `selected_workspace` / `workspace_mode` 存在 localStorage |
| 目录外副作用 | Docker 模式按 `PROJECTS_PATH` 挂载；本机无沙箱时 README 警告「agent 拥有整盘」。Canvas 把 backend 的 `runtime_services` URL 塞进 agent 上下文，避免猜端口 |
| 已知坑 | 相对 `working_dir` 曾被解析到错误 CWD，上传路径对不上 worktree（规格 WUP-001）；Cloud 子会话与本机 workspace 不能混用 parent |

主源：

- https://github.com/OpenHands/OpenHands/blob/main/README.md
- https://github.com/OpenHands/OpenHands/blob/main/docs/architecture.md
- https://github.com/OpenHands/OpenHands/blob/main/src/api/conversation-metadata-store.ts
- https://github.com/OpenHands/OpenHands/blob/main/src/api/conversation-service/agent-server-conversation-service.api.ts
- https://github.com/OpenHands/OpenHands/blob/main/src/api/launch-child-conversation-client-tool.ts
- https://github.com/OpenHands/OpenHands/blob/main/src/services/child-conversation-launch.ts
- https://github.com/OpenHands/software-agent-sdk/blob/main/openhands-sdk/openhands/sdk/conversation/request.py
- https://github.com/OpenHands/software-agent-sdk/blob/main/openhands-agent-server/openhands/agent_server/conversation_service.py

**对 Taskboard**：控制面 / 执行面分离可借鉴。但 OpenHands 的执行面是自研 Agent Server，不是「Embedded Terminal 跑官方 CLI」。若 v1 坚持官方 TUI，**不要**再叠一层 Agent Server worktree。Docker / VM 超出 v1 范围。

---

### 3.5 Zed — 并行线程默认共享项目；冲突时用户开 linked worktree

| 项 | 内容 |
| --- | --- |
| 形态 | 开源桌面编辑器；Threads Sidebar 按 project 分组并行 Agent / Terminal Thread |
| 仓库 / 文档 | https://github.com/zed-industries/zed （~89k★）；https://zed.dev/docs/ai/parallel-agents |
| 隔离策略 | **默认复用当前 Project 目录**。仅当「两线程可能改同一批文件」时，用标题栏 worktree picker 新建 linked worktree |
| 谁创建 / 清理 | **编辑器壳**。新 worktree 为 **detached HEAD**，避免两棵树占用同一分支。归档线程且无其他活跃线程占用时，保存 git 状态并删盘；从 History 恢复会重建 worktree。只删 **Zed 自己建过** 的 worktree（本地 DB 记账） |
| 路径 | 设置 `git.worktree_directory`，默认 `"../worktrees"`。解析结果在项目外时自动追加项目名：`~/code/zed` → `~/code/worktrees/zed/` |
| 分支 | 创建时不自动 checkout 已占用分支；用户再用 branch picker 建新分支或选空闲分支 |
| 非 git | worktree 隔离依赖 git。Zed 的「worktree」一词在信任模型里也指任意打开的目录 / 单文件，与 git worktree 不是同一概念 |
| gitignored 共享 | **不自动复制**。提供 `create_worktree` Task hook，环境变量 `ZED_WORKTREE_ROOT` / `ZED_MAIN_GIT_WORKTREE`，文档示例是拷 `.env` |
| resume | Thread History 恢复会话；若当时的 git worktree 已删会自动恢复。外部 Agent 的 history / checkpoint 能力因 ACP 集成而异 |
| 目录外副作用 | **不处理**。Zed Agent 另有 OS sandbox（限制 terminal / fetch），**不作用于** External Agent 与 Terminal Thread |
| 已知坑 | 同一分支不能同时 checkout 在两棵树上（git 限制）；sandbox 挡不住 language server / 普通终端等旁路 |

主源：

- https://zed.dev/docs/ai/parallel-agents
- https://zed.dev/docs/ai/agent-panel
- https://zed.dev/docs/git（Git Worktrees 节）
- https://zed.dev/docs/tasks（`create_worktree` hook）
- https://zed.dev/docs/reference/all-settings.md（`git.worktree_directory`）
- https://github.com/zed-industries/zed/blob/main/crates/git_ui_core/src/created_worktrees.rs

**对 Taskboard**：并行 Run 列表按 Project 分组 + **默认不隔离、冲突再切树**，是最贴近「Issue 态势为主、执行是强配套」的桌面实践。Task hook 比盲目 symlink 全部 gitignored 更安全。

---

### 3.6 Nimbalyst — 常规会话用主目录；Worktree Session 可选

| 项 | 内容 |
| --- | --- |
| 形态 | Electron 桌面：可视化工作区 + Claude Code / Codex / OpenCode |
| 仓库 | https://github.com/nimbalyst/nimbalyst （~1.5k★，MIT） |
| 隔离策略 | **双模式**。Regular session：主 workspace、当前分支。Worktree session：独立目录 + `worktree/<name>` 分支。一棵 worktree 可挂多个 session，一个 session 最多一棵树 |
| 谁创建 / 清理 | **壳**（`GitWorktreeService` + SQLite `worktrees` 表）。IPC：`worktree:create/delete/list/...`。创建用 per-repo lock |
| 路径 | `../{projectName}_worktrees/{name}/` |
| 分支 | `worktree/{finalName}`；名字默认同目录名（文档写 adjective-noun，如 `swift-falcon`），冲突则 `-1` `-2` |
| 非 git | `simpleGit.checkIsRepo()` 失败直接抛 `Not a git repository` |
| gitignored 共享 | **未做** 自动复制 / symlink |
| 创建 / 清理 | 用户点 New Worktree / `Cmd+Alt+W`。删除：去目录 + `git worktree remove` + 删分支。归档走后台队列。删树时 `ai_sessions.worktree_id` SET NULL，会话历史保留 |
| resume | `ai_sessions` 持久；`worktreePath` 作为 Claude Code `workspacePath`。可靠性文档承认：先建 git 再写 DB 失败会孤儿树；归档队列曾只在内存 |
| 目录外副作用 | **不处理**。Blitz / Super Loop 会为并行或循环再各建一棵树 |
| 已知坑 | 文档 `WORKTREE_RELIABILITY_IMPROVEMENTS.md` 列出 DB/git 不一致、squash 无备份、无健康检查等；实现里创建已加 lock，归档仍可能在崩溃后不一致 |

主源：

- https://github.com/nimbalyst/nimbalyst/blob/main/docs/WORKTREES.md
- https://github.com/nimbalyst/nimbalyst/blob/main/docs/WORKTREE_RELIABILITY_IMPROVEMENTS.md
- https://github.com/nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- https://github.com/nimbalyst/nimbalyst/blob/main/packages/electron/src/main/services/GitWorktreeService.ts

**对 Taskboard**：Regular vs Worktree 双模式可直接映射「默认 Run 用 Project 目录 / 可选隔离 Run」。一树多会话则与「一个 Issue 同时最多一个活跃 Run」不完全同构——Taskboard 若做隔离，更干净的是 **一 Run 一树**。

---

### 3.7 rookedsysc/kanvibe — 建卡（带分支名）就建 worktree + tmux/zellij

| 项 | 内容 |
| --- | --- |
| 形态 | 键盘优先桌面看板；嵌入终端（tmux / zellij / 本地 PTY） |
| 仓库 | https://github.com/rookedsysc/kanvibe （~0.1k★，AGPL-3.0） |
| 隔离策略 | 创建 TODO 且提供分支名时：`git worktree add <path> -b <branch> <base>`。也支持 `createSessionWithoutWorktree`（沿用已有目录）。扫描对话框会把仓里 **已有 worktree** 登记成 TODO |
| 谁创建 / 清理 | **壳**。清理入口是 **删除任务**，不是移到 DONE |
| 路径 | `../{projectName}__worktrees/{branch 中 / 换成 -}` |
| 分支 | 用户给的分支名；删除时 `branch -D`（项目根上当前 checkout 的分支受保护，不删） |
| 非 git | 前置依赖就是 git；注册的是 git 仓 |
| gitignored 共享 | **未做** |
| 创建 / 清理时机 | **源码**：`moveTaskToColumn(DONE)` 只改状态，保留 `sessionName` / `worktreePath`。`deleteTask` 才 `removeSessionOnly` + `removeWorktreeAndBranch`。官方站点英文首页也写「只把卡移到 DONE 不会清理」。README 仍写「移到 DONE 会自动删分支 / worktree / 会话」——与源码和站点矛盾，**以源码为准** |
| resume | 任务行记下 `worktreePath` + tmux/zellij `sessionName`；会话在连终端时才真正起来 |
| 目录外副作用 | 不管端口 / DB。zellij 会在 worktree 里写 `.zellij-layout.kdl` |
| 已知坑 | 清理命令对路径做白名单，避免 `rm -rf` 打到项目根；外部（非 `__worktrees` 约定）的 worktree 也曾因 DB 路径为空而清不掉（QA #275） |

主源：

- https://github.com/rookedsysc/kanvibe/blob/main/README.md
- https://github.com/rookedsysc/kanvibe/blob/main/src/lib/worktree.ts
- https://github.com/rookedsysc/kanvibe/blob/main/src/desktop/main/services/kanbanService.ts
- https://github.com/rookedsysc/kanvibe/blob/main/docs-site/content/en/index.mdx

**对 Taskboard**：与 #9「Issue 关闭不杀 Run、Run 结束不表示 Issue 完成」一致的一点是——**看板状态 ≠ 目录生命周期**。README 那种「DONE 就删树」会和已决规则打架；KanVibe 源码其实已经拆开了。

---

## 4. 官方 CLI：原生 worktree / 隔离开关

本机探针：Grok Build `1.0.3 (1a29d5bc12d4)`；Claude Code `2.1.226`；Codex CLI `0.147.0`。

### 4.1 Grok Build — 有完整 worktree 子系统

| 项 | 事实 |
| --- | --- |
| 开关 | `-w, --worktree [<NAME>]`；`--worktree-ref` / `--ref` 指定起点 |
| 管理 | `grok worktree list\|show\|rm\|gc`；另有 `db` 维护。跟踪库在 `~/.grok/worktrees.db` |
| 路径 | `~/.grok/worktrees/<repo>/<name>` |
| 行为 | 需要 git 仓；从当前 HEAD 起，**含未提交改动**；`--ref` 可要干净 checkout。结果是 detached 的真 checkout。headless `-p` **不会**因该 flag 建树 |
| 生命周期 | 结束 / 删除 session **不删** worktree；`gc` 只在用户调用时跑。`--max-age` 可清闲置且无进程占用的树 |
| resume | `grok -w -r <session-id>` 在**新** worktree 里恢复会话；远程 session 要 `--restore-code` 才把快照落到树里 |
| 配置 | `new_session_worktree_mode` / `fork_worktree_mode`：`ask \| always \| never` |

主源：本机 `grok --help` / `grok worktree --help`；https://docs.x.ai/build/features/worktrees ；https://docs.x.ai/build/cli/reference ；https://docs.x.ai/build/settings/reference

### 4.2 Claude Code — 有 `--worktree`、subagent `isolation: worktree`、桌面默认每会话一树

| 项 | 事实 |
| --- | --- |
| 开关 | `-w, --worktree [name]`；可与 `--tmux` 联用。省略 name 则生成如 `bright-running-fox` |
| 路径 / 分支 | 默认 `<repo>/.claude/worktrees/<name>/`，新分支 `worktree-<name>`。文档建议把 `.claude/worktrees/` 写进 `.gitignore` |
| 谁创建 | **CLI 自己**。也可会话内 `EnterWorktree` 工具；桌面应用「每个新 session 自动一棵树」 |
| gitignored | `.worktreeinclude`（gitignore 语法）**复制**同时被 ignore 的文件；不复制已跟踪文件 |
| 非 git | 默认要 git；可用 `WorktreeCreate` / `WorktreeRemove` hook 换成 SVN / Perforce / Hg。走 hook 时 **不处理** `.worktreeinclude` |
| 基线 | `worktree.baseRef`：`fresh`（默认，远程默认分支）或 `head`（当前 HEAD）。不能填任意分支名 |
| 清理 | 交互退出：干净且未命名 → 自动删树和分支；有改动或已命名 → 询问。`-p` 不清理。subagent / 后台 session 的树由 `cleanupPeriodDays` 定期扫，有未推送提交则跳过；运行中会 `git worktree lock` |
| 隔离强化 | 进树后禁止改主 checkout 的文件、禁止 Bash cwd 落到主仓、禁止 `git -C` / `GIT_DIR` 把 git 重定向回主仓 |
| resume | `--resume` / `--continue` 会回到原 worktree；树没了则在启动目录继续并清 binding。`--fork-session` 的 fork **不**继承原树 |

主源：本机 `claude --help`；https://code.claude.com/docs/en/worktrees ；https://code.claude.com/docs/en/sub-agents ；https://code.claude.com/docs/en/common-workflows

### 4.3 Codex — 无 `--worktree`；隔离面是 sandbox

| 项 | 事实 |
| --- | --- |
| worktree flag | **没有**。本机 `codex --help` / `codex exec --help` 与官方 CLI Reference 均无创建隔离 checkout 的开关 |
| 有的隔离 | `-s, --sandbox`：`read-only` / `workspace-write` / `danger-full-access`；另有 `--dangerously-bypass-approvals-and-sandbox` |
| 对已有 worktree | 认识 linked worktree：project hook / `.codex/` 从主 checkout 读，避免每棵树一份信任状态。提示词假定「可能处于 dirty worktree」 |
| resume | `codex resume` / `--last`，按 session id，与目录隔离无关 |

主源：本机 `codex --help`；https://developers.openai.com/codex/cli/reference ；https://github.com/openai/codex（`codex-rs/config` / `app-server/README.md` 对 linked worktree 的说明）

### 4.4 对 Adapter 的直接含义（避免双层）

| Agent | 壳再建 worktree？ | 再传 CLI 原生隔离？ |
| --- | --- | --- |
| Grok Build | 可以，但启动时 **不要** 再加 `--worktree` | 若走原生：壳只 `cd` 到 Project，把 `-w` / `--ref` 交给 CLI |
| Claude Code | 可以，但 **不要** 再加 `--worktree`，也不要让桌面式「每会话自动进树」与壳树叠两层 | 若走原生：由 Adapter 声明 `worktree` 字段并透传；`.worktreeinclude` 归项目约定，不归壳 |
| Codex | 若要目录隔离，**只能壳做**（或接受只做 sandbox、不隔离文件树） | 不要发明 `--worktree`；sandbox 是另一维度，解决不了「两个 Run 写同一文件」 |

---

## 5. 对照表

### 5.1 隔离形态

| 产品 | 复用主目录 | git worktree | 容器 / VM | 无目录隔离 |
| --- | --- | --- | --- | --- |
| cline/kanban | 仅 Home 侧栏 | **默认，每任务一树** | — | — |
| Vibe Kanban | — | **默认，每 Workspace 每仓一树** | Cloud 部署是另一路 | 非 git 直接拒绝 |
| Routa | **默认 Session cwd** | 手动按需 | 可选 sandbox / Docker（正交） | 不建树时就是共享目录 |
| OpenHands | 用户选了文件夹 → `local_repo`；非 git / 创建失败也回落这里 | 未选文件夹默认 `new_worktree`；子会话默认 worktree | Docker / Cloud sandbox | 无沙箱本机安装 = 整盘 |
| Zed | **默认** | 用户从 picker 建 linked worktree | — | External Agent / Terminal Thread 无 OS sandbox |
| Nimbalyst | Regular session | 可选 Worktree session | — | — |
| KanVibe | 无分支名 / `createSessionWithoutWorktree` | 有分支名则建 | — | — |
| Grok CLI | 默认 | `--worktree` | sandbox profile 正交 | — |
| Claude CLI | 默认（桌面除外） | `--worktree` / `isolation: worktree` | 自有 sandbox 正交 | — |
| Codex CLI | 默认（cwd） | 无创建开关 | **sandbox 是主隔离** | 目录级不隔离 |

### 5.2 必须钉死的操作细节

| 产品 | 创建者 | 清理者 | 非 git | gitignored | 分支名 | resume id | 端口 / DB |
| --- | --- | --- | --- | --- | --- | --- | --- |
| cline/kanban | 壳，点 Play | 壳，进 Trash（先打 patch） | 拒绝或 `git init` + 需要首 commit | symlink（Turbopack 的 node_modules 除外） | detached，无长期分支 | 最近一次 CLI 会话 + patch | 不处理 |
| Vibe Kanban | 壳，建 Workspace | 壳，归档 / 删除 / 孤儿扫描 | 必须是 git | `copy_files` 复制 | `{prefix}/{4hex}-{slug}` | Workspace / Session UUID | 用户 `dev_server_script` |
| Routa | 壳，手动 API | 壳，DELETE / 级联 | 无专门降级 | 无 | `wt/<label\|id8>` | worktree.sessionId | 不处理（另有 sandbox） |
| OpenHands | Agent Server | Server 重建前 force remove | 静默留在原目录 | 无 | `openhands/<uuid>` | conversation UUID | runtime_services URL；Docker 挂载 |
| Zed | 编辑器 picker | 归档且无占用则删（仅 Zed 所建） | 隔离不可用 | Task hook 自行拷 | 先 detached，再用户选分支 | Thread id；恢复可重建树 | 不处理 |
| Nimbalyst | 壳，New Worktree | 壳，delete / archive | 抛错 | 无 | `worktree/<name>` | `ai_sessions.id` + `worktree_id` | 不处理 |
| KanVibe | 壳，建卡时 | **删除任务**（DONE 不清） | 要求 git | 无 | 用户分支名 | task + tmux/zellij 名 | 不处理 |
| Grok CLI | CLI | 用户 `worktree rm/gc` | 需要 git | 从当前 HEAD 带未提交；不单独处理 ignore 目录 | detached | session id | 不处理 |
| Claude CLI | CLI | 退出询问 / 定期扫（视命名与是否干净） | hook 换 VCS | `.worktreeinclude` 复制 | `worktree-<name>` | session id，resume 回原树 | 不处理 |
| Codex CLI | — | — | cwd 任意 | — | — | `codex resume` | sandbox，不是分端口 |

### 5.3 和 Taskboard 已决规则的摩擦

| 实践 | 可借？ | 原因 |
| --- | --- | --- |
| 默认复用主目录，隔离显式（Zed / Nimbalyst / Routa） | 可借 | 符合 v1 人工分派、research / grilling 常只读 |
| 每任务必建树（cline / Vibe） | 慎借 | 与「v1 不做自动编排」气质冲突；非 git 痛；和 Grok/Claude `--worktree` 易双层 |
| 看板 DONE / Issue 关闭就删树（部分 README） | **不借** | 与 #9「Issue 关闭不杀 Run」冲突；KanVibe 源码其实已拆开 |
| 控制面不建树、执行面自建（OpenHands / Grok / Claude） | 部分可借 | Taskboard 的执行面是官方 TUI：应 **透传** 而不是再写一个 Agent Server |
| symlink 全部 gitignored（cline） | 慎借 | Turbopack / 改依赖会踩坑；官方 Claude 选择白名单复制 |
| 用户 `copy_files` / `.worktreeinclude` / Task hook | 可借 | 小而明确，不假装智能 |
| 容器 / VM（OpenHands Docker / Cloud） | v1 不借 | 超出 Embedded Terminal + 本机 CLI |
| Codex sandbox 当目录隔离 | 不够 | 挡的是越权写盘，不是两个 Run 抢同一文件 |
| 自动分端口 / 管本地 DB | 各家都没做 | v1 只警告，不实现 |

---

## 6. 给 #16 的选项菜单

以下只是 grilling 菜单。**不关 #16，不写 ADR。** 推荐项放第 1 条。

### 选项 1（推荐倾向）：默认复用 Project 主目录；隔离是 Run 级显式选择；Grok / Claude 与壳 worktree 互斥

**做法**

- 启动 Run 的默认 cwd = Project 绑定的本地工作区。
- 表单增加可选「隔离执行目录」：
  - Grok / Claude：**优先透传** `--worktree`（及 Grok 的 `--worktree-ref`），壳不 `git worktree add`。
  - Codex / 无原生开关的 Adapter：由壳建一棵 worktree，再在该 cwd 启动 CLI，**不**伪造 `--worktree`。
- 同一 Run 禁止「壳树 + CLI `--worktree`」同时开启。
- 非 git：隐藏或禁用隔离，退回主目录；不自动 `git init`。
- 清理：不随 Issue 关闭、不随 Run `ended` 自动删树（对齐 #9）。提供显式「删除该 Run 的隔离目录」和启动时的 stale prune。Grok 树交给 `grok worktree rm/gc`。
- resume：继续 = 新 Run（#9 已决）；若 Adapter 拿得到原生 session id，则在**同一隔离策略**下恢复会话。
- gitignored / 端口：v1 不做自动 symlink 全家桶；文档提示用 Claude `.worktreeinclude`、Zed hook 或项目自己的 setup。端口 / DB 只警告「并行 Run 会抢」。

**推荐理由**

- 与种子项目里「可选隔离」一档（Zed / Nimbalyst / Routa）一致，也与 Grok / Claude 官方模型兼容。
- research / grilling 类 Issue 多数只读，不必为每次 Run 复制整棵树。
- 把「谁建树」说死，避免双层 worktree——这是本票必须钉住的 CLI 事实。
- 清理与 Issue 生命周期解耦，不违反 #9。

**不选它的理由（grilling 时要能反驳）**

- 用户忘记开隔离时，两个实现类 Run 仍会在同一工作区互改文件。
- Adapter 要按 Agent 分叉（透传 vs 壳建），不能假装三家 CLI 同一个开关。

### 选项 2：每个 Run 一律由壳建 ephemeral worktree（cline / Vibe 模型）

**做法**：启动即 `git worktree add`；Agent 永远看不到主目录；结束策略再议（Trash 打 patch 或归档删树）。

**不采纳倾向**

- Grok / Claude 再带 `--worktree` 会双层；要禁用原生开关，等于丢掉官方清理 / 隔离强化 / `.worktreeinclude`。
- 非 git Project 必须 init 或拒收，比选项 1 伤面。
- 一律建树接近自动编排，和 v1「人工分派、能看见过程」不一致。
- 仍解决不了端口 / DB。

### 选项 3：永远复用主目录，隔离只靠 CLI sandbox / 权限模式

**做法**：所有 Run `cd` Project；Grok `--sandbox`、Codex `-s`、Claude permission mode 交给 Adapter。

**不采纳倾向**

- sandbox 限制的是「能不能写出项目外 / 能不能上网」，不是「两个 Agent 写同一文件」。
- 无法回答 #16 的问题：同一 Project 内并行 Run 如何避免互相破坏。
- Codex `workspace-write` 仍是同一棵工作树。

### 选项 4：容器或 VM（OpenHands Docker / Cloud）

**做法**：每个 Run 一个容器，挂载或克隆 Project。

**不采纳倾向**

- 与 Embedded Terminal 跑本机官方 TUI 的北极星冲突。
- 凭证、文件系统、GPU / 本机工具链都会变重，不像个人本地优先的 v1。
- 可作为更晚的信任边界扩展，不是这次目录策略。

### 选项 5：引入 Workspace 对象（Vibe 模型）——一 Issue 一 Workspace，内含多仓 worktree + dev server

**做法**：Run 不直接绑目录，先建 Workspace。

**不采纳倾向**

- 多一个与 Project / Run 平行的概念，v1 不需要。
- Vibe 本身在 sunsetting；其「自动 setup / 自动 dev server」是编排。
- 与「Run 绑定 Issue」相比过重。

### 选项 6（可与 1 组合的细则，不单独成策略）：按 Issue 类型给不同默认值

**做法**：research / grilling 默认主目录；实现类 Issue 默认隔离。

**单独拿来当主策略的问题**

- 种子项目几乎都不看「票类型」，只看「这是不是一次可能写文件的会话」。
- wayfinder 的 research 也可能改文档仓；grilling 也可能改 CONTEXT。用类型一刀切会误伤。
- 更稳的是选项 1 的「用户显式选择」，再在 UI 上对实现类 Issue **预勾**隔离（仍可取消）。

---

## 7. 失败恢复与 resume 清单（供 #16 拍板时对照）

| 失败模式 | 各家怎么做 | v1 建议记在选项 1 下 |
| --- | --- | --- |
| `worktree add` 报 already registered | cline / Vibe / OpenHands 都先 `prune` 或 force remove | 壳建树前 `git worktree prune` |
| 半截目录 | Vibe comprehensive cleanup；Nimbalyst 创建失败 rm 目录 | 创建失败必须删残留路径 |
| DB 有记录盘上没有 | Routa validate → `error`；Vibe ensure 重建；Zed 恢复线程时重建 | 以盘为准或一键重建，不要假装还在 |
| 进 Trash / 归档后还想继续 | cline 打 patch 再 apply；Zed 恢复线程重建树；Claude resume 回原树，树没了就在启动目录继续 | 继续 = 新 Run；能拿到原生 session id 就恢复会话，树按「当时是否隔离」重建或放弃 |
| 主仓脏导致 add 失败 | Vibe 文档列为常见失败 | 壳建树应用 commit-ish，不要依赖干净工作区；或改走 CLI `--worktree`（Grok 会带上未提交） |
| 双层 worktree | 无产品有意这么做 | Adapter 启动参数表互斥 |

---

## 8. 明确不在本票范围

- 不决定 #16；不写 ADR。
- 不讨论自动把 Run 结果 merge / 开 PR 的编排（cline auto-commit、Vibe 一键 PR、Claude 后台 session 自动 draft PR）。
- 不把 OpenHands Agent Server 或 Docker 沙箱设计进 v1。
- 不修改 map issue #1，不另开票。
