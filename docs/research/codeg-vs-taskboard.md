# 调研：codeg 与 Agent Taskboard 的对照

- **Ticket**: [#39](https://github.com/youjiaxing/agent-taskboard/issues/39)
- **Branch**: `research/codeg-vs-taskboard`
- **Date**: 2026-08-20
- **Skill**: `research`
- **Scope**: 专评 codeg（https://github.com/xintaofei/codeg ，官方文档 https://docs.codeg.app ）与已钉的 Agent Taskboard v1 是否同一类产品。以四份旧文稿为起点，全部跟到 **当前** 主源（README / 官方文档全站 / 仓库源码 / 官方 iOS 与 Android 客户端仓库），标出相对旧文稿「有无实质新事实」。不写 ADR；不替 [#40](https://github.com/youjiaxing/agent-taskboard/issues/40)（是否改写 v1 规格）拍板。
- **起点文稿**（不重做全市场）:
  - [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) `research/agent-kanban-models` → `docs/research/agent-kanban-models.md`
  - [#18](https://github.com/youjiaxing/agent-taskboard/issues/18) `research/agent-worktree-isolation` → `docs/research/agent-worktree-isolation.md`
  - [#30](https://github.com/youjiaxing/agent-taskboard/issues/30) `research/comparable-features` → `docs/research/comparable-features.md`
  - [#35](https://github.com/youjiaxing/agent-taskboard/issues/35) `research/cline-kanban-vs-taskboard` → `docs/research/cline-kanban-vs-taskboard.md`（文稿体例以这一份为模板）
- **词表**: 根目录 `CONTEXT.md`（Issue / Project / Run / Frontier / Host / Client / Embedded Terminal / Dependency / 认领 / 父 Issue / 上次态势 / 隔离执行目录 / 等待操作 / 执行已停 / 自动推进 / 待确认 / 自检 / 查看改动 / 改动备注 / 配对）。不用同义替换。
- **取证时间**: 2026-08-20。codeg 仓库 `stargazers_count` 2869、`pushed_at` 2026-08-19T23:53:17Z、`created_at` 2026-02-09、Apache-2.0、未归档（[repo API](https://api.github.com/repos/xintaofei/codeg)）；最新 release **v0.26.2**（2026-08-19，[releases](https://github.com/xintaofei/codeg/releases)）；仓库内 `package.json` version 0.26.2。iOS 仓库 pushed 2026-07-27、Android 仓库 pushed 2026-07-27。

---

## 1. codeg 现状（对到当前主源）

### 1.1 形态与定位

- **定位**：多智能体编码工作区（multi-agent coding workspace）——"run every AI coding agent in one place — and let them work together"。[README](https://github.com/xintaofei/codeg/blob/main/README.md)
- **一句话叙事**：聚合各家 Agent CLI 的会话到一个可搜索工作区；主 Agent 可把子任务委派给其它类型的 Agent；不想盯着的活写进 To-dos 板，每任务一个 worktree 无人值守跑，落地前等人审阅。[README](https://github.com/xintaofei/codeg/blob/main/README.md)
- **三种运行方式**：桌面 app（Tauri 2 壳）、自建 `codeg-server`（浏览器可达）、Docker；另有原生 iOS / Android 客户端连桌面 Web Service 或自建 server。[README](https://github.com/xintaofei/codeg/blob/main/README.md)、[architecture](https://docs.codeg.app/reference/architecture)
- **架构**：一个 Rust 核心（`codeg_lib`）+ 一个 Next.js/React 静态前端，编译出三个二进制——`codeg`（桌面）、`codeg-server`（HTTP+WebSocket）、`codeg-mcp`（每次 agent 会话按需拉起的 stdio MCP 伴生，承载委派工具）。桌面走 Tauri IPC，浏览器走 HTTP+WS，同一套 UI。[architecture](https://docs.codeg.app/reference/architecture)
- **许可与社区**：Apache-2.0；README 有商业赞助位（API 中转站类赞助商）；微信群社区。[README](https://github.com/xintaofei/codeg/blob/main/README.md)

### 1.2 工作单元（自建 Session 与自建 Task，无 Tracker）

- **一等工作单元是 Session（对话）**：`codeg.db`（SQLite）存会话、设置、账号元数据；`~/.codeg/` 默认数据目录（`CODEG_HOME` / server 用 `CODEG_DATA_DIR`）。[architecture](https://docs.codeg.app/reference/architecture)、[configuration](https://docs.codeg.app/getting-started/configuration)
- **第二个工作单元是 To-dos 的 Task**：一张自建卡 = 标题 + 描述 + 要用的 composer 状态（agent / mode / 选项）。"A task is a title, a description, and the composer state to run it with." 走固定管道 `to do → queued → setting up → running ⇄ awaiting input → to review → merging → done`（failed / canceled 是旁路）。[tasks](https://docs.codeg.app/guide/tasks)
- **Task 与 Session 的区别**（官方原文）：会话是你和 agent 轮流对话；automation 是定时触发的会话；task 是「有起点、有隔离执行处、结尾有验收门」的工作单元。[tasks](https://docs.codeg.app/guide/tasks)
- **无任何 Issue Tracker 接驳**：README、docs 全站、源码均无 GitHub Issues / GitLab / Linear 集成。源码里 `github` 只出现在 git 凭据管理（`version_control.rs` 用 `api.github.com/user` 验证 token、Settings 里 GitHub 账号表单）。docs 全站 grep `issue tracker` / `github issue` 无结果；源码 grep `issue` 无 Tracker 语义命中。**结论：工作单元不是 Tracker Issue，也不读外部 Issue。**

### 1.3 派活 / 编排

- **派活主路径是聊天**：人开一个会话（composer），选 agent 和 model，发消息即开跑；不是「认领一张票再启动」。[workspace](https://docs.codeg.app/guide/workspace)
- **To-dos 的启动方式**：卡上 Start、拖到 In progress 列、Schedule（定时间，到点当 Start）、以及「Process automatically」（folder 设置里打开后，任务在容量空出时自动认领自己）——默认关。[tasks](https://docs.codeg.app/guide/tasks)
- **并发上限**：每 folder 一个并发限制，默认 2；超出排队（Queued）。[tasks](https://docs.codeg.app/guide/tasks)
- **Automations（cron 无人值守）**：把一条完整配置好的 composer 存成自动化，按 cron 定时或手动触发，headless 开 agent 会话；0.23 起可「Enqueue task」——不跑会话，只往 To-dos 板投一张卡。调度器住在 Codeg 进程里（桌面或 server）；Codeg 关着时错过的调度在下次打开时补一次，不补全部。[automations](https://docs.codeg.app/guide/automations)
- **agent 也能代建**：会话里的 agent 经工具可保存 automation / 排队 to-do（默认关，Settings → General 两个开关）。[automations](https://docs.codeg.app/guide/automations)
- **聊天频道远程派活**：Telegram / Lark(飞书) / WeChat(iLink) 机器人，`/task`、`/approve`、`/deny` 等命令，从手机发任务、批权限、收结果。[chat-channels](https://docs.codeg.app/guide/chat-channels)

### 1.4 Tracker 关系（无）

- 无 Tracker Adapter 概念；GitHub 只作为 git remote（push/pull 凭据）出口，由「git 账号」管理，token 放 OS keyring。[privacy](https://docs.codeg.app/reference/privacy)、[git](https://docs.codeg.app/guide/git)
- 会话聚合（aggregation）读的是**各家 agent 自己的 session store**（`~/.claude/projects`、`~/.codex/sessions`、OpenCode 的 SQLite 等），按「会话在哪个目录跑的」归档，不是读 Issue。[aggregation](https://docs.codeg.app/guide/aggregation)

### 1.5 依赖与编排

- **无任务间 Dependency**：docs 全站无依赖概念（grep `depend` 命中的都是设置文案）；To-dos 无任务链接、无阻塞关系；卡片顺序只是拖拽排序（供自动处理按序消费）。[tasks](https://docs.codeg.app/guide/tasks)
- **无人值守形态是完整一等能力**：cron automation + 定时启动 + Process automatically + 每 folder「Merge automatically」（审阅列自动落地，最旧优先，一次一个）+ 「Delete worktree after merge」。[tasks](https://docs.codeg.app/guide/tasks)、[automations](https://docs.codeg.app/guide/automations)

### 1.6 隔离（worktree 由壳创建）

- **Task 的隔离执行目录由 Codeg 自己创建**：`<project>-task-<id>` 目录、`task/<id>` 分支、钉在创建瞬间的 HEAD commit；默认建在项目旁，可配置统一目录；跨重跑复用同一 worktree；init 命令只在全新 worktree 里跑。[tasks](https://docs.codeg.app/guide/tasks)、源码 `src-tauri/src/work_task/git.rs`（`git worktree add` 封装）
- **会话级 worktree 也由壳创建**：git 分支菜单 → New worktree → Codeg 预填分支名与兄弟目录、跑 `git worktree add`、并开一个根在该 worktree 的新会话。[git](https://docs.codeg.app/guide/git)
- **Automation 的隔离**：每次 run 默认新建一次性 worktree（"New worktree per run (default)"）。[automations](https://docs.codeg.app/guide/automations)
- **子 agent 不自动隔离**：委派出的 worker 共享 lead 的工作目录，除非 lead 指定其它目录；文档明确「A sub-agent does not get an isolated worktree automatically」。[multi-agent](https://docs.codeg.app/guide/multi-agent)

### 1.7 审阅

- **Task 审阅列**：完成 → to review，卡上显示 agent 用 `task_complete` 报的一行结论；详情 = Result（Markdown 摘要）+ Changed files（相对任务基线 commit 的 diff，+/- 计数）+ Progress 时间线 + Details + 头部 total tokens。[tasks](https://docs.codeg.app/guide/tasks)
- **Preflight 命令**：folder 可配一条验收预检，任务到 review 时在 worktree 里跑，绿/红灯上卡；失败留输出尾部。[tasks](https://docs.codeg.app/guide/tasks)
- **四种出路**：Merge（agent 在自己的会话里做合并——把 base 合进 worktree、解冲突、再落到 base 分支；Codeg 不信任 agent 自述，turn 结束后查 git truth：base HEAD 是否真的动了、是否真含这份工作，否则回 review 并清掉半成品合并）、Follow up（先选意图再写文本：Rework / Keep going / Ask / Double-check；Ask 明确不许动文件）、Complete（没改文件时直接标记完成、不建空合并）、Abandon。[tasks](https://docs.codeg.app/guide/tasks)
- **会话里的审阅**：Changes tab 只读 diff（HEAD vs Working Tree）、行级无；整份 git 客户端（commit / branch / merge / rebase / reset / 三窗冲突编辑器 / push 状态标记）。[git](https://docs.codeg.app/guide/git)

### 1.8 官方 TUI vs 自研聊天

- **全部交互是自研聊天，官方 TUI 不出现**：Codeg 把 agent CLI 当子进程，用 **Agent Client Protocol (ACP)** 驱动——"Codeg is the client and the agent is the server"；自己渲染消息、工具调用卡、权限询问卡。[architecture](https://docs.codeg.app/reference/architecture)
- **Claude Code 与 Codex 尤其关键**：两家官方 CLI 不提供 ACP，Codeg 装的是 ACP 组织维护的 adapter 包（`@agentclientprotocol/claude-agent-acp`、`@agentclientprotocol/codex-acp`），直接包 SDK，**连官方 CLI TUI 都不经过**。[supported-agents](https://docs.codeg.app/guide/supported-agents)
- **集成终端（⌘J）是用户自己的 shell**，不是 agent TUI："a real shell, not a sandbox"，给用户自己跑命令用。[workspace](https://docs.codeg.app/guide/workspace)
- 其余 11 家走各家自己的 ACP 通道；自定义 agent 从公开 ACP registry 注册（0.22 起）。[supported-agents](https://docs.codeg.app/guide/supported-agents)、[custom-agents](https://docs.codeg.app/guide/custom-agents)

### 1.9 产品形态与配对

- **形态全家桶**：Tauri 2 桌面（macOS 已签名公证 / Windows 未签 Authenticode / Linux AppImage+deb+rpm）、`codeg-server`（Linux/macOS/Windows 原生二进制或 Docker，默认绑 `0.0.0.0:3080`，纯环境变量配置）、桌面「Web Service」开关（0.0.0.0 + 访问 token + 手机扫码）、原生 iOS（要求 iOS 26+，App Store v1.0.1）与 Android（要求 Android 12+，APK v1.0.0）客户端。[installation](https://docs.codeg.app/getting-started/installation)、[deployment](https://docs.codeg.app/getting-started/deployment)、[web-service](https://docs.codeg.app/reference/settings/web-service)
- **配对 = URL + 一个长期访问 token**：手机 app 加 server profile（URL + token + Test Connection）；浏览器首次连 Web Service 输一次 token。token 存 iOS Keychain / Android Keystore。**没有一次性配对码、没有按 Client 撤销**——换 token 是全局 regenerate。[installation](https://docs.codeg.app/getting-started/installation)、[web-service](https://docs.codeg.app/reference/settings/web-service)
- **远程走用户自己的网络**：文档推荐局域网直连、HTTPS 反代（Caddy/nginx）、VPN、tunnel；server 无内置 TLS。[deployment](https://docs.codeg.app/getting-started/deployment)
- **手机是 Client 不是核心**："No agent CLI or project checkout runs on the phone"；手机可开会话、看回复与工具调用流、答权限询问、浏览项目与分支。[architecture](https://docs.codeg.app/reference/architecture)、iOS 仓库 `docs/ios-redesign-plan.md`（定位「coding agent 的远程驾驶舱」：看进度、收审批、给指令、开任务、读结果）
- **数据与秘密**：`~/.codeg/` SQLite；桌面秘密进 OS keyring，server 落 `tokens.json`；无遥测、无账号、无产品云——"there's no Codeg cloud in the middle"。[privacy](https://docs.codeg.app/reference/privacy)
- **移动端能力边界**：iOS app 甚至带 Settings（Agents / MCP / ModelProviders / VersionControl / QuickMessages / System）；锁屏推送没有——iOS 仓库 `docs/live-activity-analysis.md` 明说自托管架构下无 APNs，Live Activity 不可行。

### 1.10 多项目

- 多 folder（项目目录）各自一组会话与一张 task 板；folder 可设别名/颜色/默认 agent；可把其它目录链进 workspace（linked folders）。[workspace](https://docs.codeg.app/guide/workspace)
- 无跨 folder Frontier 概念；唯一的跨 folder 视图是 Token Usage 报告与全局会话搜索（⌘K）。[token-usage](https://docs.codeg.app/guide/token-usage)

### 1.11 token 用量

- **完整本地报告**：读各家 agent 自己转录里记的用量（input / output / cache write / cache read，每 turn 一行），按 7/30/90 天/自定义区间、按 folder / agent / model 分解，缓存命中率、热力图、最重会话榜；**明确不估价**——"It's not a bill. There are no prices anywhere on the page."。[token-usage](https://docs.codeg.app/guide/token-usage)

### 1.12 其它（一笔带过）

- 会话聚合（import 各家 session store、原 agent 按 id 续跑、@ 引用旧会话只读）[aggregation](https://docs.codeg.app/guide/aggregation)；MCP 服务器管理与 skills（全局/项目级、共享 `~/.agents/skills` 约定）[mcp](https://docs.codeg.app/guide/mcp)、[skills](https://docs.codeg.app/guide/skills)；Office 文档（officecli + 实时预览）与科研 skills [guide](https://docs.codeg.app/guide/)；Split View / 平铺多会话；desktop pet；主题与快捷键设置。

---

## 2. 对照表：codeg 现状 vs Taskboard 已钉决策

已钉出处：[Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1) 的 Notes / Decisions so far / Out of scope，含 [#9](https://github.com/youjiaxing/agent-taskboard/issues/9)、[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#16](https://github.com/youjiaxing/agent-taskboard/issues/16)、[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)、[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)、[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)、[#32](https://github.com/youjiaxing/agent-taskboard/issues/32)。

| 维度 | codeg 现状 | Taskboard v1 已钉 | 关系 |
| --- | --- | --- | --- |
| 工作单元 | 自建 Session（对话）为主 + 自建 Task 卡（To-dos）；无外部 Issue（[§1.2](#12-工作单元自建-session-与自建-task无-tracker)） | Issue（Tracker 上的工作项）为展示与分派基本单位；不自建第二套库（#11、#17） | **冲突** |
| Tracker | 无 Tracker Adapter；GitHub 只作 git remote 凭据出口（[§1.4](#14-tracker-关系无)） | Tracker Adapter（v1 GitHub）为单一真源；上次态势只是只读缓存（#11、#29） | **冲突**（无 vs 真源） |
| 派活 | 开聊天即跑；task 卡 Start / 拖列 / 定时；Process automatically 默认关；cron automation 默认可用（[§1.3](#13-派活--编排)、[§1.5](#15-依赖与编排)） | 人认领 + 启动 Run；自动推进默认关、待确认 60s、自检（#20）；Dependency 只解锁 Frontier 不自动开跑（#17） | **部分覆盖**（人启动）+ **冲突**（cron / 自动认领 / 定时） |
| 依赖 | 无任务间 Dependency（[§1.5](#15-依赖与编排)） | Dependency 是 Tracker 上的阻塞关系；被阻塞方不进 Frontier；父 Issue 是另一回事（#11、CONTEXT.md） | **正交**（codeg 没有这一维） |
| 隔离 | Task / Automation / 会话 worktree 全部由 Codeg 自己 `git worktree add` 创建；子 agent 不隔离（[§1.6](#16-隔离worktree-由壳创建)） | 默认 Run 在 Project 主目录；隔离执行目录只走 Agent 原生 worktree；看板不替 CLI 建树（#16 / ADR 0004） | **覆盖目标**（并行不互踩）+ **冲突**（壳建树、默认即隔离） |
| 审阅 | review 列 + 基线 diff + preflight 绿灯 + 四种出路；Follow up 意图（Ask 不动文件）；merge 由 agent 做但 git truth 校验（[§1.7](#17-审阅)） | 查看改动只读、相对启动 commit、现场现算；改动备注只进下一轮开场白；v1 无开 PR 入口（#22 / ADR 0009） | **覆盖**（diff + 基线 + 下一轮反馈）+ **部分**（codeg 的 merge/Complete 是产品化落地，Taskboard 拒开 PR 入口但不拒终端外落地？——见 §3.3 #4） |
| 完成信号 | Task 完成 = merge 落地经 git 校验或 Complete（无改动时）；会话有 In Progress/Review/Completed 状态；无 SessionEnd/StopFailure/自检语义（[§1.7](#17-审阅)） | Run 结束 ≠ Issue 完成；完成 = Issue 关闭（看板不代关）；SessionEnd / StopFailure / 退出码只影响 Run 结束态；自检规则（#9、#20） | **冲突**（Task 的完成是产品自管语义） |
| 官方 TUI vs 聊天 | 自研聊天全面替代：ACP 驱动 agent 子进程，Claude/Codex 走 adapter 包连 SDK；⌘J 终端是用户 shell（[§1.8](#18-官方-tui-vs-自研聊天)） | 官方 CLI 进 Embedded Terminal，不自研聊天替代 TUI；Agent 定义要求官方交互 TUI（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope、CONTEXT.md） | **冲突**（根基性） |
| 产品形态 | Tauri 2 桌面 + 自建 server + Docker + 浏览器 + 原生 iOS/Android；Web Service 开关（[§1.9](#19-产品形态与配对)） | Tauri 2 桌面 + 浏览器（含手机）Client；Host 常驻；配对 = 一次性码 + 长期令牌 + 可撤销（#12、#21 / ADR 0006、0007） | **部分覆盖**（桌面 + 浏览器 + 手机当 Client）+ **正交**（配对协议、能力矩阵、server/Docker 形态） |
| 配对/远程 | URL + 单一长期 token，全局 regenerate；无一次性码、无按 Client 撤销；远程走用户自己的网络（[§1.9](#19-产品形态与配对)） | 配对 = 可到达地址 + 一次性配对码 + 长期令牌；Host 可撤销某个 Client；本机回环页免配对（#21 / ADR 0006） | **部分覆盖**（token + 用户网络）+ **正交**（一次性码 / 按 Client 撤销） |
| 多项目 | 多 folder 各自会话与 task 板；linked folders；无跨 folder Frontier（[§1.10](#110-多项目)） | 多 Project 各自 Frontier；中间四列只跟当前选中 Project；不做跨 Project/Host 聚合（#11、#14、#15） | **覆盖**（同构） |
| token 用量 | 完整本地报告（读 agent 转录），无价格（[§1.11](#111-token-用量)） | token 用量统计破例进 v1；美元费用与账号额度不进（#32） | **覆盖**（且边界一致：不估价） |
| 登录态 | Codeg 内建 Authentication & Models 面板：in-app OAuth（Codex 在 app 里登录）、API key、custom endpoint、model provider，直接写各家 config 文件（[supported-agents](https://docs.codeg.app/guide/supported-agents)、[authentication](https://docs.codeg.app/guide/authentication)） | Adapter 不管各家登录态 / API key，不拿来禁启动（#13、[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope） | **冲突** |
| git 管理 | 看板内完整 git 客户端（commit/分支/merge/rebase/reset/三窗冲突编辑器/远程账号）（[§1.7](#17-审阅)） | 看板内完整 git 管理应拒；查看改动只读（#30 §15.5、#22） | **冲突** |
| 无人值守编排 | cron automation（默认可用）+ 定时任务 + Process automatically + Merge automatically，是产品一等能力（[§1.5](#15-依赖与编排)） | v1 不做默认无人值守编排器；自动推进默认关、冷启动不推进（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)、#20） | **冲突**（虽各自默认关，但 codeg 形态完整且默认存在） |

---

## 3. 三列清单

### 3.1 codeg 覆盖了 Taskboard

| # | Taskboard 需求 | codeg 证据 |
| --- | --- | --- |
| 1 | 人点启动 → Agent 执行（派活主路径） | 会话发消息即跑；task 卡 Start / 拖列启动（[README](https://github.com/xintaofei/codeg/blob/main/README.md)、[tasks](https://docs.codeg.app/guide/tasks)） |
| 2 | 并行 Run 不互踩（隔离目标） | 每 task 一个独立 worktree、钉创建时 HEAD；automation 每 run 新 worktree（[tasks](https://docs.codeg.app/guide/tasks)、[automations](https://docs.codeg.app/guide/automations)） |
| 3 | 列表上的 Run 忙闲可见性（在跑/在等人/已停三态素材，[#32](https://github.com/youjiaxing/agent-taskboard/issues/32)） | 卡上状态 running / awaiting input / review / failed + agent 里程碑一行；会话状态点 In Progress/Review/Completed（[tasks](https://docs.codeg.app/guide/tasks)、[workspace](https://docs.codeg.app/guide/workspace)） |
| 4 | 查看改动（diff 素材） | review 详情 Changed files 相对任务基线 commit 的 diff + 全量 diff；会话 Changes tab 只读 diff（[tasks](https://docs.codeg.app/guide/tasks)、[git](https://docs.codeg.app/guide/git)） |
| 5 | 改动备注只进下一轮开场白、不灌在跑的 Run（#22） | Follow up 是审阅后的下一轮（rework/keep going），Ask 意图明确不动文件；不是即时回投在跑的 Run（[tasks](https://docs.codeg.app/guide/tasks)） |
| 6 | token 用量统计进 v1，美元费用不进（#32） | 完整本地报告、明确无价格（[token-usage](https://docs.codeg.app/guide/token-usage)） |
| 7 | 多 Project 各自看板、不做跨板聚合 | 每 folder 一张 task 板；跨 folder 只有统计与搜索（[tasks](https://docs.codeg.app/guide/tasks)、[workspace](https://docs.codeg.app/guide/workspace)） |
| 8 | 远程访问走用户自己的网络、无产品中继 | 局域网 / HTTPS 反代 / VPN / tunnel，无内置 TLS、无中继（[deployment](https://docs.codeg.app/getting-started/deployment)） |
| 9 | 看板本体无账号、本地优先（个人本地） | 无遥测、无账号、无产品云（[privacy](https://docs.codeg.app/reference/privacy)） |
| 10 | 手机只当 Client、不在手机上起 Host | "No agent CLI or project checkout runs on the phone"；手机答权限、开任务、读结果（[architecture](https://docs.codeg.app/reference/architecture)、iOS 仓库 docs） |

### 3.2 正交（codeg 不做、Taskboard 要做）

| # | Taskboard 已钉 | 说明 |
| --- | --- | --- |
| 1 | Issue Tracker 接驳（GitHub 读写、认领、评论、写回） | codeg 完全没有 Tracker 层（[§1.4](#14-tracker-关系无)） |
| 2 | Frontier 定义与筛选（未关闭 ∧ 无未完成阻塞 ∧ 未被认领） | codeg 无 Frontier 概念；task 板只有自己写的卡（词表见根目录 `CONTEXT.md`） |
| 3 | 认领（Tracker 上的 assignee 钉子）与父 Issue 层次 | codeg 无认领、无父/子（[§1.2](#12-工作单元自建-session-与自建-task无-tracker)） |
| 4 | Dependency（阻塞关系、解锁 Frontier 不自动开跑） | codeg 无任务间依赖（[§1.5](#15-依赖与编排)） |
| 5 | Triage Role / Label Mapping / skills 只读透镜 | codeg 无（[#10](https://github.com/youjiaxing/agent-taskboard/issues/10)；codeg 的 skills 是给 agent 用的技能包，不是 Issue 透镜） |
| 6 | 完成信号体系：SessionEnd / StopFailure / 待确认 60s / 自检 / 自动推进开关 | codeg 无完成信号概念；Task 完成 = 产品自管语义（merge 落地 / Complete）（[#20](https://github.com/youjiaxing/agent-taskboard/issues/20) / ADR 0005） |
| 7 | 上次态势 / 离线（只读副本，不拿旧数据认领） | codeg 无 Tracker 缓存语义；它的本地 SQLite 是自己的 SoT（[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)、[architecture](https://docs.codeg.app/reference/architecture)） |
| 8 | 配对协议：一次性码 + 长期令牌 + 按 Client 撤销；本机回环页免配对 | codeg 是单一长期 token、全局 regenerate，无一次性码、无按 Client 撤销（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[web-service](https://docs.codeg.app/reference/settings/web-service)） |
| 9 | 启动配置表单 + 启动环境快照（Adapter 声明字段、按 Project×Agent 记默认） | codeg 有每 agent 的 mode/选项与 per-agent worker 默认，但无「目标目录用户默认壳整环境快照」概念（[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#23](https://github.com/youjiaxing/agent-taskboard/issues/23)） |
| 10 | Embedded Terminal 跑官方 CLI TUI | codeg 无官方 TUI；⌘J 只是用户 shell（[§1.8](#18-官方-tui-vs-自研聊天)） |
| 11 | 通知走本机系统通道、点击跳转对应 Run/Issue | codeg 通知在 app 内（desktop pet 徽标、坏运行徽标）；聊天频道推送走第三方聊天服务；无本机系统通道描述（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#32](https://github.com/youjiaxing/agent-taskboard/issues/32)） |

### 3.3 冲突（codeg 做了、Taskboard 已拒）

| # | codeg 现状 | Taskboard 已拒 |
| --- | --- | --- |
| 1 | 自建 Session / Task 卡当工作单元 | 自建第二套任务库当 SoT（#11、#17 §4.2） |
| 2 | 自研聊天全面替代官方 TUI（ACP 驱动，Claude/Codex 连官方 CLI TUI 都不经过） | 自研聊天式 Agent UI 替代官方 CLI TUI（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope）；Agent 定义要求官方交互 TUI（CONTEXT.md） |
| 3 | cron automation + 定时任务 + Process automatically + Merge automatically：无人值守编排是一等能力 | 默认无人值守编排器（心跳/指派即自动领完 Frontier）（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope；#20 自动推进默认关、待确认、自检） |
| 4 | Task 完成 = 产品自管落地（agent 合并进 base 分支、Complete 直标完成） | 看板不代关 Issue；Run 结束 ≠ 完成；v1 产品化「开 PR」入口或把 PR 当完成证据应拒（#9、#22）。注：codeg 的落地是本地 git merge 不是 PR，冲突程度按此打折，但「产品替 agent 的完成做最终裁决」的语义与 Taskboard 已钉相反 |
| 5 | worktree 一律由壳创建（task / automation / 会话级），Task 默认就隔离 | 默认 Run 在 Project 主目录；隔离执行目录只走 Agent 原生 worktree；看板不替 CLI 建树（#16 / ADR 0004） |
| 6 | 看板内完整 git 客户端（commit / 分支 / merge / rebase / reset / 三窗冲突编辑器 / 远程账号） | 看板内完整 git 管理应拒；查看改动只读（#30 §15.5、#22） |
| 7 | Codeg 内建 Authentication & Models：in-app OAuth（Codex 在 app 里登录）、API key、custom endpoint，写各家 config 文件 | 看板管理各家 Agent 的登录态 / API key 不是职责边界（#13、[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) Out of scope） |
| 8 | 聊天频道远程驱动（Telegram / Lark / WeChat）+ webhook + 每日报告；WeChat 走 iLink 托管服务 | 锁屏后台推送不当 v1 一等验收（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)）；产品自建公网中继应拒（iLink 是第三方托管，轻冲突） |

---

## 4. 与北极星兼容的启发候选（不拍板）

### 4.1 值得带到决策票讨论（事实 + 为何可能有用）

1. **审阅的「下一轮反馈带意图」**：Follow up 先选 Rework / Keep going / Ask / Double-check 再写文本，Ask 明确不许动文件。[tasks](https://docs.codeg.app/guide/tasks)——Taskboard 的改动备注只进下一轮开场白（#22 已钉）；意图分类是备注的附加结构，能让下一轮开场白更准，不改变已钉机制。
2. **落地前用 git truth 校验，不信任 agent 自述**：merge 后查 base HEAD 是否真动、是否真含工作，否则回 review。[tasks](https://docs.codeg.app/guide/tasks)——Taskboard 的自检是模型判断（#20 已钉）；客观 git 校验可作为自检的补充手段讨论，不替 #20 拍板。
3. **task 卡上的「最新里程碑一行」**（`task_progress` 实时上卡）+ 四种「需要你」原因细分（权限 / 提问 / plan 审批 / 子 agent 卡住）。[tasks](https://docs.codeg.app/guide/tasks)——直接喂给 #32 已采纳的「列表三态 / Run 事件通知」的实现细节：等待操作可以细分来源。
4. **token 用量报告的分层**：按 folder / agent / model / session 分解、缓存命中率、不估价。[token-usage](https://docs.codeg.app/guide/token-usage)——#32 已破例把 token 用量统计纳入 v1；codeg 的「读 agent 自己转录、不估美元」与已钉边界一致，分解维度可作规格素材。
5. **「执行已停」的恢复语义具体化**：task 中断标记 interrupted、retry 在同一 worktree 继续并带「上次被打断」的说明、worktree 丢失后从记录分支重建。[tasks](https://docs.codeg.app/guide/tasks)——与 #20 的「执行已停：恢复原生会话或释放认领」同思路，提供边缘场景清单。
6. **手机端「只读 live 会话视图 + 回答等待操作」**：手机可看运行中会话的实时转录（只读、不能 prompt），但能答权限询问；task 卡上 awaiting input 一眼可见。[tasks](https://docs.codeg.app/guide/tasks)、[architecture](https://docs.codeg.app/reference/architecture)——与「手机只当 Client、不做完整查看改动」相容；手机答等待操作是已钉边界内的增量素材。
7. **审阅旁的 folder 级 preflight 命令**（绿灯/红灯上卡）。[tasks](https://docs.codeg.app/guide/tasks)——注意：Taskboard 已拒「待审列 / 把查看改动写成 Tracker 状态」；preflight 是用户自定义命令、不写 Tracker 状态，是否值得讨论由 #40 决定，本票不拍板。

### 4.2 看着炫但越北极星（应拒或至少不进 v1）

1. **ACP 自研聊天**（含 Claude/Codex 的 adapter 包路线）：功能极全但根基与北极星相反——官方 CLI 进 Embedded Terminal 是已钉决策，codeg 的整条产品线都是反例。
2. **cron automation + 定时启动 + 自动认领 + 自动 merge 全家桶**：无人值守编排的完整形态；与 #20「默认关但形态存在」不同，codeg 是默认可用的一等能力，越界。
3. **看板内完整 git 客户端**（含三窗冲突编辑器、reset、push 状态）：#30 §15.5 已拒整族。
4. **管各家登录态 / API key（in-app OAuth、写各家 config）**：#13 已钉不是看板职责。
5. **聊天频道远程驱动（Telegram / Lark / WeChat / webhook / 每日报告）**：第三方聊天服务当控制面，含 iLink 托管依赖；越「无账号、无中继、用户自己的网络」边界。锁屏推送（iOS Live Activity）官方自己也承认被自托管架构卡住（iOS 仓库 `docs/live-activity-analysis.md`）。
6. **会话聚合的「读各家 session store 当历史」**：对 Taskboard 无对应需求（Taskboard 的历史在 Tracker 上），且属于「复制第二份会话数据」方向，越界。

---

## 5. 相对四份旧文稿：补了什么新事实

codeg 在四份旧文稿里**完全没被扫过**（grep 四份文稿无 codeg 命中）。以下分「同类重复」与「codeg 独有」。

**同类产品也有的重复**（旧文稿已覆盖，仅确认存在）：

- 每任务一 worktree、壳建树（cline/kanban 同，[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) §2.3、[#18](https://github.com/youjiaxing/agent-taskboard/issues/18) §3.1）——codeg 的 task worktree 同款。
- 列表上的在跑/在等人可见性（[#30](https://github.com/youjiaxing/agent-taskboard/issues/30) §4）——codeg 卡上里程碑一行同款。
- 审阅 = 基线 diff（[#17](https://github.com/youjiaxing/agent-taskboard/issues/17)、[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)）——codeg review 详情同款。
- 多项目各自板、无跨板聚合（cline 同）——codeg folder 板同款。
- 无账号、本地优先、远程走用户网络（cline 同）——codeg 同款。

**codeg 独有、旧文稿没有的实质新事实**：

| # | 新事实 | 证据 |
| --- | --- | --- |
| 1 | **ACP（Agent Client Protocol）作为统一驱动协议，且 Claude Code / Codex 走官方 adapter 包连 SDK、不经过官方 CLI TUI**。旧文稿的「对接 Agent CLI」分类（#17 §3.4）只有 PTY / stream-json / SDK 直连等，没有「两家头部 CLI 官方不提供 ACP、靠 ACP 组织 adapter 包装」这一事实 | [supported-agents](https://docs.codeg.app/guide/supported-agents)、[architecture](https://docs.codeg.app/reference/architecture) |
| 2 | **会话聚合**：读各家 agent 的 session store（`~/.claude/projects`、`~/.codex/sessions`、SQLite 等）导入、原 agent 按 id 续跑、@ 引用旧会话只读。旧文稿无此能力 | [aggregation](https://docs.codeg.app/guide/aggregation) |
| 3 | **跨 agent 类型委派**（delegate_to_agent 经 codeg-mcp）：@ 提及、子 agent 卡实时状态、深度上限（默认 1）、11 家能 lead、子 agent 不自动隔离。旧文稿无跨类型委派案例 | [multi-agent](https://docs.codeg.app/guide/multi-agent) |
| 4 | **To-dos 的「审阅后再落地」完整管道**：review 列 + preflight + Follow up 意图 + agent 做 merge + git truth 校验 + 失败回 review。旧文稿的看板类产品（#17）是「进 Done 即完成 / 自动开跑」，没有「落地前验收 + 客观校验」模型 | [tasks](https://docs.codeg.app/guide/tasks) |
| 5 | **原生 iOS / Android 客户端**（codeg-ios / codeg-android，token 在 Keychain/Keystore，无 APNs 锁屏推送）。旧文稿移动端只有 Nimbalyst 推送（#30 §3） | [installation](https://docs.codeg.app/getting-started/installation)、iOS 仓库 `docs/live-activity-analysis.md` |
| 6 | **自建 server + Docker 形态**（`codeg-server`，环境变量配置、token 门禁、自更新回滚）。旧文稿同类产品均为本地 Web / 桌面 | [deployment](https://docs.codeg.app/getting-started/deployment) |
| 7 | **token 用量报告读 agent 转录、不估价**（缓存命中率、热力图、按 folder/agent/model 分解）。旧文稿 #30 §7 只有 KanVibe 额度查询 / claude-code-kanban 会话分解，无「读转录、非计费」实现 | [token-usage](https://docs.codeg.app/guide/token-usage) |
| 8 | **聊天频道远程驱动**（Telegram / Lark / WeChat + webhook + 每日报告）。旧文稿无 | [chat-channels](https://docs.codeg.app/guide/chat-channels) |
| 9 | **cron automation + 定时任务 + 自动 merge**：无人值守编排的完整形态（默认可用）。旧文稿 #30 §15.1 应拒的是 OpenHands / Routa / KanVibe 的 cron/webhook/自动推列——codeg 是同类里形态最完整的 | [automations](https://docs.codeg.app/guide/automations) |

---

## 6. 直接用 codeg 能否覆盖 Taskboard 北极星

**结论：不能。**

理由（用上面的冲突与正交，不写产品建议）：

1. **北极星的工作单元是 Tracker 上的 Issue**（多项目 Issue 态势 + 依赖为一等）；codeg 的工作单元是自建 Session 与自建 Task 卡，无任何 Issue Tracker 接驳（§3.2 #1、§3.3 #1）。「把 GitHub Issues 看清、理顺依赖」这个北极星核心动作在 codeg 里不存在。
2. **Dependency / Frontier / 认领 / 父 Issue / triage 全部缺失**（§3.2 #2–#5）；codeg 连任务间依赖都没有，态势维度为零。
3. **执行面根基相反**：北极星已钉「官方 CLI 进 Embedded Terminal、不自研聊天替代 TUI」；codeg 是自研聊天全面替代，Claude/Codex 连官方 CLI TUI 都不经过（§3.3 #2）。
4. **无人值守编排**：codeg 的 cron / 定时 / 自动认领 / 自动 merge 是一等能力，北极星 v1 明确不做默认无人值守编排器（§3.3 #3）。
5. **隔离与完成语义相反**：壳建树、Task 默认隔离（#16 已钉「默认主目录、只走 Agent 原生 worktree、看板不建树」）；Task 完成 = 产品自管落地（#9/#20 已钉「Run 结束 ≠ Issue 完成、看板不代关」）（§3.3 #4、#5）。
6. **配对协议不同**：单一长期 token 无一次性码、无按 Client 撤销（§3.2 #8）——不构成覆盖，是另一套远程模型。

能覆盖的只是执行配套面的一大部分（并行不互踩、diff 审阅、列表三态、token 统计不估价、多项目各自板、无账号本地优先、手机当 Client），这些恰是 Taskboard 已钉或已采纳候选的同类素材（§3.1），不改变「产品类不同」的判断。

**是不是同一类产品（一句话）**：不是同一类——codeg 是「多 agent 编码工作区 / 编排面」（自建会话与任务 + 自研聊天 + 无人值守），Taskboard 是「Issue 态势 + 依赖 + 执行配套」的看板；两者只在执行配套面（隔离 / 审阅 / token 统计 / 本地优先）重叠。

---

## 附：证据索引

| 主题 | 主源 |
| --- | --- |
| 仓库形态 / 许可 / 维护 / 版本 | https://github.com/xintaofei/codeg 、https://api.github.com/repos/xintaofei/codeg（star 2869、pushed 2026-08-19、Apache-2.0、未归档）、[releases v0.26.2](https://github.com/xintaofei/codeg/releases) |
| 产品叙事 / 形态 / 工作单元 / To-dos | https://github.com/xintaofei/codeg/blob/main/README.md 、https://docs.codeg.app/guide/tasks |
| 架构（三二进制、ACP、codeg-mcp、数据目录） | https://docs.codeg.app/reference/architecture |
| 派活 / 编排 / 无人值守 | https://docs.codeg.app/guide/automations 、https://docs.codeg.app/guide/chat-channels |
| 隔离 / worktree 谁建 | https://docs.codeg.app/guide/git 、https://docs.codeg.app/guide/tasks 、源码 `src-tauri/src/work_task/git.rs` / `src-tauri/src/work_task/engine.rs` |
| 多 agent 委派 | https://docs.codeg.app/guide/multi-agent |
| 会话聚合 / token 用量 | https://docs.codeg.app/guide/aggregation 、https://docs.codeg.app/guide/token-usage |
| 形态与部署 / 配对 | https://docs.codeg.app/getting-started/installation 、https://docs.codeg.app/getting-started/deployment 、https://docs.codeg.app/reference/settings/web-service 、https://docs.codeg.app/getting-started/configuration |
| 隐私 / 无账号 / 无遥测 | https://docs.codeg.app/reference/privacy |
| agent 名单 / ACP adapter / 登录态 | https://docs.codeg.app/guide/supported-agents 、https://docs.codeg.app/guide/custom-agents 、https://docs.codeg.app/guide/authentication 、源码 `src/lib/types.ts`（14 家含 Qoder；docs 页面写 13，README 写 14——以源码为准） |
| 移动端 | https://github.com/xintaofei/codeg-ios 、https://github.com/xintaofei/codeg-android 、iOS 仓库 `docs/ios-redesign-plan.md` / `docs/live-activity-analysis.md` |
| Taskboard 已钉 | [Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1)、`CONTEXT.md`、[#9](https://github.com/youjiaxing/agent-taskboard/issues/9)、[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)、[#16](https://github.com/youjiaxing/agent-taskboard/issues/16)、[#20](https://github.com/youjiaxing/agent-taskboard/issues/20)、[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)、[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)、[#29](https://github.com/youjiaxing/agent-taskboard/issues/29)、[#32](https://github.com/youjiaxing/agent-taskboard/issues/32) |

- 本票未写 ADR；未改 CONTEXT.md；未关 [#39](https://github.com/youjiaxing/agent-taskboard/issues/39)；未替 [#40](https://github.com/youjiaxing/agent-taskboard/issues/40) 拍板。
