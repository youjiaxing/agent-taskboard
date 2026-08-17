# 调研：同类产品里值得借鉴的功能

- **Ticket**: [#30](https://github.com/youjiaxing/agent-taskboard/issues/30)
- **Branch**: `research/comparable-features`
- **Date**: 2026-08-17
- **Scope**: 同类产品（提出者点名 + 上一轮对照 + 增补）的「好用功能」：快捷键、命令面板、搜索过滤、已保存视图、通知、空状态、浏览器打开 Issue、批量操作、费用/token 露出、草稿、置顶、稍后再看、列表上就能做的动作等。只从官方 README / 官方文档 / 源码取证；`gh` CLI 本机实测作补充；二手榜单只作发现入口，不作证据。
- **北极星对齐**: 个人本地效率工具（无账号、无权限、无多租户、无产品中继）；Issue 态势为主、Agent 执行为强配套；Embedded Terminal 跑官方 Agent CLI TUI；v1 不做默认无人值守编排器；[Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1) 的 Out of scope 整段视为应拒。

---

## 0. 范围与不重做

本票只扫「他们还做了什么让人觉得好用」，以下问题已由别票回答，本票不重写结论：

- 面板怎么切 → [#2](https://github.com/youjiaxing/agent-taskboard/issues/2) `research/layout-ia` → `docs/research/layout-ia.md`
- 工作单元 / 派活 / 审阅 / CLI 对接 / 状态 / 过程 / 人怎么插手 → [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) `research/agent-kanban-models` → `docs/research/agent-kanban-models.md`
- 目录怎么隔离 → [#18](https://github.com/youjiaxing/agent-taskboard/issues/18) `research/agent-worktree-isolation`
- 三家 CLI 自己的 flag / 登录 → [#7](https://github.com/youjiaxing/agent-taskboard/issues/7) `research/agent-cli-surface`
- 查看改动本身（diff 基线、行评、人闸）已钉 → [#22](https://github.com/youjiaxing/agent-taskboard/issues/22) ADR 0009，本票只提与它相邻的增量，不重开。

**主源清单**（本票取证对象）：

| 产品 | 主源 |
| --- | --- |
| Claude Code Board | https://github.com/cablate/Claude-Code-Board/blob/master/README.md |
| Routa | https://github.com/phodal/routa/blob/main/README.md |
| cline/kanban | https://github.com/cline/kanban/blob/main/README.md |
| OpenHands | https://github.com/OpenHands/OpenHands/blob/main/README.md、https://github.com/OpenHands/automation/blob/main/README.md |
| Vibe Kanban | https://github.com/BloopAI/vibe-kanban/blob/main/README.md |
| Nimbalyst | https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md |
| KanVibe | https://github.com/rookedsysc/kanvibe/blob/main/README.md |
| claude-code-kanban | https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md |
| gh-dash | https://github.com/dlvhdr/gh-dash/blob/main/README.md、官方文档（keybindings / notification-section / reusing） |
| Zed Agent Panel | https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md、`docs/src/command-palette.md` |
| Plane（增补） | https://github.com/makeplane/plane（`apps/web/core/components/views/`、`apps/web/core/services/view.service.ts` 等源码路径） |
| OpenProject（增补） | https://github.com/opf/openproject（`docs/user-guide/` 官方用户手册在仓库内） |
| gh CLI（补充实测） | 本机 `gh issue view --help` |

增补说明：Plane 与 OpenProject 是 [#2](https://github.com/youjiaxing/agent-taskboard/issues/2) 已对照过的同信任主源，本票只从「功能」角度再读，不重复它们的布局结论。

**已钉、与本票相关的决策**（冲突判断时引用）：四列定稿 [#15](https://github.com/youjiaxing/agent-taskboard/issues/15)；通知通道已见 [#21](https://github.com/youjiaxing/agent-taskboard/issues/21)（事件清单留给规格补钉）；查看改动 [#22](https://github.com/youjiaxing/agent-taskboard/issues/22)（只读、相对启动 commit）；自动推进 [#20](https://github.com/youjiaxing/agent-taskboard/issues/20)（默认关、待确认 60s）；Adapter 不管各家登录态 [#13](https://github.com/youjiaxing/agent-taskboard/issues/13)；当前 Project 内按标题搜索（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 收官补钉项）；主题清单 [#25](https://github.com/youjiaxing/agent-taskboard/issues/25)。

---

## 1. 键盘流与快捷键

**谁做了、好用在哪**

- **gh-dash**：vim 风格快捷键，且**全部可覆盖**（"Overridable vim-style keyboard hotkeys"）；`?` 开帮助菜单（列出当前上下文可用键位），`/` 聚焦搜索框，`r`/`R` 刷新当前/全部 section，`s` 切换 PR/Issue 视图，`q` 退出。官方文档逐个列出默认键位，帮助菜单随上下文变化。
  - 主源：https://github.com/dlvhdr/gh-dash/blob/main/README.md ；https://github.com/dlvhdr/gh-dash/blob/main/docs/src/content/docs/getting-started/keybindings/global.mdx
- **KanVibe**：自述 **keyboard-first**："Use shortcuts for project filters, task search, notifications, task detail panels, and common task actions **without losing terminal focus**"——快捷键的存在意义是「不打断终端心流」。设置里可开 **Vim 式看板控制**：`h/j/k/l` 移动卡片、`/` 找可见卡片文字、`n` 新建任务、`:move progress` 改状态、`dd` 删除；任务详情页有**编号 dock 快捷键**（元数据 / hook 状态 / AI chat / PR 动作，按键先于键入到达嵌入式终端）。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **claude-code-kanban**：键盘优先，按 `?` 出全量快捷键参考（"Press `?` for the full shortcut reference"）；`Shift+L` 会话日志、`Shift+M` 跟随最新消息。
  - 主源：https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md
- **OpenProject**：常规快捷键含 `s` 全局搜索、`g w p` 去工作包、`n w p` 新建工作包、`j/k` 列表上下选择、`?` 开快捷键清单；另有 access keys 一套。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/keyboard-shortcuts-access-keys/README.md

**对 Taskboard**：不冲突，且是「态势工具」类产品的共性。v1 桌面壳值得做的下限是：`?` 帮助面板、列 / Project 切换、聚焦底栏 Embedded Terminal、启动/停止 Run、打开查看改动。注意两个约束：浏览器 Client 的全局快捷键能力弱于桌面，v1 若做要按 Client 能力矩阵分开定；「可覆盖键位 + 自定义命令」（gh-dash 的 keybindings 配置）属于以后再说，不是 v1。

---

## 2. 搜索、过滤与已保存视图

**谁做了、好用在哪**

- **gh-dash**：`/` 聚焦搜索框后可**临时改写当前 section 的 GitHub 查询**，回车即刷新、焦点回到列表；改动**不持久**（官方文档明说），要持久就改配置文件——这是「临时搜索」与「已保存视图」分得很清的模型。每个 section 是一条保存的查询（"User-defined, per-repo, PRs & issues sections"），配置 YAML 一切；还支持**每仓库一份配置**（`.gh-dash.yml` 覆盖全局配置）与「包含其它配置文件」。
  - 主源：https://github.com/dlvhdr/gh-dash/blob/main/docs/src/content/docs/getting-started/keybindings/global.mdx 、https://github.com/dlvhdr/gh-dash/blob/main/docs/src/content/docs/configuration/reusing.mdx 、https://github.com/dlvhdr/gh-dash/blob/main/README.md
- **KanVibe**：**快捷任务搜索**可从任何地方打开，按项目名或分支名过滤，回车**直接跳进该任务的工作区**（不必先回看板）；项目过滤器配键盘导航，在仓库间快速切。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **Nimbalyst**：会话全文搜索（`Cmd+L`），带全文索引；会话列表虚拟化以撑大历史量。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- **OpenProject**：视图 = 一组过滤条件 + 排序（"a _view_ is a list of work packages … based on a set of filter criteria"）；每个项目自带默认视图（All open / Latest activity / Overdue / Assigned to me 等 7 种），用户可**创建、保存、改自己的视图**，标 Private / Public / **Favorite** 分栏展示。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/work-packages/work-package-views/README.md
- **Plane**：视图是一等对象——`ViewService` 有 `createView / patchView / deleteView / getViews`（https://github.com/makeplane/plane/blob/preview/apps/web/core/services/view.service.ts#L18-L42），项目视图与全局视图两档，列表页 `views-list.tsx` 渲染、`applied-filters` 展示已套用的过滤条件；每条 Issue 行上有 quick-action 下拉（`apps/web/core/components/issues/issue-layouts/quick-action-dropdowns/`）。
  - 主源：https://github.com/makeplane/plane/blob/preview/apps/web/core/components/views/views-list.tsx 、https://github.com/makeplane/plane/tree/preview/apps/web/core/components/issues/issue-layouts/quick-action-dropdowns

**对 Taskboard**：不冲突。搜索的下限已经钉了（当前 Project 内按标题搜索、更早已关闭的去 Tracker，见 [#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 收官补钉；triage 角色用于筛选分组见 [#11](https://github.com/youjiaxing/agent-taskboard/issues/11)）。本票能补充的素材：① gh-dash 的「临时改查询、回车刷新、不持久」交互很适合搜索框；② 搜索命中后**直达**（kanvibe 跳到任务工作区）与「搜索直达 Run 终端」同思路；③ 「已保存视图 / 自定义过滤分区」与四列定稿（[#15](https://github.com/youjiaxing/agent-taskboard/issues/15)）不冲突，但属以后再说——v1 四列 + 标题搜索已覆盖个人使用的主路径。

---

## 3. 通知

**谁做了、好用在哪**

- **Zed Agent Panel**：把 Zed 放到后台后，Agent 生成完毕可发**桌面系统通知**和/或**声音**，两者可单独开关（`agent.notify_when_agent_waiting`、`agent.play_sound_when_agent_done`），也可全关——关键词是「等」：通知在 agent **停下等人/完成**时发，不是每条消息都吵。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md （Get Notified 一节）
- **KanVibe**：**通知面板**汇总「agent 状态变化、后台同步结果、任务事件」三类，点一条**跳到对应任务**。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **Claude Code Board**：会话事件发 **Windows Toast**（"Real-time Notifications - Windows Toast notifications for session events"）；完成时有明确通知（工作流对比里写 "Clear completion notifications when tasks finish"）。
  - 主源：https://github.com/cablate/Claude-Code-Board/blob/master/README.md
- **Nimbalyst**：手机端**推送**「agent 完成」；桌面端有「等输入 / 运行中 / 未读」分组的导航角标；`schedule_wakeup` 到点发 OS 通知。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- **OpenProject**：完整**通知中心**——铃铛 + 未读数字红点（99+ 封顶），按「原因」（@提及 / 指派 / 关注 / 提醒 / 日期告警）和「项目」过滤，未读/全部切换，一键全部已读，点通知在**分屏打开该工作包并自动滚动到触发那条活动**；另有每日一封邮件汇总。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/notifications/README.md

**对 Taskboard**：不冲突，且直接喂给 [#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 留下的「通知事件清单（通道已见 [#21](https://github.com/youjiaxing/agent-taskboard/issues/21)）」补钉项。事件素材按同类证据收敛为四类：**Run 停下等输入**（Zed `notify_when_agent_waiting`、KanVibe 状态变化、Nimbalyst 等输入角标）、**正常完成**（CCB toast、Nimbalyst 完成推送）、**异常停止**（KanVibe 状态变化）、**Host 崩溃捡回**。每条通知**点击跳转**对应 Run/Issue（KanVibe / OpenProject 都是这个模式）。注意边界：手机锁屏推送是 Out of scope（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)），v1 通知只走本机系统通道；「完整通知收件箱（已读/未读、原因过滤）」是 Tracker 原生能力，见 §15.7 应拒。

---

## 4. 列表上的执行可见性（Run 忙闲 / 在等人）

**谁做了、好用在哪**

- **cline/kanban**：用 hooks 把「最新一条消息或工具调用」显示在每张卡片上——"monitor hundreds of agents at a glance **without opening each one**"。不开卡就能扫全貌。
  - 主源：https://github.com/cline/kanban/blob/main/README.md
- **KanVibe**：板上直接看到**哪个 agent 在跑哪个任务、同时几个**；悬停/聚焦卡片出会话面板，列出每个运行中会话**正在做什么**及其分支出的子任务；点某会话直接切到它所在的 tmux 窗口并把输入焦点给终端。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **claude-code-kanban**：需要权限/输入的会话**琥珀色高亮**（"waiting-for-user indicators — Amber highlight on sessions needing permission or input"）；会话活动指示实时刷新。
  - 主源：https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md
- **Nimbalyst**：「等输入 / 运行中 / 未读」分组角标（同 §3）。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md

**对 Taskboard**：不冲突，与 [#20](https://github.com/youjiaxing/agent-taskboard/issues/20) 的语义恰好互补——同类产品把「在跑 / 在等人 / 已停」当**观察信号**，不把它当完成信号；这正好是「执行已停」「自检」之外补上的第三态「在等人」（Run 活跃但 Agent 停在权限询问/提问上）。v1 值得在底栏 Run 条与看板列上显示这三态（来源：进程观察 + 各家 hook，不解析 TUI 内容）。注意：KanVibe 让 hook **自动推列**（TODO→PROGRESS→…）属于编排器行为，应拒，见 §15.1。

---

## 5. 列表上就能做的动作

**谁做了、好用在哪**

- **cline/kanban**：卡片上直接 **play** 启动任务（自动建 worktree + 终端）、**Commit / Open PR** 出口、Script Shortcut 一键跑 `npm run dev`（见 §12）。
  - 主源：https://github.com/cline/kanban/blob/main/README.md
- **KanVibe**：搜索结果上直接「建后续任务」（**保留项目与分支上下文**）；任务详情 dock 用编号键直开元数据 / hook 状态 / PR 动作。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **gh-dash**："Everything you can do on GitHub - **diff, comment, checkout, push, update** etc."——列表/预览两级就能完成 GitHub 上的高频动作，另有 custom actions 挂自定义命令。
  - 主源：https://github.com/dlvhdr/gh-dash/blob/main/README.md
- **Routa**：卡片在泳道间流转（带证据、带 Gate 判定），人主要在板上操作而非钻进聊天。
  - 主源：https://github.com/phodal/routa/blob/main/README.md
- **Plane**：Issue 行上的 quick-action 下拉（`quick-action-dropdowns/` 一组文件，按列表/归档/周期分场景）。
  - 主源：https://github.com/makeplane/plane/tree/preview/apps/web/core/components/issues/issue-layouts/quick-action-dropdowns

**对 Taskboard**：不冲突，与 [#15](https://github.com/youjiaxing/agent-taskboard/issues/15)（详情分块、底栏只留还要打交道的 Run）正交。v1 值得抄的是**行上高频动作**：Frontier 列上直接「认领 + 启动 Run」、进行中列上直接「聚焦终端 / 停止 / 查看改动」、最近完成列上直接「查看改动」。注意切一刀：cline/kanban 的 Commit / Open PR 入口是 Out of scope（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)「v1 产品化开 PR 入口」应拒），本票只抄「认领 / 启动 / 停止 / 查看改动」这类与已钉能力面（[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)）重合的动作。

---

## 6. 在浏览器打开 Issue

**谁做了、好用在哪**

- **gh CLI（本机实测）**：`gh issue view -w/--web` ——"Open an issue in the browser"。这是 GitHub 官方 CLI 的一等能力，一行命令把 Issue 原文页交给浏览器。
- **Routa**：GitHub 仓库可导入为虚拟工作区，在应用内浏览 tree / files / **issues / PRs / comments**。
  - 主源：https://github.com/phodal/routa/blob/main/README.md
- **gh-dash**：列表上能完成 GitHub 高频动作（§5），但复杂操作的设计取向仍是「不离开终端做能做的事，做不了的交回 GitHub」——它依赖 `gh` 的能力面，本身不重造 GitHub。

**对 Taskboard**：不冲突，而且是 Out of scope「完整替代 GitHub/GitLab Web UI 的全部能力」的正解互补：v1 每条 Issue 给一个「在浏览器打开」入口（GitHub 原生 URL，无需任何额外集成），把评论、历史、批量、里程碑等 Tracker 原生能力留在浏览器里，看板保持薄。手机 Client 尤其受益（手机不完整做查看改动是已钉边界，[#1](https://github.com/youjiaxing/agent-taskboard/issues/1)），手机上「浏览器打开 Issue」就是详情兜底。

---

## 7. 费用 / token / 额度露出

**谁做了、好用在哪**

- **KanVibe**：**AI usage 面板**（`Cmd/Ctrl+0` 或底栏图标）显示**每个账号的剩余额度**（Claude / Codex / Gemini 按账号列卡片，标出所属账号与套餐），Claude 的**每模型周限额**单列在周总量之下——"remaining quota per account, without leaving the terminal"。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md
- **claude-code-kanban**：**每会话 context 用量条 + token/费用分解 + 模型信息**，侧栏与详情面板都有；另有 `context-status.sh` 把用量接进 Claude 的 statusline。
  - 主源：https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md
- **Zed**：当前线程 token 用量显示在消息编辑器附近（"Zed surfaces how many tokens you are consuming for your currently active thread"），并以此驱动自动压缩阈值。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md （Token Usage and Compaction 一节）
- **Nimbalyst**：context 窗口用量显示 + pace 跟踪；速率限制有琥珀警示 / 红色阻断组件。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md

**对 Taskboard**：不冲突但分量要掂量。官方 CLI TUI 自己已经显示 token / 费用（三家都在界面上有），看板再显示是**冗余 + 解析成本**；KanVibe 的「额度查询」走各家账号 API，与 [#13](https://github.com/youjiaxing/agent-taskboard/issues/13)「Adapter 不管各家登录态/API key」的职责边界有张力——本票不拍板，留给将来决策票。v1 不做；以后若要动，优先做「每个 Run 的 token/费用」而非「账号额度」。

---

## 8. 草稿与输入持久化

**谁做了、好用在哪**

- **Nimbalyst**：**会话草稿持久化**（"Session draft persistence — unsent input preserved"）——没发出去的输入重启后还在；交互式提问（AskUserQuestion / 权限询问等）本身也可跨重启持久。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- **Claude Code Board**：基于已有会话**快速新建并智能预填**（"Quick Session Launch — Create new sessions based on existing ones with intelligent prefilling"）。
  - 主源：https://github.com/cablate/Claude-Code-Board/blob/master/README.md
- **Zed**：已发消息可**点开编辑重发**；Agent 生成中发的消息默认排队，可开 Steer 提前插入，可单条/全部撤下。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md （Editing Messages / Queueing Messages 两节）

**对 Taskboard**：不冲突，但对应面小。Taskboard 的「输入面」是启动表单 + 开场白（含改动备注，已钉 [#22](https://github.com/youjiaxing/agent-taskboard/issues/22)）+ 向运行中 Run 注入语句；[#13](https://github.com/youjiaxing/agent-taskboard/issues/13) 已记「按 Project×Agent 记默认值」，草稿是它的增量（未点启动就关掉的表单内容不丢）。v1 低优先，属以后再说。

---

## 9. 置顶 / 收藏 / 稍后再看

**谁做了、好用在哪**

- **Nimbalyst**：会话**置顶**（Session pinning）；worktree 也可置顶/改名。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- **claude-code-kanban**：会话日志里**钉住重要消息**（Follow & pin，`Shift+M` 跟随最新）。
  - 主源：https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md
- **OpenProject**：视图可标 **Favorite**，在「我的视图」区单列。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/work-packages/work-package-views/README.md
- **Zed**：模型可收藏（favorite models），一键在收藏间循环切换。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md （Favoriting Models 一节）

**对 Taskboard**：不冲突。「置顶一张 Issue」（在 Frontier 列里把要优先的票顶到前面）和「置顶底栏某条 Run」都可行，但 v1 四列是排序简单模型（[#15](https://github.com/youjiaxing/agent-taskboard/issues/15)），置顶是增量，以后再说。「稍后再看」更接近通知收件箱管理（GitHub 网页通知原生有该能力），看板不建第二套，见 §15.7。

---

## 10. 空状态

**谁做了、好用在哪**

- **Zed**：Agent Panel 的**空状态里直接放「新建线程」入口**（"the agent selector button on the left (in the empty state)"），空状态 = 上手动作，不是说明书。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/ai/agent-panel.md （Creating New Threads 一节）
- **Plane**：每种视图布局各有独立空状态组件（`apps/web/core/components/issues/issue-layouts/empty-states/project-view.tsx`、`global-view.tsx`），空状态按视图场景区分。
  - 主源：https://github.com/makeplane/plane/tree/preview/apps/web/core/components/issues/issue-layouts/empty-states
- **OpenProject**：通知中心空状态文案按过滤场景区分（"no notification with current filter" 之类，`in-app-notification-center.component.ts` L146-147）。
  - 主源：https://github.com/opf/openproject/blob/dev/frontend/src/app/features/in-app-notifications/center/in-app-notification-center.component.ts#L146-L147

**对 Taskboard**：与已钉一致，不重开。[#15](https://github.com/youjiaxing/agent-taskboard/issues/15) 已钉「三种空分开且不写说明书」；本票只确认同类产品（Zed / Plane / OpenProject）都是「空状态按场景分开、放上手动作、不放说明书」的取向，与定稿同向。

---

## 11. 批量操作

**谁做了、好用在哪**

- **OpenProject**：工作包表支持**批量编辑**（bulk edit：勾选多行后批量改属性，`app/views/work_packages/bulk/edit.html.erb`，页面标题即 "bulk edit selected work packages"）；另有批量删除、批量重指派等。
  - 主源：https://github.com/opf/openproject/blob/dev/app/views/work_packages/bulk/edit.html.erb
- **gh-dash**：custom actions 可把任意 shell 命令挂到选中项上（例如 README 示例把 `lazygit` 挂到 `g` 键），本质是「对选中项执行自定义批量动作」。
  - 主源：https://github.com/dlvhdr/gh-dash/blob/main/docs/src/content/docs/configuration/reusing.mdx

**对 Taskboard**：不冲突，但 v1 不做：① GitHub 网页对 Issue 的批量操作（改标签、指派、关闭）已经原生可用，看板重复做违反「完整替代 GitHub Web UI 不做」；② 批量「认领 / 启动 Run」风险高（认领语义是单张占用，[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)），不做。若以后要做，只做低风险批量（如批量改 triage 标签），属以后再说。

---

## 12. 常用命令一键执行（快捷脚本）

**谁做了、好用在哪**

- **cline/kanban**：**Script Shortcut**——设置里写一条命令（如 `npm run dev`），导航栏一个 play 按钮就跑起来，"instead of remembering commands or asking your agent to do it"。配合看板自带的 dev server 场景，调试/预览不用记命令。
  - 主源：https://github.com/cline/kanban/blob/main/README.md

**对 Taskboard**：不冲突，属以后再说。底栏 Embedded Terminal 是真实终端（[#15](https://github.com/youjiaxing/agent-taskboard/issues/15)），用户本来就能敲；「每 Project 保存几条常用命令一键执行」是省心增量，v1 不做。注意与启动配置（[#13](https://github.com/youjiaxing/agent-taskboard/issues/13)）的边界：这是终端里的普通命令，不是 Agent 启动参数。

---

## 13. 数据与存储管理

**谁做了、好用在哪**

- **claude-code-kanban**：**Storage manager**——查看磁盘占用、清理过期会话与任务（"Inspect disk usage and clean up stale sessions and tasks"）。Run 日志/会话越积越多时，给用户一个自查自清入口。
  - 主源：https://github.com/NikiforovAll/claude-code-kanban/blob/main/README.md

**对 Taskboard**：不冲突，属以后再说。Host 会攒 Run 记录与日志，v1 先不管，以后做一个「占用查看 + 清理」面板即可。

---

## 14. 命令面板

**谁做了、好用在哪**

- **Zed**：命令面板是**主要动作入口**（"The Command Palette is the main way to access actions in Zed"），模糊匹配所有动作与键位，`new file` 之类自然语言输入即过滤。
  - 主源：https://github.com/zed-industries/zed/blob/main/docs/src/command-palette.md
- **Plane**：前端有 command palette 钩子（`useCommandPalette`，例如 `toggleCreateViewModal` 从列表页直接开「新建视图」弹窗）。
  - 主源：https://github.com/makeplane/plane/blob/preview/apps/web/core/components/views/views-list.tsx#L29

**对 Taskboard**：不冲突，属以后再说。v1 的动作面小（切换列/Project、搜索、启动/停止、查看改动），快捷键 + 标题搜索已覆盖；等动作数量上来了再上命令面板（桌面端 Tauri 做全局命令面板很便宜，浏览器端受限）。

---

## 15. 应拒功能族（对齐 Out of scope，每条给原因）

**15.1 定时 / 事件触发 / 后台自动化编排**

- OpenHands Automation Service：**cron 定时**跑会话、**webhook 事件触发**（"Run OpenHands conversations on a cron schedule"、"Trigger automations via webhooks (e.g., GitHub events)"），预置自动化含「把 GitHub Issue 拆成任务」（README："automatically decomposing GitHub issues into tasks"）。
  - 主源：https://github.com/OpenHands/automation/blob/main/README.md 、https://github.com/OpenHands/OpenHands/blob/main/README.md
- Routa：schedules / webhooks / background tasks / workflow runs（"Use schedules, webhooks, background tasks, and workflow runs for automation beyond one-off prompts"）。
  - 主源：https://github.com/phodal/routa/blob/main/README.md
- KanVibe：hook 自动把任务推过 TODO→PROGRESS→PENDING→REVIEW→DONE（"let … hooks move tasks through the workflow automatically"）；cline/kanban 依赖完成即自动开跑（已在 [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 详述）。
  - 主源：https://github.com/rookedsysc/kanvibe/blob/main/README.md

**原因**：对齐 Out of scope「默认无人值守编排器」。[#20](https://github.com/youjiaxing/agent-taskboard/issues/20) 已钉自动推进默认关 + 待确认 60s + 自检，定时器 / webhook / 自动推列都是「让系统自己领活干」的形态，与已钉规则冲突。观察信号（§4）可以要，自动流转不要。

**15.2 邮件通知摘要**

- OpenProject：每日一封「全部通知汇总」邮件（"a once-a-day summary of all notifications by email"）。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/notifications/README.md

**原因**：对齐「无账号、本地优先」。个人本地工具没有邮件服务与收件地址体系；通知走本机系统通道（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21)）即可。

**15.3 团队 / 云 / 账号一族**

- Nimbalyst：团队协作（tracker / 文档共享、E2E 加密共享链接）、账号登录（邮箱 magic link / Google OAuth）、可选的团队同步服务。
  - 主源：https://github.com/Nimbalyst/nimbalyst/blob/main/docs/FEATURE_INVENTORY.md
- Vibe Kanban：可选登录后开团队板（"optional GitHub/Google login to open a team board"；上一轮 [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 已记）。
  - 主源：https://github.com/BloopAI/vibe-kanban/blob/main/README.md
- OpenHands：云后端 / Enterprise 基础设施选项。
  - 主源：https://github.com/OpenHands/OpenHands/blob/main/README.md

**原因**：对齐 Out of scope「账号系统、云同步、团队协作与权限、产品自建公网中继」。整族应拒，不再推荐回来。

**15.4 自研应用预览浏览器**

- Vibe Kanban：内置浏览器，带 **devtools / inspect 模式 / 设备模拟**（"Preview your app — built-in browser with devtools, inspect mode, and device emulation"），workspace 自带 dev server。
  - 主源：https://github.com/BloopAI/vibe-kanban/blob/main/README.md

**原因**：与北极星「Issue 态势为主、Agent 执行为强配套」无关，是编辑器/IDE 壳功能（[#2](https://github.com/youjiaxing/agent-taskboard/issues/2) 已明确避免完整 IDE 壳）；预览需求交给用户自己的浏览器/终端即可。

**15.5 看板内完整 git 管理**

- cline/kanban：导航栏分支名点开**完整 git 界面**——浏览 commit 历史、切分支、fetch/pull/push、可视化（"browse commit history, switch branches, fetch, pull, push, and visualize your git all without leaving Kanban"）。
  - 主源：https://github.com/cline/kanban/blob/main/README.md

**原因**：查看改动已钉为**只读** diff（[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)）；切分支、commit、push 是执行面行为，归 Agent / 终端 / git 工具，看板不替代。这与「看板不替 CLI 建树」（[#16](https://github.com/youjiaxing/agent-taskboard/issues/16)）同一取向：看板是态势与控制面，不是 git 客户端。

**15.6 自研聊天 / 消息管理**

- Claude Code Board：消息类型过滤（user/assistant/tool_use/thinking）、会话导出 JSON、会话内搜索、快捷模板（Code Review / Bug Fixing 等）、Workflow Stage。
  - 主源：https://github.com/cablate/Claude-Code-Board/blob/master/README.md
- OpenHands Canvas 的会话聊天 UI。

**原因**：对齐 Out of scope「自研聊天式 Agent UI 以替代官方 CLI TUI」。消息过滤 / 导出 / 模板都是聊天界面的一等能力，聊天界面本身已在 [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 应拒，本票只点名不重做。消息本身在官方 TUI 里，看板不复制一份。

**15.7 第二套通知收件箱**

- OpenProject 通知中心完整模型（未读/全部、按原因过滤、全部已读、每日邮件）；gh-dash 的通知视图（`notificationsSections` 按 reason:author / participating / mention / review-requested 等分区，未读/已读、归档）。
  - 主源：https://github.com/opf/openproject/blob/dev/docs/user-guide/notifications/README.md 、https://github.com/dlvhdr/gh-dash/blob/main/docs/src/content/docs/configuration/notification-section.mdx

**原因**：通知的「收件箱管理」（已读/未读、原因分区、稍后再看、归档）在 GitHub 网页通知里原生成套；看板只发**自己产生的 Run 事件通知**（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21) 通道 + 事件清单），不建第二套收件箱——否则既重复 Tracker 能力，又引入「两处已读状态不一致」的维护负担。

**15.8 自建第二套 Issue 库**

- Vibe Kanban 自建 kanban Issue（云板）、Claude Code Board 的 Work Item、Routa 自建卡片为 SoT——上一轮 [#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 已逐家应拒。

**原因**：工作单元是 Tracker 上的 Issue（[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#17](https://github.com/youjiaxing/agent-taskboard/issues/17)）；自建第二套库 = 双写。本票只点名不重做。

---

## 16. 三列清单

### v1 值得抄（建议升决策/原型票的候选）

| # | 功能 | 证据（主源） | 与已钉决策 |
| --- | --- | --- | --- |
| 1 | **列表行上直接做动作**：Frontier 列「认领+启动 Run」、进行中列「聚焦终端/停止/查看改动」、最近完成列「查看改动」，不必先进详情 | cline/kanban 卡片 play；KanVibe 快捷动作；gh-dash 列表动作；Plane quick-action 下拉 | 不冲突（[#15](https://github.com/youjiaxing/agent-taskboard/issues/15) 详情分块正交；动作面已钉于 [#11](https://github.com/youjiaxing/agent-taskboard/issues/11)） |
| 2 | **键盘流 + `?` 快捷键帮助**：列/Project 切换、聚焦底栏终端、启动/停止、查看改动；帮助面板随上下文列键位 | gh-dash（vim 键位 + `?` 帮助）；KanVibe keyboard-first；claude-code-kanban `?`；OpenProject 快捷键清单 | 不冲突；浏览器 Client 按能力矩阵降级 |
| 3 | **当前 Project 内搜索 + 过滤 chips**：已钉的标题搜索（[#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 补钉）配 gh-dash 式「临时改查询、回车刷新、不持久」交互与 triage 角色/状态过滤 chips（[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)），命中可直达 Run/Issue | gh-dash `/`；KanVibe 快捷任务搜索直达 | 不冲突（在已钉搜索上补交互） |
| 4 | **Run 事件系统通知 + 点击跳转**：四类事件（等输入/正常完成/异常停止/Host 崩溃捡回）→ OS 通知，点击跳到对应 Run/Issue；桌面+声音可分开关 | Zed `notify_when_agent_waiting` / `play_sound_when_agent_done`；KanVibe 通知面板跳转；CCB toast | 不冲突（[#21](https://github.com/youjiaxing/agent-taskboard/issues/21) 通道已钉，事件清单本就是规格补钉项） |
| 5 | **列表上的 Run 三态**：在跑 / 在等人 / 已停，只观察不推进（不表示完成） | KanVibe 板上会话可见性；claude-code-kanban waiting-for-user 琥珀高亮；cline/kanban 卡片最新消息 | 不冲突（与 [#20](https://github.com/youjiaxing/agent-taskboard/issues/20) 完成信号语义互补；「在等人」是执行已停之外的第三态） |
| 6 | **浏览器打开 Issue**：每条 Issue 一个入口跳 GitHub 原文，复杂操作交回 Tracker；手机 Client 作详情兜底 | gh CLI `-w/--web` 本机实测；routa GitHub 浏览；gh-dash 不重造 GitHub | 不冲突（与「完整替代 GitHub Web UI 不做」互补） |

### 以后再说（有借鉴价值，v1 不做）

| # | 功能 | 证据 | 备注 |
| --- | --- | --- | --- |
| 1 | 已保存视图 / 自定义过滤分区（v1 四列定稿后做） | gh-dash sections；OpenProject views（默认 7 种 + 私有/公开/收藏）；Plane `ViewService` | 与 [#15](https://github.com/youjiaxing/agent-taskboard/issues/15) 四列不冲突，是增量 |
| 2 | 命令面板 + 全局/跨 Project 搜索 | Zed command palette；Plane `useCommandPalette`；OpenProject `s` | v1 动作面小，快捷键+标题搜索已够 |
| 3 | 费用 / token / 额度露出 | KanVibe 账号额度；claude-code-kanban token/费用分解；Zed token 用量；Nimbalyst context 用量 | 官方 TUI 已有基础值；额度查询与 [#13](https://github.com/youjiaxing/agent-taskboard/issues/13) 职责边界有张力，留给决策票 |
| 4 | 置顶 / 收藏（置顶 Issue 或底栏 Run；收藏视图） | Nimbalyst 置顶；claude-code-kanban 钉消息；OpenProject Favorite 视图 | 与 [#15](https://github.com/youjiaxing/agent-taskboard/issues/15) 排序模型可共存 |
| 5 | 常用命令一键执行（每 Project 保存几条，如 `npm run dev`） | cline/kanban Script Shortcut | 底栏是真终端，用户可自己敲；是省心增量 |
| 6 | 草稿与未发送输入持久化（启动表单/开场白草稿） | Nimbalyst draft persistence；CCB 预填新建；Zed 编辑重发 | [#13](https://github.com/youjiaxing/agent-taskboard/issues/13) 已记默认值，草稿是增量 |
| 7 | 批量操作（只做低风险：批量改 triage 标签） | OpenProject bulk edit；gh-dash custom actions | GitHub 网页已有批量能力，不重复；批量认领/启动不做 |
| 8 | 数据与存储管理（Run 记录/日志占用查看与清理） | claude-code-kanban Storage manager | Host 数据会攒，v1 先不管 |

### 应拒（对齐 Out of scope，原因见 §15）

| # | 功能 | 证据 | 应拒原因（简述） |
| --- | --- | --- | --- |
| 1 | 定时/事件/后台自动化编排（cron、webhook、自动推列、依赖完成自动开跑） | OpenHands Automation；routa schedules/webhooks；KanVibe hook 推列；cline 链式开跑 | 默认无人值守编排器（Out of scope；[#20](https://github.com/youjiaxing/agent-taskboard/issues/20) 自动推进默认关） |
| 2 | 邮件通知摘要 | OpenProject 每日邮件汇总 | 无账号、本地优先；通知走本机系统通道 |
| 3 | 团队 / 云 / 账号一族（团队协作、云看板、云后端、登录） | Nimbalyst 团队/账号；Vibe Kanban 云板；OpenHands 云后端 | 账号系统、云同步、团队协作、公网中继（Out of scope） |
| 4 | 自研应用预览浏览器（devtools/设备模拟） | Vibe Kanban 内置浏览器 | 与北极星无关，IDE 壳功能（[#2](https://github.com/youjiaxing/agent-taskboard/issues/2)） |
| 5 | 看板内完整 git 管理（commit 历史/切分支/push/可视化） | cline/kanban 导航栏 git 界面 | 查看改动已钉只读（[#22](https://github.com/youjiaxing/agent-taskboard/issues/22)）；写操作归执行面 |
| 6 | 自研聊天 / 消息管理（消息过滤、导出、模板、会话搜索） | CCB 消息管理；OpenHands Canvas | 自研聊天替代官方 CLI TUI（Out of scope；[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 已拒，点名不重做） |
| 7 | 第二套通知收件箱（已读/未读、原因分区、稍后再看、全部已读） | OpenProject 通知中心；gh-dash 通知视图 | Tracker 原生通知已成套；看板只发 Run 事件通知 |
| 8 | 自建第二套 Issue 库（自建卡片为 SoT / 云板 / Work Item） | Vibe Kanban、CCB Work Item、routa SoT | 工作单元是 Tracker 上的 Issue（[#11](https://github.com/youjiaxing/agent-taskboard/issues/11)、[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 已拒，点名不重做） |

---

## 附：本票没动的东西

- 未写 ADR；未替任何未开的决策票拍板（含通知事件清单、费用露出与 [#13](https://github.com/youjiaxing/agent-taskboard/issues/13) 的边界）。
- 未改 CONTEXT.md、未改 [Map #1](https://github.com/youjiaxing/agent-taskboard/issues/1)、未关 [Ticket #30](https://github.com/youjiaxing/agent-taskboard/issues/30)。
- 「v1 值得抄」六项是否升决策/原型票，由主代理按 [#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 的收官流程决定。
