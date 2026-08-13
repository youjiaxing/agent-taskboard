# 调研：Agent 看板类产品的工作单元与编排模型

- **Ticket**: [#17](https://github.com/youjiaxing/agent-taskboard/issues/17)
- **Branch**: `research/agent-kanban-models`
- **Date**: 2026-08-14
- **Scope**: 高信任开源主源（官方仓库 README / 产品文档 / 源码类型与编排路径）。二手榜单只作发现入口。工作单元结论见 §2–§4.4；**CLI 对接、执行状态、过程展示、人机交互**见 §3.4 与 §4.5。
- **北极星对齐**: Issue 态势为主、Agent 执行为强配套；工作单位是 Issue Tracker 上的 Issue；Embedded Terminal 跑官方 Agent CLI TUI；v1 不做自动认领 / 自动串行跑 Frontier；个人本地，无账号多租户。
- **与 [#2](https://github.com/youjiaxing/agent-taskboard/issues/2) 的分工**: IA 票已回答面板划分 / 导航 / 多会话壳（文稿 `research/layout-ia` → `docs/research/layout-ia.md`）。本票只补 **工作单元、派活、与外部 Tracker 的关系、审阅/人闸**。不重做 IA 总表。

---

## 1. 问题与筛选标准

### 1.1 问题

有哪些高信任、可参考的「Agent 看板 / 并行 Agent 工作台」开源产品？

1. **工作单元**是什么（自建卡片、外部 Issue、Session、Spec、Conversation）？
2. **如何把工作交给 Agent**（官方 CLI TUI、自研聊天、ACP、自动流水线）？
3. **卡片/任务与外部 Issue Tracker** 是什么关系？
4. **审阅面**（diff / 证据 / 人闸）怎么放？
5. 哪些模式贴合 Agent Taskboard，哪些应明确拒绝？
6. 产品怎么对接 Agent CLI（官方 TUI PTY / ACP / 包一层聊天 / hooks / stdin）？跑到一半的第二句 prompt、权限询问怎么送进去？壳不重写 Agent 的话，怎么知道还活着 / 在等人 / 已退出？
7. 执行状态机挂在哪（卡片 / Session / Run）？列表/板上不点开能看见什么？
8. 执行过程怎么给看（整份官方 TUI / 摘要转录 / 卡片上的 hook 一行 / 终端+diff 分屏 / 时间线）？
9. 人怎么介入（启动；中途暂停/注入/行评/批准；结束：人标完成 / 退出码 / 自动 commit / gate）？

### 1.2 筛选

| 维度 | 要求 |
| --- | --- |
| 源 | 官方 README / 官方 docs / 仓库源码与类型定义（非二手榜单作证据） |
| 相关能力 | 至少覆盖「看板或任务板 / 并行 Agent / 与 Tracker 或 git 的交接」中的 **2 项** |
| 形态 | 桌面、本地 Web、self-host 或本地 TUI 均可；已停更但模型清楚的也可作卡片 |

**发现入口（不作结论）**: [andyrewlee/awesome-agent-orchestrators](https://github.com/andyrewlee/awesome-agent-orchestrators)。本票在种子之外增补 4 个高信任对照：Paperclip（自动认领反例）、Claude Squad（官方 TUI + worktree）、Emdash（外部 Issue 挂到本地 Task）、Cyrus（外部 Issue 指派即开跑）。

**明确降权**: 纯 Ralph 循环 runner、无看板也无 Tracker 的 multiplexer、团队 SaaS everything-app。OpenHands 在 IA 中已作 Chat-home 反例；本票只从其工作单元与编排再读。

### 1.3 与本产品的能力映射

| Agent Taskboard 概念 | 本调研中的对照 |
| --- | --- |
| Project | 本地仓库 / workspace / 已登记 repo |
| Issue Tracker / Tracker Adapter | GitHub / Linear / 自建 SQLite / markdown tracker |
| Issue | 外部 Issue、自建卡片、Spec/story、内部 ticket |
| Dependency / Frontier | 卡片依赖、blocker、lane 解锁 |
| Agent / Agent Adapter | CLI 探测、ACP、SDK、hooks |
| Run | session / workspace / heartbeat run / conversation |
| Embedded Terminal | 官方 CLI TUI 的真实 PTY vs 自研聊天包一层 |

已钉、本票不再重开：[Run 生命周期与 Issue 绑定](https://github.com/youjiaxing/agent-taskboard/issues/9) — Run 通常绑一个 Issue、可游离；同一 Issue 同时最多一个活跃 Run；Run 与 Issue 状态独立；v1 不自动推进 Issue。

---

## 2. 项目卡片（主源摘录）

### 2.1 Claude Code Board — Session 为根 + 自研 Web 聊天包 Claude Code

| 项 | 内容 |
| --- | --- |
| 形态 | Windows 本地 Web（frontend + backend），无账号 |
| 仓库 | https://github.com/cablate/Claude-Code-Board （~153★，MIT） |
| 工作单元 | **Session**。可选挂到自建 **Work Item**（`planning / in_progress / completed / cancelled`） |
| 派活面 | 自研聊天：`npx @anthropic-ai/claude-code` + `-p --output-format=stream-json`，WebSocket 回灌 UI；**不**展示官方 TUI |
| Tracker 关系 | 无外部 Issue Tracker。Work Item 是本地第二套任务库 |
| 审阅 / 人闸 | 会话消息过滤与导出；无 diff 人闸 |
| 并行隔离 | 多 Session 各有 `workingDir`；无 worktree 隔离 |
| 维护状态 | README 置顶 **不再积极维护 / archived**；GitHub `archived=false`，最后 push 2026-01-28 |

**编排要点**

- 用户在 Web 里 New Session → 选目录、Workflow Stage、Agent、可选 Work Item。
- `ProcessManager` 明确用 stream-json 解析结构化输出，把官方 CLI 降成 headless 子进程。
- Work Item 只是「把多个 Session 归到一个本地任务下」，不是 Issue Tracker。

**对 Taskboard**

- 可抄：Session 可挂可不挂任务；跨 Session 续聊用原生 `claudeSessionId` / `--resume`。
- **应拒**：用 stream-json 自研聊天替代官方 TUI；用本地 Work Item 当工作单元。

主源：

- https://github.com/cablate/Claude-Code-Board/blob/master/README.md
- https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/types/session.types.ts
- https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/types/workitem.types.ts
- https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/services/ProcessManager.ts

---

### 2.2 Routa — Workspace 卡片 + 专家泳道自动流转 + Review Gate

| 项 | 内容 |
| --- | --- |
| 形态 | Tauri 桌面 + Next.js Web + CLI；双后端共用 `api-contract.yaml` |
| 仓库 | https://github.com/phodal/routa （~1.8k★，MIT，2026-08 仍活跃） |
| 工作单元 | Workspace 范围内的 **看板卡片**。卡片随泳道长出 YAML story → execution brief → Dev Evidence → Review Findings。另有独立 **Session** 作即兴线程 |
| 派活面 | 泳道切换触发 specialist session（ACP / MCP / 适配器）。**不是**官方 CLI TUI |
| Tracker 关系 | **自建看板为 SoT**。GitHub Issue 可列表/创建，并可 **镜像** 成本地 markdown（`kind: github_mirror`），不是 Tracker 一等 |
| 审阅 / 人闸 | 分层 Review Gate：Harness Monitor（发生了什么）→ Entrix Fitness（硬门闩/证据）→ Gate Specialist（验收条款，Done / 打回 Dev / 升级人工） |
| 并行隔离 | 一等 worktree / codebase 对象；会话与卡片可并行 |
| 维护状态 | 活跃（push 2026-08-13） |

**编排要点**

```
目标自然语言 → Workspace 卡片
Backlog Refiner → Todo Orchestrator → Dev Crafter → Review Guard → Done Reporter
                                         ↘ Blocked Resolver
```

每列有独立 prompt 合同与证据合同；下游故意不信任上游。ROUTA 协调、CRAFTER 实现、GATE 验证。

**对 Taskboard**

- 可抄：**证据写在工作对象上**，而不是埋进聊天；Review 是门闩不是又一个聊天角色。
- **应拒**：专家自动流转整条交付管道；把自建卡片当 SoT；用 ACP/会话 UI 替代官方 TUI。
- GitHub 镜像证明「外部 Issue → 本地第二份文档」是双写。v1 不要走这条路。

主源：

- https://github.com/phodal/routa/blob/main/README.md
- https://github.com/phodal/routa/blob/main/docs/core-concepts/how-routa-works.md
- https://github.com/phodal/routa/blob/main/docs/use-routa/kanban.md
- https://github.com/phodal/routa/blob/main/src/core/kanban/github-issues.ts
- https://github.com/phodal/routa/blob/main/src/core/github/github-issue-sync.ts

---

### 2.3 Cline Kanban — 自建卡片 + 官方 CLI PTY + 依赖完成即开跑

| 项 | 内容 |
| --- | --- |
| 形态 | `npx kanban` 本地 Web；Research Preview；无账号 |
| 仓库 | https://github.com/cline/kanban （~1.3k★，Apache-2.0，2026-08 仍活跃） |
| 工作单元 | 自建 **Task card**（prompt + baseRef + 审阅设置）。落在 `~/.cline/kanban/workspaces/*/board.json` |
| 派活面 | 多数 Agent = **PTY 里跑官方 CLI TUI**；Cline 自己走 SDK 原生聊天。点卡片 play 启动 |
| Tracker 关系 | 无外部 Issue Tracker。GitHub 只作 **Commit / Open PR** 出口 |
| 审阅 / 人闸 | 卡片详情 = Agent TUI + worktree diff；行级评论回投 Agent。可选 `autoReviewEnabled` → 自动 commit/PR 后进 Done |
| 并行隔离 | 每卡 ephemeral worktree；gitignore 目录（如 `node_modules`）symlink |
| 维护状态 | 活跃（Research Preview；CHANGELOG 到 0.1.70） |

**编排要点**

- 列：Backlog → In Progress → Review → Done（曾名 Trash）。
- `⌘+click` 建依赖。卡片从 Review **移到 Done** 后，已解锁的 Backlog 链接卡 **自动 kickoff**（`useLinkedBacklogTaskActions` + `trackTasksAutoStartedFromDependency`）。
- 依赖至少一端必须在 Backlog；Done 中的卡不能再链。
- 架构明文：浏览器是控制面，本地 runtime 是真相；PTY 路径是「经典 Kanban」。

**对 Taskboard**

- **最值得抄的派活面**：人工点 play → 官方 CLI 进真实 PTY → 同屏看 diff。
- 可抄：Dependency 解锁语义；审阅用 worktree diff + 行评。
- **应拒**：自建 `board.json` 当 Issue；依赖完成自动开跑；auto-commit / auto-PR 跳过人工。

主源：

- https://github.com/cline/kanban/blob/main/README.md
- https://github.com/cline/kanban/blob/main/docs/architecture.md
- https://github.com/cline/kanban/blob/main/src/core/task-board-mutations.ts
- https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-linked-backlog-task-actions.ts
- https://github.com/cline/kanban/blob/main/web-ui/src/hooks/use-review-auto-actions.ts

---

### 2.4 OpenHands Agent Canvas — Conversation / Automation 控制面

| 项 | 内容 |
| --- | --- |
| 形态 | 浏览器控制面 + 可切换 Agent Server（本机 / Docker / VM / Cloud） |
| 仓库 | https://github.com/OpenHands/OpenHands （~84k★，MIT，2026-08 仍活跃） |
| 工作单元 | **Conversation**（绑 backend + workspace）。**Automation** 是定时/事件触发的另一条控制面 |
| 派活面 | Canvas 自研对话 UI。第三方 Agent 经 **ACP**（stdio JSON-RPC）由 Agent Server 拉起 CLI 子进程，**渲染的是 Canvas 聊天，不是官方 TUI**。Automation Server 按 cron / webhook `dispatch` |
| Tracker 关系 | GitHub / Linear / Slack 是 **触发源与集成**，不是板上的一等 Issue。预置自动化含「把 GitHub Issue 拆成任务」；精选 responder 含 `github-pr-reviewer`、`github-repo-monitor` |
| 审阅 / 人闸 | conversation + files + **diff-viewer** + browser；无「Issue 完成」人闸。Automation 有 run 状态与日志 |
| 并行隔离 | 隔离在 backend / sandbox，不在「每张 Issue 一个 worktree」模型上 |
| 维护状态 | 活跃；Automation 服务单独仓 `OpenHands/automation`（Beta） |

**编排要点**

- 前端职责：渲染 conversation / terminal / browser / files / settings / automations；**不**执行 agent、**不**提供沙箱。
- 启动预置自动化 = 新建 Conversation，把 skill 触发命令交给 Agent。
- Issue 进系统的方式是 webhook / 用户在聊天里让 Agent 去拆，不是 Tracker Adapter 拉列表。

**对 Taskboard**

- 可抄：控制面 vs 执行后端分离（IA 已记）；完成信号与调度是未来编排器的事，不是 v1。
- **应拒**（与 #2 一致）：Conversation 当首页和工作单元；ACP/自研聊天替代官方 TUI；把外部 Issue 只当触发器。

主源：

- https://github.com/OpenHands/OpenHands/blob/main/README.md
- https://github.com/OpenHands/OpenHands/blob/main/docs/architecture.md
- https://github.com/OpenHands/OpenHands/blob/main/docs/ACP_AGENTS.md
- https://github.com/OpenHands/automation/blob/main/README.md
- https://github.com/OpenHands/OpenHands/blob/main/src/manifests/automation-interface.ts

---

### 2.5 Vibe Kanban — 自建 Issue + Workspace 执行室（日落中）

| 项 | 内容 |
| --- | --- |
| 形态 | `npx vibe-kanban` 本地/可 self-host；可选 GitHub/Google 登录开团队板 |
| 仓库 | https://github.com/BloopAI/vibe-kanban （~28k★，Apache-2.0） |
| 工作单元 | 自建 **kanban Issue**（描述即 Agent prompt）。执行单元是 **Workspace**（可挂多仓、多 Session） |
| 派活面 | 创建 Workspace 即开 Agent；主交互是 **自研聊天**。有集成终端，但是辅助 |
| Tracker 关系 | **自建 Issue 为 SoT**。GitHub 只经 `gh` 开 PR。不登录则「看板/Issue/团队不可用」，只剩本地 Workspace |
| 审阅 / 人闸 | Changes panel：unified / side-by-side diff + 行评，随下一条聊天发给 Agent；再 `Create PR` |
| 并行隔离 | 每 Workspace 一个 git worktree + 工作分支（`vk/...`） |
| 维护状态 | **公司日落**（2026-04-10 公告）。远程 kanban Issue / 评论 / 项目 / org 将撤；本地 Workspace 保留。源码仓最后 push 2026-04-24；awesome 列表将其标为 resting |

**编排要点**

- 一张 Issue 可连 **多个 Workspace**（同一功能并行多 Agent）。
- 子 Issue 状态独立，子全完成 **不** 自动完成父 Issue。
- 产品叙事把 Issue 当规划、Workspace 当执行——分离本身对，但 Issue 不是外部 Tracker。

**对 Taskboard**

- 可抄：**规划对象与执行环境分离**（对应本产品 Issue vs Run）；一行评收集后一次性回投。
- **应拒**：自建 Issue 云库；登录才能看板；聊天为执行主表面；把描述整份当 prompt（与 #9「只带定位、读 Tracker 最新」相反）。

主源：

- https://github.com/BloopAI/vibe-kanban/blob/main/README.md
- https://github.com/BloopAI/vibe-kanban/blob/main/docs/issue-management.mdx
- https://github.com/BloopAI/vibe-kanban/blob/main/docs/workspaces/index.mdx
- https://github.com/BloopAI/vibe-kanban/blob/main/docs/reviewing-code.mdx
- https://github.com/BloopAI/vibe-kanban/blob/main/docs/integrations/github-integration.mdx
- https://www.vibekanban.com/blog/shutdown

---

### 2.6 Nimbalyst — Session 看板 + 本地 Tracker 双层

| 项 | 内容 |
| --- | --- |
| 形态 | Electron 桌面 + 手机伴侣；MIT；有可选团队同步服务 |
| 仓库 | https://github.com/Nimbalyst/nimbalyst （~1.5k★，2026-08 仍活跃） |
| 工作单元 | **两层**：① Session（看板列是 session 阶段）；② 本地 **Tracker item**（Plans / Bugs / Tasks / Features…，YAML schema，markdown 落盘） |
| 派活面 | 自研 Agent Mode（Claude Code / Codex SDK + MCP）。有 embedded Ghostty 终端，但是开发者配套，不是「官方 TUI = 执行主表面」 |
| Tracker 关系 | 本地 markdown tracker 为 SoT。GitHub Issues 经扩展 **导入** 成本地 item，带回链与 re-snapshot。PR 与 tracker 是引用关系，不是 GitHub Issue 一等 |
| 审阅 / 人闸 | 红绿 WYSIWYG / 每文件 approve-reject；`in-review` 车道：**Agent 可送审，只有人能批准**。另有 GitHub PR 视图（`gh`，不存 token） |
| 并行隔离 | 可选 worktree；一 worktree 可多 Session；Blitz / Super Loop 各有隔离策略 |
| 维护状态 | 活跃 |

**编排要点**

- Session 列：backlog / planning / implementing / validating / complete。
- Tracker 是通用对象系统（连角色、菜谱都能建），Agent 经 MCP 增删改跑。
- 打开 worktree 会自动把 session 链到引用它的 tracker item。

**对 Taskboard**

- 可抄：**人闸不可被 Agent 关闭**；PR ↔ 工作项 ↔ 执行会话三跳；本地 markdown 作为 **后续** Tracker Adapter 的参考，不是 v1 GitHub 主路径。
- **应拒**：再造一套本地 tracker 与 GitHub Issue 双写；Session 阶段看板当主态势；自研 Agent Mode 替代官方 TUI。

主源：

- https://github.com/Nimbalyst/nimbalyst/blob/main/README.md
- https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- https://github.com/Nimbalyst/nimbalyst/blob/main/UserDocs/creating-custom-trackers.md

---

### 2.7 Claude Code Kanban — 只观察、不指挥

| 项 | 内容 |
| --- | --- |
| 形态 | `npx claude-code-kanban` 本地 Web + SSE |
| 仓库 | https://github.com/NikiforovAll/claude-code-kanban （~44★，MIT，2026-08 仍有提交） |
| 工作单元 | Claude Code 自己写到 `~/.claude` 的 **task 文件 + session jsonl**。看板列 = Pending / In Progress / Completed |
| 派活面 | **无派活**。用户照常在官方 CLI 里干活；仪表盘只监视。README 原句：*It never directs Claude's work.* |
| Tracker 关系 | 无。任务来自 Claude 本地 task 目录 / self-team |
| 审阅 / 人闸 | 无代码审阅。有 waiting-for-user、blockedBy/blocks（Claude 任务字段）、session/agent log |
| 并行隔离 | 无；隔离是 Claude 自己的 session/cwd |
| 维护状态 | 小项目、活跃 |

**编排要点**

- hooks 安装后才有 agent log / 等待指示；否则只见任务文件。
- 发现是 chokidar 事件驱动，不是轮询全盘。

**对 Taskboard**

- 可抄：**观察层不得变成编排器**。v1 看 Run 状态可以用 hooks / 退出码，但不要据此认领或关 Issue。
- **应拒**：把 Claude 内部 todo 列表当成 Issue Tracker。

主源：

- https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md
- https://github.com/NikiforovAll/claude-code-kanban/blob/main/docs/session-scanning.md

---

### 2.8 KanVibe — 分支即任务 + 官方 CLI 进 tmux + hook 推列

| 项 | 内容 |
| --- | --- |
| 形态 | Electron / 浏览器；键盘优先 |
| 仓库 | https://github.com/rookedsysc/kanvibe （~138★，AGPL-3.0，2026-08 仍活跃） |
| 工作单元 | 自建 **branch TODO**（SQLite）。登记 Project = 本地/远程 git 仓 |
| 派活面 | 建任务时准备 worktree + **tmux/zellij** 窗，浏览器 xterm 挂上。侧栏另有 AI chat。Agent 是官方 CLI，但会话在 multiplexer 里 |
| Tracker 关系 | 无 Issue Tracker 一等。`gh` 用于 PR；卡片上有 PR badge。DONE 时 **自动删** branch / worktree / 终端 |
| 审阅 / 人闸 | 列 REVIEW = Agent 停手等人。GitHub 风格 diff（Monaco）。钩子把「Agent 说完」映射成 REVIEW，不是验收通过 |
| 并行隔离 | 一任务一 worktree + 一 multiplexer session |
| 维护状态 | 活跃 |

**编排要点**

- 列：TODO → PROGRESS → PENDING → REVIEW → DONE。
- Hooks 按 Agent 生命周期推列（Claude AskUser → PENDING；Codex PermissionRequest → PENDING；OpenCode `session.idle` → REVIEW）。
- 扫目录可把已有 worktree 收成 TODO。

**对 Taskboard**

- 可抄：hook 只反映 **Run 忙闲**（等人 / 在跑 / 停了），不表示 Issue 完成；一任务一终端工作区。
- **应拒**：branch 名当工作单元；DONE 自动删隔离区；hook 自动推列当成完成语义。

主源：

- https://github.com/rookedsysc/kanvibe/blob/main/README.md

---

### 2.9 Paperclip — 内部 Issue + 心跳自动认领（v1 反例）

| 项 | 内容 |
| --- | --- |
| 形态 | 自托管控制面（可 trusted-local）；多 company |
| 仓库 | https://github.com/paperclipai/paperclip （~78k★，MIT，2026-08 仍活跃） |
| 工作单元 | 控制面 **Issue**（`ENG-123` 式 identifier，父/子、blocker、单 assignee）。**不是** GitHub Issue。路线图：Bring-your-own-ticket-system 仍未做 |
| 派活面 | **Heartbeat**：定时 / 指派 / @提及 / 人工 Invoke。Agent 醒来后 **atomic checkout** 认领，`in_progress` 必须有执行锁。适配器拉起 Claude Code / Codex / webhook 等 |
| Tracker 关系 | Paperclip 自己就是 Tracker。明确「不是代码评审工具」「不要做完整 Jira/GitHub 替代」 |
| 审阅 / 人闸 | `in_review`、board 审批（雇人、CEO 战略）、预算硬停。产品说验证靠 diffs / 截图 / 测试，但核心不是 PR 工具 |
| 并行隔离 | project workspace + 执行 worktree / operator branch |
| 维护状态 | 极活跃 |

**编排要点**

- 四件套分开：结构（parent）≠ 依赖（blocker）≠ 所有权（assignee）≠ 执行（checkout/run）。
- 两个 Agent 同时 checkout → `409`，禁止重试。
- 未配置密钥在 dispatch 前变成 blocker，不启动注定失败的 run。

**对 Taskboard**

- 可抄（语义，不是产品）：结构 / Dependency / 认领 / Run 四套关系不要揉成一个字段。这与本产品 Issue / Dependency / assignee / Run 独立已经同向。
- **应拒（v1 硬拒）**：心跳自动认领；Agent 自己 pick work；自建公司级票系统；多租户 org chart / 预算 / 账号。这就是「自动跑 Frontier」的完整形态。

主源：

- https://github.com/paperclipai/paperclip/blob/master/README.md
- https://github.com/paperclipai/paperclip/blob/master/docs/start/core-concepts.md
- https://github.com/paperclipai/paperclip/blob/master/docs/api/issues.md
- https://github.com/paperclipai/paperclip/blob/master/doc/execution-semantics.md
- https://github.com/paperclipai/paperclip/blob/master/doc/PRODUCT.md

---

### 2.10 Claude Squad — 实例/任务 = 官方 TUI + worktree

| 项 | 内容 |
| --- | --- |
| 形态 | 本地 TUI（`cs`）；依赖 tmux + `gh` |
| 仓库 | https://github.com/smtg-ai/claude-squad （~8.3k★，AGPL-3.0，2026-07 仍有提交） |
| 工作单元 | **Session / instance**（一条「任务」= 一个后台 Agent 实例）。无 Issue 对象 |
| 派活面 | `n` / `N` 新建；**attach 进官方 CLI TUI**（tmux）。`-y` 可 yolo 自动接受 |
| Tracker 关系 | 无 Issue Tracker。`s` 提交并 push 分支；审阅在进 GitHub 之前 |
| 审阅 / 人闸 | preview tab + **diff tab**；`c` checkout（提交并暂停）后才推进 |
| 并行隔离 | 每实例独立 tmux + **git worktree / 自有分支** |
| 维护状态 | 活跃 |

**对 Taskboard**

- 可抄：派活 = 拉起官方程序；人进 TUI；diff 是审阅 tab 不是聊天。这是 Embedded Terminal 的最小模型。
- **应拒**：用 session 列表当 Issue 态势；无 Tracker 绑定。

主源：

- https://github.com/smtg-ai/claude-squad/blob/main/README.md

---

### 2.11 Emdash — 本地 Task 为执行单元，外部 Issue 可选挂靠

| 项 | 内容 |
| --- | --- |
| 形态 | Electron 桌面；local-first SQLite；YC W26 |
| 仓库 | https://github.com/generalaction/emdash （~5.4k★，Apache-2.0，2026-08 仍活跃） |
| 工作单元 | 桌面 **Task**（执行工作单元，链到 Workspace）。**Conversation** 是可恢复的对话记录；**Session** 是活进程。外部 Issue 是 **LinkedIssue** |
| 派活面 | 探测本机 CLI；hooks 写用户级配置以跟踪状态。同时有 **ACP 聊天** 与终端 Session |
| Tracker 关系 | 插件可读 Linear / GitHub / Jira / GitLab / Asana / …。创建 Task 时可 **选择并挂靠** 外部 Issue（`linkedIssue` 列）。Task 仍是桌面自有对象，不是 Tracker 一等 |
| 审阅 / 人闸 | 产品级 diff / PR / CI / merge。Issue 不因 Task 结束而关闭（需看具体写回，主路径是挂靠而非双向同步） |
| 并行隔离 | 每 Task 独立 git worktree；可 SSH 远端 Host |
| 维护状态 | 活跃 |

**编排要点**

- CONTEXT 把 Host 文件系统当权威，桌面 Registry 只镜像——对「Tracker 是 Issue 权威、本地只缓存」有类比价值。
- `getIssueTaskName` 用外部 Issue 的 branchName 派生 Task 名，说明外部票是输入，不是板的主键。

**对 Taskboard**

- 可抄：外部 Issue **可选挂靠** 到执行对象；多 Tracker 用适配器。但本产品应把方向反过来：**Issue 是主键，Run 挂靠 Issue**，而不是 Task 主键、Issue 挂靠。
- 部分应拒：再做一层本地 Task 库；ACP 聊天与官方 TUI 双主表面。

主源：

- https://github.com/generalaction/emdash/blob/main/README.md
- https://github.com/generalaction/emdash/blob/main/CONTEXT.md
- https://github.com/generalaction/emdash/blob/main/apps/emdash-desktop/src/core/features/tasks/node/operations/updateLinkedIssue.ts
- https://github.com/generalaction/emdash/blob/main/apps/emdash-desktop/src/core/features/tasks/browser/create-task-modal/issue-combobox-field.tsx

---

### 2.12 Cyrus — 外部 Issue 指派即自动开跑

| 项 | 内容 |
| --- | --- |
| 形态 | 常驻进程（self-host / 付费托管）；webhook |
| 仓库 | https://github.com/cyrusagents/cyrus （~763★，Apache-2.0，2026-08 仍活跃） |
| 工作单元 | **外部 Tracker 上的 Issue**（Linear 为主，亦支持 GitHub / GitLab / Slack） |
| 派活面 | 监视「指派给 Cyrus 的 Issue」→ 自动建 worktree → 跑 Claude Code / Codex / Cursor / Gemini → 把活动 **流回 Tracker**（评论、下拉、审批） |
| Tracker 关系 | Tracker 是触发源 **且** 是写回面。路由：`routingLabels` > `projectKeys` > `teamKeys`。GitHub 主要用于 PR（`gh`） |
| 审阅 / 人闸 | Linear/GitHub 上的 agent session 交互（下拉、审批）。用户访问控制可静默丢弃或评论拒绝 |
| 并行隔离 | 每 Issue 一个 isolated worktree；仓克隆在 `~/.cyrus/repos/` |
| 维护状态 | 活跃 |

**对 Taskboard**

- 这是「工作单元 = 外部 Issue」里最彻底的实现，也是 **v1 最不该抄的编排**：指派即自动跑、自动写回、常驻 webhook、24/7 服务。
- 可抄的仅是：**路由规则按仓 / label**（多 Project 时「这张 Issue 属于哪个本地目录」），以及写回带 `generated-by-…` 标记。本产品写回应是可选人工动作（#9 已钉）。

主源：

- https://github.com/cyrusagents/cyrus/blob/main/README.md
- https://github.com/cyrusagents/cyrus/blob/main/docs/SELF_HOSTING.md
- https://github.com/cyrusagents/cyrus/blob/main/docs/CONFIG_FILE.md
- https://github.com/cyrusagents/cyrus/blob/main/docs/GIT_GITHUB.md

---

## 3. 横切对照表

### 3.1 工作单元 × 派活面 × Tracker 关系

| 项目 | 工作单元 | 派活面 | Tracker 关系 |
| --- | --- | --- | --- |
| Claude Code Board | Session（可选挂 Work Item） | 自研聊天 + Claude `stream-json` | 无；本地 Work Item |
| Routa | Workspace 卡片 + YAML Spec | 泳道自动化 / ACP session | 自建板为 SoT；GitHub 可镜像/导入 |
| Cline Kanban | 自建 Task card | **官方 CLI PTY**（Cline 走 SDK 聊天） | 无；GitHub 只出 PR |
| OpenHands | Conversation / Automation | Canvas 聊天 + ACP；cron/webhook dispatch | Issue 作触发源，非板上 SoT |
| Vibe Kanban | 自建 Issue + Workspace | 自研聊天（终端为辅） | 自建 Issue 为 SoT；`gh` 出 PR |
| Nimbalyst | Session + 本地 Tracker item | SDK/MCP Agent Mode | 本地 tracker 为 SoT；GitHub 导入 |
| Claude Code Kanban | Claude 本地 task/session | **不派活**（只观察官方 CLI） | 无 |
| KanVibe | 自建 branch 任务 | 官方 CLI 进 tmux/zellij + hook 推列 | 无；`gh` 出 PR |
| Paperclip | 内部控制面 Issue | Heartbeat + atomic checkout | 自己就是 Tracker；BYO tracker 未做 |
| Claude Squad | Session/instance | **官方 CLI TUI**（tmux attach） | 无；`gh` 推分支 |
| Emdash | 本地 Task（可挂 LinkedIssue） | CLI + ACP 聊天 | 外部 Issue **挂靠** 本地 Task |
| Cyrus | **外部 Tracker Issue** | 指派/webhook **自动**开跑 | Tracker 触发 + 写回 |

### 3.2 审阅 / 人闸 × 并行隔离

| 项目 | 审阅 / 人闸 | 隔离 |
| --- | --- | --- |
| Claude Code Board | 无 diff 人闸 | 多 Session 目录 |
| Routa | 证据合同 + Fitness 硬门 + Gate 人/自动升级 | worktree 一等对象 |
| Cline Kanban | TUI + worktree diff + 行评；可选自动 commit/PR | 每卡 worktree |
| OpenHands | 对话内 diff/files；无 Issue 完成门 | backend/sandbox |
| Vibe Kanban | Changes panel 行评回聊天；PR | 每 Workspace worktree |
| Nimbalyst | 红绿批准；Agent 不能批 `in-review` | 可选 worktree |
| Claude Code Kanban | 无代码审阅 | 无 |
| KanVibe | REVIEW 列 + Monaco diff；DONE 清场 | 每任务 worktree + mux |
| Paperclip | `in_review` / board 审批 / 预算 | 执行 worktree |
| Claude Squad | diff tab，人 checkout 后才推进 | 每实例 worktree |
| Emdash | diff / PR / CI / merge | 每 Task worktree |
| Cyrus | Tracker 上的审批交互 | 每 Issue worktree |

### 3.3 与北极星匹配（本票维度，不是 IA）

评分：✓ 直接可借 · ◐ 借局部 · ✗ 作主模型不合适

| 项目 | Issue 当工作单元 | 官方 CLI TUI 派活 | Tracker 非双写 | 人派活（非自动跑 Frontier） | 建议 |
| --- | --- | --- | --- | --- | --- |
| Cline Kanban | ✗ 自建卡 | ✓ PTY | ✗ 无 Tracker | ◐ 可手点；**依赖自动开跑** | **派活/审阅抄；编排拒** |
| Claude Squad | ✗ Session | ✓ TUI attach | ✗ 无 Tracker | ✓（除非 `-y`） | **派活最小模型** |
| Claude Code Kanban | ✗ Claude todo | ✓（用户自己开 CLI） | n/a | ✓ 不指挥 | **观察层** |
| KanVibe | ✗ branch 任务 | ◐ CLI 在 mux 里 | ✗ | ◐ hook 自动推列 | **hook=忙闲；勿自动清场** |
| Emdash | ◐ 本地 Task 主键 | ◐ CLI + ACP | ◐ 挂靠 | ✓ 人建 Task | **挂靠方向要反转** |
| Nimbalyst | ◐ 本地 tracker | ✗ Agent Mode | ✗ 导入双写 | ◐ Super Loop/Blitz 偏自动 | **人闸；勿双写** |
| Vibe Kanban | ✗ 自建 Issue | ✗ 聊天主表面 | ✗ | ✓ 人开 Workspace | **Issue/Run 分离可抄** |
| Routa | ✗ 自建卡/Spec | ✗ ACP/session | ✗ 镜像 | ✗ 泳道自动 | **证据门；拒自动流转** |
| OpenHands | ✗ Conversation | ✗ ACP 聊天 | ✗ 触发器 | ✗ Automation | **控制面/后端；拒 Chat-home** |
| Claude Code Board | ✗ Session | ✗ stream-json 聊天 | ✗ | ✓ 人开 Session | **反例** |
| Cyrus | ✓ 外部 Issue | ✗ 后台跑 CLI | ◐ 写回 Tracker | ✗ 指派即跑 | **工作单元对；编排拒** |
| Paperclip | ✗ 内部票 | ✗ heartbeat 适配器 | ✗ 自建 Tracker | ✗ 自动认领 | **v1 总反例** |

### 3.4 CLI 对接 × 执行状态 × 过程展示 × 人机交互

本小节只补四轴，不改 §3.1–§3.3 的工作单元结论。状态机若同时有「看板列」和「Run/Session」，表里写的是 **壳看见执行** 的那一层。

#### 对接方式（五种）

| 方式 | 代表 | 第二句 prompt / 权限询问 | 壳如何知道活着 / 等人 / 退出（不重写 Agent） |
| --- | --- | --- | --- |
| **A. 官方 CLI TUI 进真实 PTY** | Cline（非 Cline 引擎）、Claude Squad、KanVibe、Emdash TUI 路径 | 人在 TUI 里键入；权限也在官方 TUI。Squad `-y` / Cline 实验性 auto 权限会吞掉询问 | **进程**：pid / PTY exit。**语义忙闲**：须 hooks 或官方协议，不能靠扫终端字 |
| **B. Headless CLI / stream-json / SDK 包一层聊天** | Claude Code Board、Vibe、Cyrus、Paperclip 本地适配器 | 聊天框或 Tracker 评论再打一枪（Board=`sendMessage`+`--resume`；Cyrus=Linear 评论灌进 session） | 适配器看子进程退出码 + 自研会话状态；等人靠自研 UI 或 Tracker 卡片 |
| **C. ACP（stdio JSON-RPC）** | OpenHands、Routa、Emdash ACP 路径 | Canvas/会话再发 turn；权限走 ACP `pendingPermission` | 协议里的 generating / permission count / conversation status |
| **D. 只观察 hooks / 文件** | Claude Code Kanban | 板不送 prompt；人仍在官方 CLI 里打字。等人靠 hook 写 `_waiting.json` | 监视 `~/.claude` jsonl + agent-activity + live-session registry |
| **E. 心跳 / 列切换拉起进程** | Paperclip heartbeat、Routa 泳道、Cyrus 指派 | 中途不是 TUI，是评论 / Invoke / 再一次 wakeup | run：`queued/running/succeeded/failed/timed_out/cancelled`；与 Issue 列分开 |

#### 12 项目对照

| 项目 | CLI 对接 | 执行状态（挂在哪；板上不点开） | 过程怎么给看 | 人机：开 / 中 / 结 |
| --- | --- | --- | --- | --- |
| Claude Code Board | B：`npx claude-code -p --output-format=stream-json`，可 `--resume` / `--continue`；可加 `--dangerously-skip-permissions` | 挂 **Session**：`processing / idle / completed / error / interrupted / crashed`。仪表盘列表一眼看状态 | 自研 Web 聊天 + 过滤 tool_use/thinking；**无**官方 TUI | 开：New Session。中：聊天再发（新一轮进程）。结：标 completed/error；无 diff 完成门 |
| Routa | C + E：ACP/适配器 session（create / prompt / cancel / reconnect / stream / trace）；看板列切换排队开 session | 挂 **卡片泳道** + **Session** 对象。板上见列；session 可取消/重连 | 会话转录 + Harness traces + 卡片上越积越厚的证据，不是官方 TUI | 开：开 Session 或推列。中：再 prompt / cancel。结：Review/Done 的 Fitness+Gate（可升级人工） |
| Cline Kanban | A：`node-pty` 跑官方 CLI；WS 传输入/resize/exit。Cline 引擎走 SDK 聊天（B）。hooks：`to_review` / `to_in_progress` / `activity` | 挂 **Task session**：`idle / running / awaiting_review / failed / interrupted`，另有 `reviewReason`（attention/exit/error/interrupted/hook）+ `exitCode`。卡片上 hook 一行（最新消息/工具名）。列（Backlog/…）是板状态，不是进程 | **官方 TUI + worktree diff 分屏**；卡片上 hook 摘要，可扫上百 Agent | 开：play。中：TUI 键入；行评回投。结：人 Commit/PR，或 `autoReview` 自动 commit/PR 后进 Done；进程退出只到 `awaiting_review` |
| OpenHands | C：Agent Server spawn ACP 子进程；Canvas 聊天。Automation：`dispatch` | 挂 **Conversation**：`STARTING / RUNNING / STOPPED / PAUSED / AWAITING_USER_INPUT / FINISHED / ARCHIVED / ERROR`。会话列表有状态点/徽章。Automation 另有 run 状态 | 聊天 + 自研 terminal/files/**diff-viewer**/browser；**不是**官方 TUI | 开：新 Conversation / 点预置自动化。中：再发 turn。结：FINISHED/STOPPED；无 Issue 完成门 |
| Vibe Kanban | B 为主（自研聊天）+ 辅助集成终端。规划态有 Approval；可配 `dangerously_skip_permissions` | 挂 **Workspace**：Running / Idle；「Needs Attention」举手。侧栏不点进也能看。Issue 列是规划状态 | 聊天转录 + Changes panel；终端不是主表面 | 开：建 Workspace 即开跑。中：聊天/改消息重发；行评攒一批再 Send。结：人 Create PR；子 Issue 不自动完成父 |
| Nimbalyst | B/C 混合：Claude Code / Codex **SDK + MCP** 的 Agent Mode。Ghostty 终端是开发配套 | 挂 **Session 阶段列**（backlog…complete）+ 导航徽章：等人 / 在跑 / 未读。Tracker `in-review` 是另一层 | Agent Mode 转录（工具块、红绿 diff）；交互 prompt 持久化。**不是**官方 TUI 主表面 | 开：composer 新 Session。中：答 AskUser / ExitPlanMode / ToolPermission / GitCommitProposal；文件 approve/reject。结：人标 complete；Blitz/Super Loop 偏自动 |
| Claude Code Kanban | D：不启动 CLI。hooks 装上才有 agent log / 等待。发现靠 chokidar，不轮询 | 挂 Claude **task 文件**（Pending/In Progress/Completed）+ session：琥珀「等人」、agent active/idle。`?filter=active` 廉价探针 | Session log 时间线 + Agent log；**不嵌 TUI** | 开：无（人在官方 CLI 里自己开）。中：板不注入。结：板不关票 |
| KanVibe | A：官方 CLI 进 **tmux/zellij**，浏览器 xterm + node-pty 挂上。Hooks 推列 | 挂 **任务列**：TODO/PROGRESS/PENDING/REVIEW/DONE。不点开也能看见。PENDING=AskUser 或 Bash 权限（因 Agent 而异） | **整屏官方 TUI**（mux pane）+ 侧栏 chat/PR + Monaco diff | 开：建 branch TODO（自动 worktree+窗）。中：终端键入；编号 dock。结：hook→REVIEW；人标 DONE 则删 branch/worktree/终端 |
| Paperclip | E+B：heartbeat 拉起 `claude_local` / `codex_local` 等适配器（本机 CLI 进程，**不是**嵌 TUI）。已有 session 则续。超时/取消有 grace 再强杀 | 挂 **Issue 列** 与 **Run** 两套：run=`queued / running / succeeded / failed / timed_out / cancelled`。看板/仪表盘实时推 agent 与 run 状态 | 可读摘要为主，底下才是日志/tool-call；产品明确「不要日志崇拜」 | 开：指派/定时/Invoke。中：评论、@提及、board interaction（下拉/确认）。结：Agent PATCH `done` 或 board；另有静默 run watchdog / 任务树 watchdog |
| Claude Squad | A：tmux 里官方程序；`↵` attach。`-y` 自动接受提示 | 挂 **instance 列表**。不 attach 也能在 preview tab 瞄一眼 | preview（未 attach 的终端预览）+ **diff tab**；要完整 TUI 必须 attach | 开：`n` / `N`。中：attach 再打字。结：`c` 提交并暂停；`s` 推分支；`D` 杀掉。退出 ≠ 任务完成 |
| Emdash | **A+C 双路径**：`tui-agents` 用 `PtySpawner`（argv/stdin 送首句；有的 provider 标 `pty-only`）。ACP 路径另算。状态 **只来自 hooks/插件，不扫终端字** | 进程：`starting / running / exited`。Agent：`idle / working / awaiting-input / error / completed`。通知：等人 / 做完。ACP 另用 `isGenerating` + `pendingPermissionCount` | 官方 TUI PTY **或** ACP 聊天 + diff/PR。双主表面 | 开：人建 Task。中：PTY 键入或 ACP turn；权限通知。结：PR/CI/merge。hook 完 ≠ Tracker Issue 关 |
| Cyrus | B：Claude **Agent SDK** / headless runner（`spawnClaudeCodeProcess`，可扔到容器）；`--continue` 续。**不**给官方 TUI | 无本地板。状态流回 **Tracker 活动**（thought/action）。人在 Linear/GitHub 上看 | Tracker 评论/活动时间线，不是终端 | 开：把 Issue 指派给 Cyrus（自动）。中：在 Issue 上评论，灌进仍在跑的 session；下拉/审批。结：子程序 verification → `gh` PR → 摘要写回 |

#### 壳不重写 Agent 时，什么信号可靠

| 信号 | 能说明什么 | 不能说明什么 | 主源 |
| --- | --- | --- | --- |
| PTY/进程 exit + 退出码 | 进程没了；Cline 把 0 标 `reviewReason=exit`，非 0 标 `error` | Issue/卡片完成。#9：退出码 0 ≠ 任务完成 | Cline `session-state-machine.ts`；本仓库 #9 |
| 官方 hooks（AskUser / PermissionRequest / session.idle / Stop） | 在跑 / 等人 / 这一轮停手 | 验收通过、该关 Issue | KanVibe README；Cline `runtimeHookEvent`；Emdash providers.md（禁止从终端字推断） |
| ACP `pendingPermission` / `isGenerating` | 权限门、是否在生成 | Tracker 完成 | Emdash `agent-status-transition.ts`；OpenHands ACP_AGENTS |
| 自研聊天「Idle」 | 适配器认为这一轮结束 | 与官方 TUI 不一致时会撒谎 | Vibe workspace Idle；Claude Code Board `idle` |
| 只读监视 jsonl/task 文件 | Claude 自己的 todo/session | 不是本产品 Issue Tracker | Claude Code Kanban |

---

## 4. 给后续原型 / grilling 的可抄 / 应拒

> 不是 ADR。只给原型与 grilling 用的选项和红线。

### 4.1 可抄

1. **Issue 与 Run 分离**  
   Vibe 的 Issue↔Workspace、Emdash 的 Task↔Session/Conversation、本产品 #9 已钉的 Issue↔Run。板上的主键必须是 Issue Tracker 上的 Issue；Run 是一次可观察执行。

2. **派活主路径：人在 Issue 上点启动 → Embedded Terminal 跑官方 Agent CLI TUI**  
   Cline 的 PTY 路径、Claude Squad 的 attach。Agent Adapter 只负责探测、参数、启动；不要用 stream-json / ACP 聊天换掉 TUI。

3. **启动指令只带 Issue 定位**  
   与 #9 一致。反面是 Vibe「描述即整份 prompt」。Agent 自己读 Tracker 最新正文、评论、Dependency。

4. **Dependency 只解锁 Frontier，不启动 Run**  
   可抄 Cline「至少一端在可执行集、Done 不再入链」的链接规则；**不要**抄 `trashTaskAndGetReadyLinkedTaskIds` 之后的自动 kickoff。

5. **审阅面挂在 Issue/Run 旁，不另做聊天产品**  
   worktree/分支 diff + 行级评论作为 **下一次 Run 的输入素材**（Cline / Vibe Changes panel）。Nimbalyst：「Agent 可送审，只有人能批准 / 关 Issue」。

6. **并行隔离默认每活跃 Run 一个 worktree**  
   Cline / Claude Squad / Emdash / KanVibe 的共识。同一 Issue 同时最多一个活跃 Run（#9），因此一 Issue 不必同时多个 worktree。

7. **观察 ≠ 编排**  
   Claude Code Kanban 的「never directs」。Hooks 只反映 Run 忙闲（KanVibe 的 PENDING/REVIEW 映射有用），**不得**认领、关 Issue、推 Frontier。

8. **可选 Tracker 写回是独立动作**  
   Cyrus 证明写回能做；#9 已钉 v1 为可选、失败不影响本地 Run、不自动关 Issue。需要时用一条结构化评论，白名单字段即可。

9. **结构 / Dependency / 认领 / 执行 四套关系分开**  
   Paperclip execution-semantics 与 Routa「parent ≠ blocker」同向。本产品已有 Issue、Dependency、Frontier（未关闭 ∧ 无未完成阻塞 ∧ 未被占用）、Run。不要用看板列冒充这四者。

### 4.2 应拒

| 应拒 | 原因 | 反面教材 |
| --- | --- | --- |
| 自建第二套任务库当 SoT | 违反「工作单位是 Tracker 上的 Issue」；必双写或让 GitHub 沦为附件 | Vibe Issue、Cline `board.json`、Routa 卡片、Paperclip 内部票、KanVibe SQLite、Nimbalyst 本地 tracker 作主路径 |
| 自研聊天 / ACP 渲染替代官方 TUI | 违反 Embedded Terminal | Claude-Code-Board stream-json、OpenHands ACP、Vibe chat、Routa session |
| 自动认领 / 自动跑 Frontier | v1 不做编排器；完成信号不可靠 | Paperclip checkout、Cyrus 指派即跑、Cline 依赖自动开跑、Routa 泳道自动 |
| Conversation / Session 当应用根工作单元 | 态势变成「下一个 prompt」 | OpenHands、Claude-Code-Board、Claude Squad 若当主壳 |
| 专家角色自动流转交付管道 | 个人工具过重；把 Review 交给另一个 Agent | Routa lane specialists |
| 看板列移动 = Issue 完成 | 与 #9「Run 结束 ≠ Issue 完成」冲突 | 几乎所有自建板；KanVibe DONE 还删 worktree |
| 账号 / 多租户 / org / 预算公司 OS | 北极星：个人本地 | Paperclip；Vibe 登录才能看板 |
| 外部 Issue 只当 webhook 触发器 | 板上没有 Frontier | OpenHands Automation、Cyrus（虽用外部 Issue，但是触发器+自动跑） |
| GitHub 镜像成本地 markdown 再编排 | 明确双写 | Routa `github_mirror`；Nimbalyst importer 作主路径 |
| 把 Claude/Codex 内部 todo 当 Tracker | 换仓即丢；无 wayfinder 语义 | Claude Code Kanban |

### 4.3 需要双写吗？

**v1 不需要。** 对照下来，双写都出现在「产品自己要当 Tracker」的项目。Agent Taskboard 已有 Tracker Adapter，GitHub 上 Issue / label / assignee / comment / sub-issue / blocked_by 可一等建模（见 `docs/research/github-tracker-api.md`）。

允许的「非双写」写：

- 可选 Run 追溯评论（#9 白名单）
- 可选启动时 assignee 认领
- 不自动改流转 label、不自动 close

若未来做 local markdown Tracker，那是 **又一个 Adapter**，不是 GitHub 的影子库。

### 4.4 审阅面 v1 建议做到哪一层（供 map 开票，非 ADR）

建议 grilling 默认选 **薄审阅**：

1. Issue 详情里看到该 Issue 当前/最近 Run 的分支或 worktree diff（只读）。
2. 行级评论 → 变成「下一次 Run 的可编辑指令」，仍在官方 TUI 里改。
3. 「开 PR」可以是 Agent Adapter 可声明的可选动作或用户去 `gh`，不是第二套评审产品。
4. **不要**做 Routa 式自动 Gate Specialist，也不要把列拖到 Done 当完成证明。

完成信号（CLI 退出码 / hook / 人工标记）只影响 **Run 结束态**，不推进 Issue。

### 4.5 对接 / 状态 / 过程 / 人机（可抄 / 应拒）

对齐北极星：官方 CLI TUI 进 Embedded Terminal；人派活 Run；v1 无自动编排。不改 §4.1–§4.4。

**可抄**

1. **对接 = 真实 PTY 跑官方程序**（Cline `node-pty`、Claude Squad tmux attach、Emdash `PtySpawner`、KanVibe xterm 挂 mux）。第二句 prompt 和权限询问都留在官方 TUI 里，壳不要再做一套输入框。
2. **进程态与语义忙闲分开**。进程：`starting → running → exited`（Emdash TUI session；本产品 #9 的 `starting → running → ended`）。忙闲：hooks / ACP permission，**禁止**扫终端输出猜「在等人」（Emdash providers.md 写死了这条）。
3. **列表上只挂 Run 摘要，不挂第二套完成语义**。可抄 Cline 卡片 hook 一行、OpenHands 会话状态点、KanVibe/Vibe 的「等人」标记。列（Frontier / In Run）是 Issue 态势，不是进程态。
4. **过程主表面 = 整份官方 TUI**；diff 是旁边的审阅面（Cline / Squad / KanVibe）。卡片一行、时间线、转录都是缩略，不能替代 TUI。
5. **人机三拍**：人点启动；中途进 TUI 键入或停 Run；结束由人关 Issue。行评若做，只作为 **下一次 Run 的指令素材**（Cline/Vibe），仍进官方 TUI。
6. **观察层可选用 hooks**（Claude Code Kanban / KanVibe / Emdash）：装了才有等人/停手；没装就只剩 pid/退出码。

**应拒**

| 应拒 | 原因 | 反面教材 |
| --- | --- | --- |
| stream-json / SDK / ACP 聊天当执行主表面 | 替换官方 TUI；权限门被壳重做一遍 | Claude Code Board、Vibe chat、OpenHands Canvas、Cyrus SDK、Routa session |
| 用 Tracker 评论当中途 stdin | 人离开 Embedded Terminal，完成信号也缠到 Tracker | Cyrus Linear 评论灌 session；Paperclip @提及 wakeup |
| 退出码 0 / hook「停手」自动关 Issue 或自动 commit | 与 #9 冲突；完成不可靠 | Cline autoReview；KanVibe hook→REVIEW 若再当成验收；Paperclip Agent PATCH done |
| 从终端字推断 waiting | 误报；idle 终端也会改 mtime | Emdash 明确禁止；Claude Code Kanban 也因此不用纯 mtime |
| 板上只给转录/时间线、不给 TUI | 人没法在官方权限 UI 里点允许 | Paperclip 摘要优先；Cyrus 活动流 |
| yolo / skip-permissions 当默认 | 人闸被关掉 | Squad `-y`；Board/Vibe `dangerously-skip-permissions`；Cline 研究预览的高自主 |
| 双主表面（PTY 与 ACP 聊天对等） | 用户不知道该在哪回答权限 | Emdash A+C（可作适配器能力，不能当 v1 默认 UX） |

---

## 5. 证据索引

| 项目 | 主源 |
| --- | --- |
| Claude Code Board | README · `backend/src/types/{session,workitem}.types.ts` · `ProcessManager.ts` |
| Routa | README · `docs/core-concepts/how-routa-works.md` · `docs/use-routa/kanban.md` · `docs/use-routa/sessions.md` · `docs/design-docs/execution-modes.md` · `src/core/kanban/github-issues.ts` · `src/core/github/github-issue-sync.ts` |
| Cline Kanban | README · `docs/architecture.md` · `src/core/api-contract.ts`（`idle/running/awaiting_review/failed/interrupted`、hook ingest） · `src/terminal/session-state-machine.ts` · `src/terminal/pty-session.ts` · `task-board-mutations.ts` · `use-linked-backlog-task-actions.ts` · `use-review-auto-actions.ts` |
| OpenHands | README · `docs/architecture.md` · `docs/ACP_AGENTS.md` · `OpenHands/automation` README · `src/types/conversation-status.ts` · `src/manifests/automation-interface.ts` |
| Vibe Kanban | README · `docs/issue-management.mdx` · `docs/workspaces/index.mdx` · `docs/workspaces/sessions.mdx` · `docs/workspaces/chat-interface.mdx`（Approval / skip permissions） · `docs/reviewing-code.mdx` · `docs/integrations/github-integration.mdx` · https://www.vibekanban.com/blog/shutdown |
| Nimbalyst | README · `docs/FEATURE_INVENTORY.md`（交互 prompt、session 徽章、红绿批准） · `UserDocs/creating-custom-trackers.md` |
| Claude Code Kanban | README · `docs/session-scanning.md` |
| KanVibe | README（含 hook 状态机与 DONE 清场） |
| Paperclip | README · `docs/start/core-concepts.md` · `docs/api/issues.md` · `docs/agents-runtime.md`（heartbeat run 状态） · `doc/execution-semantics.md` · `doc/TASK-WATCHDOG.md` · `doc/PRODUCT.md` |
| Claude Squad | README（tmux attach、preview/diff tab、`-y`、`c`/`s`/`D`） |
| Emdash | README · `CONTEXT.md` · `agents/integrations/providers.md`（hooks 不推断终端字；pty-only prompt） · `packages/core/src/runtimes/tui-agents/api/schemas.ts` · `tui-agent-status-transition.ts` · `acp/agent-status-transition.ts` · `updateLinkedIssue.ts` |
| Cyrus | README · `AGENTS.md`（SDK runner、评论灌 session、活动写回） · `docs/SELF_HOSTING.md` · `docs/CONFIG_FILE.md` · `docs/GIT_GITHUB.md` |
| 发现入口 | https://github.com/andyrewlee/awesome-agent-orchestrators （结论均回各仓主源） |
| 本仓库已钉 | `CONTEXT.md` · [#9](https://github.com/youjiaxing/agent-taskboard/issues/9) · [#2](https://github.com/youjiaxing/agent-taskboard/issues/2) / `docs/research/layout-ia.md` |

星标与 push 时间取自 2026-08-14 的 GitHub API，只作维护信号，不是证据本身。

---

## 6. 结论（给 wayfinder map 的短答）

**高信任可参考集（本票维度）**：Cline Kanban（官方 CLI PTY + diff 审阅）、Claude Squad（TUI attach + worktree）、Claude Code Kanban（观察层）、Emdash（外部 Issue 挂靠——方向要反转）、Nimbalyst（人闸）、KanVibe（hook=忙闲）、Routa（证据门，非自动流转）、OpenHands（控制面/后端，非工作单元）、Cyrus/Paperclip（外部或内部 Issue 自动编排的完整反例）、Vibe（Issue/执行室分离，但自建 Issue + 聊天主表面）。

**最贴合北极星的组合**：

- 工作单元 = **Issue Tracker 上的 Issue**（Cyrus 的单元选对了，编排选错了）
- 派活 = **人启动 Run，官方 CLI 进 Embedded Terminal**（Cline PTY / Claude Squad）
- Tracker = **Adapter 读写，不自建第二套库**（拒绝 Vibe/Cline/Routa/Paperclip/Nimbalyst 主路径）
- 审阅 = **Run 旁 diff + 人批准关 Issue**（Cline/Vibe 的 diff；Nimbalyst 的「只有人能批」）
- 观察 = hooks/退出码更新 Run，**永不**自动认领或推进 Frontier
- 对接 / 展示 = **官方 TUI 进 PTY**；列表只给 Run 忙闲；中途提问留在 TUI；结束由人关 Issue（§3.4 / §4.5）

**应避免**：Chat-home、自研聊天替代 TUI、自建票、心跳/指派/依赖自动开跑、列移动当完成、GitHub 镜像双写、扫终端字当「等人」、退出码当完成。
