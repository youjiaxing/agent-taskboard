# 调研：Agent Client Protocol（ACP）是什么、对 Taskboard 有无帮助

- **Ticket**: [#41](https://github.com/youjiaxing/agent-taskboard/issues/41)（wayfinder:research）
- **Branch**: `research/agent-client-protocol`
- **Date**: 2026-08-20
- **Skill**: `research`
- **Scope**: 只读 ACP 官方主源（官网 `.md` 文档、规范仓库 `agentclientprotocol/agent-client-protocol`、各家官方文档 / CLI help / GitHub）。codeg 只作「有产品把 ACP 当 Client 用」的一两句例证，不从 codeg 倒推协议。不写 ADR；不替 [#40](https://github.com/youjiaxing/agent-taskboard/issues/40)（对照 codeg 后是否改写 v1 规格）拍板；不改 CONTEXT.md。
- **起点文稿**: [#7](https://github.com/youjiaxing/agent-taskboard/issues/7) `research/agent-cli-surface`（远程分支已删，自 commit `7e7f363` 读取）；[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) `origin/research/agent-kanban-models`；[#39](https://github.com/youjiaxing/agent-taskboard/issues/39) `origin/research/codeg-vs-taskboard`。
- **词表**: 根目录 `CONTEXT.md`（Issue / Project / Run / Frontier / Host / Client / Embedded Terminal / 等待操作 / 执行已停 / 自动推进 / 待确认 / 自检 / 查看改动 / Agent Adapter / 启动配置）。**注意**：ACP 里的 "Client" 是协议角色（面向用户的界面），与产品词 Client（连 Host 的界面）不是一回事——本文写「ACP Client」以示区分。
- **取证时间**: 2026-08-20。协议当前稳定版本 **v1**（`protocolVersion: 1`）；**v2 是 Draft**（公告 2026-07-20，官方明确「draft 会变、别默认上生产」）。规范仓库创建 2025-06-23、最近 push 2026-08-20、4027★（[repo API](https://api.github.com/repos/agentclientprotocol/agent-client-protocol)）。

---

## 1. ACP 是什么（人能看懂，半页内）

ACP（Agent Client Protocol）是「编辑器/IDE ↔ 编码 Agent」之间的开放通信协议。官方定位：像 LSP 标准化语言服务器集成一样，ACP 标准化 Agent 集成（[introduction](https://agentclientprotocol.com/get-started/introduction.md)）。

- **设计假设**：用户主要待在编辑器里，Agent 是编辑器拉起的子进程，双方用 JSON-RPC 2.0 说话；尽量复用 MCP 的 JSON 类型，另加 diff 等「Agent 编码体验」专用类型；用户可读文本默认 Markdown（[introduction](https://agentclientprotocol.com/get-started/introduction.md)、[architecture](https://agentclientprotocol.com/get-started/architecture.md)）。
- **角色**：Agent = 用生成式 AI 自主改代码的程序，通常作为 ACP Client 的子进程运行；ACP Client = 用户与 Agent 之间的界面（通常是 IDE/编辑器，也可以是别的 UI），负责管理环境、用户交互、资源访问控制（[overview](https://agentclientprotocol.com/protocol/v1/overview.md)）。
- **传输**：v1 稳定传输只有 **stdio**（Client 启动 Agent 子进程，stdin/stdout 上走换行分隔的 JSON-RPC）；Streamable HTTP 还在草拟；允许自定义传输（[transports](https://agentclientprotocol.com/protocol/v1/transports.md)）。远端 HTTP/WS 官方标注「work in progress」、在 v2 Draft 里（[introduction](https://agentclientprotocol.com/get-started/introduction.md)、[announcements/acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft.md)）。
- **连接流程**：`initialize`（版本+能力协商）→ `authenticate`（如需要）→ `session/new` 或 `session/load` → 一轮轮 `session/prompt`（Agent 用 `session/update` 通知流式回传），轮次结束响应带 `stopReason`（[overview](https://agentclientprotocol.com/protocol/v1/overview.md)）。
- **LSP 类比落到哪层**：LSP 管「编辑器 ↔ 语言工具」的静态能力；ACP 管「编辑器 ↔ 编码 Agent」的整段对话协作——prompt 轮次、工具调用、权限询问、终端、diff、计划、用量都进了协议，比 LSP 覆盖面大得多。**本地 stdio 已成熟**（v1 稳定、多家实现）；**远端 HTTP/WS 未成熟**（官方明示 WIP，v2 Draft 才推进）。
- 一个连接可并行多个会话（[architecture](https://agentclientprotocol.com/get-started/architecture.md)）。

## 2. 协议能力（对到主源）

v1 稳定能力（方法/通知见 [overview](https://agentclientprotocol.com/protocol/v1/overview.md)、[schema](https://agentclientprotocol.com/protocol/v1/schema.md)）：

| 能力 | 协议面 | 稳定状态 |
| --- | --- | --- |
| 会话 | `session/new`（建）、`session/load`（续，需 `loadSession` 能力）、`session/list`、`session/delete` | 稳定（各有 stabilized 公告） |
| prompt 轮次 | `session/prompt`；`session/update` 通知流：plan / agent_message_chunk / tool_call(+update) / usage_update / current_mode_update；结束响应带 stopReason：`end_turn` / `max_tokens` / `max_turn_requests` / `refusal` / `cancelled` | 稳定（[prompt-turn](https://agentclientprotocol.com/protocol/v1/prompt-turn.md)） |
| 工具调用 | `tool_call` 带 kind（read/edit/delete/move/search/execute/think/fetch/other）与状态（pending/in_progress/completed/failed）；`locations` 支持 follow-along；内容可带 diff（oldText/newText）与 terminal 引用 | 稳定（[tool-calls](https://agentclientprotocol.com/protocol/v1/tool-calls.md)） |
| 权限询问 | Agent → ACP Client 调 `session/request_permission`（选项 allow_once / allow_always / reject_once / reject_always），**ACP Client 渲染 UI 并回 outcome**；ACP Client 可依用户设置自动允许/拒绝 | 稳定（[tool-calls](https://agentclientprotocol.com/protocol/v1/tool-calls.md)） |
| 终端 | `terminal/create` / `terminal/output` / `terminal/wait_for_exit` / `terminal/kill` / `terminal/release` | 稳定（[terminals](https://agentclientprotocol.com/protocol/v1/terminals.md)） |
| 文件系统 | `fs/read_text_file`、`fs/write_text_file`（Agent 借用 ACP Client 的文件访问；路径必须绝对） | 稳定（[overview](https://agentclientprotocol.com/protocol/v1/overview.md)） |
| 计划 | plan entries（priority + status），整体替换式更新 | 稳定（[agent-plan](https://agentclientprotocol.com/protocol/v1/agent-plan.md)） |
| 模式 | session modes（ask / architect / code…）、`session/set_mode`；官方提示未来并入 session config options | 稳定（[session-modes](https://agentclientprotocol.com/protocol/v1/session-modes.md)） |
| 用量 | `usage_update`（session/update 之一）：上下文 used/size + 可选累计 cost | **稳定**（[session-usage-stabilized](https://agentclientprotocol.com/announcements/session-usage-stabilized.md)，2026-06-05） |
| 用量（turn 级） | per-turn token 明细（input/output/thought/cache…） | **Draft**（[end-turn-token-usage RFD](https://agentclientprotocol.com/rfds/end-turn-token-usage.md)，2026-06-02 拆分，明确 not ready for Preview） |
| 登录/认证 | Agent 在 `initialize` 公告 `authMethods`（默认 `type: agent`：**Agent 自己处理登录**），ACP Client 只调 `authenticate` / `logout` 触发；ACP Client 不持有凭据 | 稳定（[authentication](https://agentclientprotocol.com/protocol/v1/authentication.md)） |
| 取消 | `session/cancel` → `cancelled` stop reason | 稳定（[prompt-turn](https://agentclientprotocol.com/protocol/v1/prompt-turn.md)、request-cancellation-stabilized） |
| 结构化询问 | elicitation（表单 / URL 交互） | 稳定（elicitation-stabilized） |
| **没有的** | v1 无「idle / 等待操作 / requires_action」显式状态。Agent 空闲 = 上一轮 prompt 已响应；在等人 = 有挂起的 `session/request_permission` 或 elicitation、或 turn 未结束——**只能间接推断**。v2 Draft 的 `state_update`（running / idle / requires_action）才显式化（[rfds/v2/prompt](https://agentclientprotocol.com/rfds/v2/prompt.md)） | v2 Draft |

**关键区分（必须钉死）**：ACP 的 `terminal/*` 是 **Agent 请 ACP Client 在 Client 环境里开一条命令终端**（例：`npm test --coverage`，实时回输出与 exit status），不是「把官方 CLI TUI 嵌进 ACP Client」（[terminals](https://agentclientprotocol.com/protocol/v1/terminals.md)）。这和 Taskboard 的 **Embedded Terminal**（官方 Agent CLI 交互 TUI 的真实 PTY）**不是同一件事**。

## 3. 和官方 CLI TUI 的关系

- **协议层**：stdio transport 规定 Agent 的 stdout 只能输出合法 ACP 消息（MUST NOT 写其它），stderr 只作日志（[transports](https://agentclientprotocol.com/protocol/v1/transports.md)）。官方 TUI 是全屏交互渲染，ACP 是 stdout 上的协议流——**同一 stdout 不可能同时又是 TUI 又是 ACP**。
- **实现层**：Grok 官方文档原话——"Use ACP when you want IDE or tool integration rather than a terminal session"（[headless-scripting](https://docs.x.ai/build/cli/headless-scripting)）。ACP 与 TUI 是**同一二进制的两个入口**：`grok` 进 TUI；`grok agent stdio` 进 ACP、不渲染 TUI（[CLI Reference](https://docs.x.ai/build/cli/reference)）。本机 `grok 1.0.5` 实测 `grok agent stdio` 子命令存在。
- **adapter 是否绕开官方 TUI**：**绕开**。
  - `@agentclientprotocol/claude-agent-acp`：包**官方 Claude Agent SDK**（README 原文 "Use Claude Agent SDK from any ACP client"），不经过 `claude` CLI（[仓库](https://github.com/agentclientprotocol/claude-agent-acp)）。
  - `@agentclientprotocol/codex-acp`：stdio ACP server，**启动官方 Codex App Server** 并翻译 ACP ↔ Codex 操作，不经过 `codex` TUI（[仓库](https://github.com/agentclientprotocol/codex-acp)）。
- **结论**：ACP 是官方 TUI 的**替代面**（另一入口），不是 TUI 底下的同一种东西，也不是叠加在 TUI 上的并行面。协议内**不存在**「同一进程一边开官方 TUI、一边讲 ACP」的旁路通道。

## 4. v1 三家现状（Grok Build / Codex / Claude Code）

| Agent | ACP 现状 | 证据 |
| --- | --- | --- |
| **Grok Build** | **原生**。`grok agent stdio` = "Run as an ACP agent over stdin/stdout" | xAI 官方 [CLI Reference](https://docs.x.ai/build/cli/reference)、[headless-scripting](https://docs.x.ai/build/cli/headless-scripting)；本机 grok 1.0.5 实测。注：ACP 官方 agents 列表未列出 Grok（列表非完备，以其官方文档为准） |
| **Codex CLI** | **非原生，靠第三方 adapter**。官方 CLI 无 ACP 入口 | OpenAI Codex 文档（[CLI](https://learn.chatgpt.com/docs/codex/cli) 等全站无 ACP 入口）；`@agentclientprotocol/codex-acp`（原 `zed-industries/codex-acp`，2026-07-22 归档后迁至 ACP 组织；新 adapter 基于官方 Codex App Server，npm 包自带兼容 `@openai/codex` 依赖） |
| **Claude Code** | **非原生，靠第三方 adapter** | Anthropic 官方 [issue #6686](https://github.com/anthropics/claude-code/issues/6686)「Add support for ACP」2026-02-09 以 `not_planned` 关闭（社区请求重开未果）；`@agentclientprotocol/claude-agent-acp` 包官方 Claude Agent SDK（原 zed-industries/claude-code-acp） |

对照（非三家，作「原生」参照系）：Gemini CLI、GitHub Copilot（public preview）、Cursor、Cline、OpenCode 等原生进 ACP agents 列表（[agents](https://agentclientprotocol.com/get-started/agents.md)）；Zed 是 ACP 发起方之一（v2 公告作者 Ben Brandt 署名为 Zed Industries / ACP Lead Maintainer，[acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft.md)）。

产品例证（一两句，不展开）：codeg 走「Codeg 是 ACP Client、agent 是 server」，Claude/Codex 用上述 adapter 包直连 SDK，自研聊天替代官方 TUI（[#39](https://github.com/youjiaxing/agent-taskboard/issues/39) 已详证）。这只是「有产品用 ACP 当 Client」的例证，不是本票协议结论的来源。

## 5. 对照表：协议能力 vs Taskboard 已钉需求

已钉出处：Map [#1](https://github.com/youjiaxing/agent-taskboard/issues/1) 与 #9、#13、#16、#20、#22、#29、#32、CONTEXT.md。

| Taskboard 已钉需求 | ACP 能力 | 关系 |
| --- | --- | --- |
| 官方 CLI 进 Embedded Terminal、不自研聊天替代 TUI（北极星） | ACP 是 TUI 替代面（另一入口）；协议无 TUI 旁路 | **覆盖不了**（帮不了这一条；走 ACP 意味着 TUI 不进终端） |
| 权限询问留在官方 TUI（北极星） | `session/request_permission` 由 ACP Client 渲染 UI | **越北极星**（走 ACP 时权限 UI 必须搬到自研面） |
| 等待操作（Run 活跃但等人） | v1 无显式状态，只能从挂起请求/未结束 turn 推断；v2 Draft 才有 `requires_action` | **覆盖不了（v1）**（可推断，但无协议事件可依赖） |
| 完成信号：SessionEnd / StopFailure / 退出码 | stopReason（end_turn / cancelled / max_tokens / refusal）；**无 StopFailure**；无会话级 SessionEnd；退出码不出现在 ACP 里（进程退出由 ACP Client 自己观察） | **部分覆盖**（轮次级原因有；会话级/失败语义无） |
| token 用量统计（#32 破例进 v1；不估价） | `usage_update`：上下文 used/size + 累计 cost——**稳定**；per-turn 明细 Draft | **部分覆盖**（session 级稳定可用；turn 级未稳定；不估价与已钉一致） |
| 查看改动（只读、相对启动 commit 现场现算，#22） | diff content（oldText/newText） | **正交偏覆盖**（能提供「改了什么」素材；但已钉方案是 git diff 现场现算，不需要 ACP） |
| Agent Adapter 合同（可配置项声明 + 启动前表单） | `initialize` 能力协商、session config options（布尔等）、modes 是天然协商面；但各家 ACP 实现的配置暴露不一（codex-acp 用环境变量 / JSON 配置；grok 用 CLI flag） | **部分覆盖**（协商面可参考；配置面各家不一，仍需 Adapter 合同） |
| 登录态：Adapter 不管各家登录态 / API key（#13） | 默认 auth `type: agent`：Agent 自持登录，ACP Client 只触发 | **覆盖**（协议不把登录态做成 ACP Client 职责；codeg 的 in-app OAuth 是其产品选择，非协议要求） |
| 工作单元 = Tracker 上的 Issue | ACP 无 Issue 概念，纯会话协议 | **正交**（不冲突也不帮忙） |
| 自检 / 自动推进 / 待确认 60s / Dependency / Frontier | 无对应物 | **正交** |
| Run 绑 Issue、同 Issue 同时最多一个活跃 Run（#9） | 一连接多会话；ACP 会话 ≠ Run | **正交** |
| 隔离执行目录（#16：Adapter 声明原生建树，看板不建） | 协议无；依赖各家 CLI（grok 有 worktree flag） | **正交** |

## 6. 三种姿势 A/B/C 的事实后果（只列事实，不拍板）

- **A. 当 ACP Client，自研聊天当执行主表面（codeg / Zed 路线）**
  - 事实：三家里仅 Grok 原生（`grok agent stdio`）；Claude / Codex 必须挂第三方 adapter（包 SDK / App Server，不跑官方 TUI）。协议把渲染职责（消息、工具卡、权限 UI、diff、plan、usage）全部交给 ACP Client；会话持久化在 Agent 侧（`session/load`）。
  - 后果：直接冲突「官方 TUI 进 Embedded Terminal」与「权限询问留在官方 TUI」两条北极星；自研聊天 = 重做各家 TUI 的 UX 面。
- **B. 官方 TUI 进 Embedded Terminal + ACP 旁路拿结构化事件（等待操作、token、SessionEnd）**
  - 事实：**做不成旁路**。① 协议无旁路通道（stdio 独占 stdout，[transports](https://agentclientprotocol.com/protocol/v1/transports.md)）；② 同一二进制的 TUI 与 ACP 是两个互斥入口（Grok 官方文档 + 本机 help）；③ Claude / Codex 的 adapter 根本不启动官方 TUI；④ v1 没有 idle / requires_action 显式事件，「等待操作」拿不到协议事件；⑤ 「SessionEnd」在 v1 里没有对应物（v2 Draft 的 state_update idle 才是）。Emdash 的「双路径」是两个入口（ACP 聊天 + 终端 Session），不是旁路（[#17](https://github.com/youjiaxing/agent-taskboard/issues/17) 已记）。
  - 后果：同一进程双通道不存在；要实现 B 只能同时维护两个 agent 进程 / 两套会话（一个 TUI 进程 + 一个 ACP 进程），且拿不到协议保证的状态一致性。
- **C. 完全不用 ACP**
  - 事实：Embedded Terminal 里跑官方 TUI（北极星现状）；事件面靠 PTY 输出解析 + 进程退出码 + 各家 CLI 面（#7 / #32 已有素材）。
  - 后果：与已钉路线一致；拿不到 ACP 的结构化事件；token 用量统计需另找来源（各家 CLI 转录/输出，如 codeg 读各家转录的做法）。

## 7. 直接回答：对 Taskboard 有多大帮助

- **帮不到核心**：Taskboard 已钉「官方 CLI 交互 TUI 进 Embedded Terminal、不自研聊天替代 TUI、权限询问留在官方 TUI」。ACP 是 TUI 的替代面，协议里也没有「TUI + ACP 旁路」通道；用它就要自研聊天与自研权限 UI——正好撞北极星。**对 v1 已钉路线，ACP 没有必须依赖的价值。**
- **帮得到（如果未来要结构化事件 / 换路线）**：① `usage_update`——token 用量（session 级）稳定，是 #32 统计的一个来源；② stopReason——轮次级完成/取消原因，可作完成信号素材（注意无 StopFailure）；③ `initialize` 能力协商 + session config options + modes——Agent Adapter 合同可参考的协商面；④ tool_call / plan / diff 通知——过程展示素材（Embedded Terminal 已有 TUI 展示，属锦上添花）。
- **帮不到或未成熟**：等待操作显式事件（v1 无，v2 Draft 才有）；per-turn token 明细（Draft）；远端 HTTP/WS（未成熟，Taskboard 本地也不需要）；Claude / Codex 官方无原生 ACP（挂第三方 adapter 是额外依赖与信任面）。
- 一句话：ACP 对「把官方 CLI 当黑盒子跑 TUI」的 Taskboard v1 几乎无用；它的价值在于「若哪一天要自研聊天/自研执行面」，那时是一份可用的协议地基——而那正是已钉路线明确不做的方向。是否因 codeg 借鉴 ACP 而改写 v1 规格，交给 [#40](https://github.com/youjiaxing/agent-taskboard/issues/40)。

## 附：证据索引

取证时间 2026-08-20。协议稳定版本 v1；v2 Draft（[公告 2026-07-20](https://agentclientprotocol.com/announcements/acp-v2-draft.md)）；规范仓库 `agentclientprotocol/agent-client-protocol` pushed 2026-08-20、4027★、创建 2025-06-23。

ACP 官方主源（全部 `.md` 后缀可直读）：
- 索引 [llms.txt](https://agentclientprotocol.com/llms.txt)；[introduction](https://agentclientprotocol.com/get-started/introduction.md)；[architecture](https://agentclientprotocol.com/get-started/architecture.md)；[agents](https://agentclientprotocol.com/get-started/agents.md)；[clients](https://agentclientprotocol.com/get-started/clients.md)；[registry](https://agentclientprotocol.com/get-started/registry.md)
- 协议 v1：[overview](https://agentclientprotocol.com/protocol/v1/overview.md)、[initialization](https://agentclientprotocol.com/protocol/v1/initialization.md)、[authentication](https://agentclientprotocol.com/protocol/v1/authentication.md)、[session-setup](https://agentclientprotocol.com/protocol/v1/session-setup.md)、[prompt-turn](https://agentclientprotocol.com/protocol/v1/prompt-turn.md)、[tool-calls](https://agentclientprotocol.com/protocol/v1/tool-calls.md)、[terminals](https://agentclientprotocol.com/protocol/v1/terminals.md)、[agent-plan](https://agentclientprotocol.com/protocol/v1/agent-plan.md)、[session-modes](https://agentclientprotocol.com/protocol/v1/session-modes.md)、[transports](https://agentclientprotocol.com/protocol/v1/transports.md)、[schema](https://agentclientprotocol.com/protocol/v1/schema.md)
- 用量：RFD [session-usage](https://agentclientprotocol.com/rfds/session-usage.md)、公告 [session-usage-stabilized](https://agentclientprotocol.com/announcements/session-usage-stabilized.md)（2026-06-05）；RFD [end-turn-token-usage](https://agentclientprotocol.com/rfds/end-turn-token-usage.md)（Draft，2026-06-02）
- v2（Draft）：公告 [acp-v2-draft](https://agentclientprotocol.com/announcements/acp-v2-draft.md)；RFD [v2/prompt](https://agentclientprotocol.com/rfds/v2/prompt.md)（state_update：running / idle / requires_action）

三家与 adapter：
- xAI：[CLI Reference](https://docs.x.ai/build/cli/reference)（`grok agent stdio`）、[headless-scripting](https://docs.x.ai/build/cli/headless-scripting)（ACP 章节）；本机 `grok 1.0.5 (5115b46bc909)` `grok agent --help`
- Codex：官方文档 [learn.chatgpt.com/docs/codex/cli](https://learn.chatgpt.com/docs/codex/cli)（无 ACP 入口）；[agentclientprotocol/codex-acp](https://github.com/agentclientprotocol/codex-acp)（README：stdio ACP server，启动 Codex App Server）；[zed-industries/codex-acp](https://github.com/zed-industries/codex-acp)（2026-07-22 归档，README 指向新仓库）
- Claude Code：[anthropics/claude-code issue #6686](https://github.com/anthropics/claude-code/issues/6686)（not_planned，2026-02-09 关闭）；[agentclientprotocol/claude-agent-acp](https://github.com/agentclientprotocol/claude-agent-acp)（README：包官方 Claude Agent SDK）
- 规范仓库：[agentclientprotocol/agent-client-protocol](https://github.com/agentclientprotocol/agent-client-protocol)（README + schema，与协议页不一致处以协议页/schema 为准）

起点文稿：
- #7 `research/agent-cli-surface`：commit `7e7f363`（远程分支已删，对象库可读）→ `docs/research/agent-cli-surface.md`
- #17 `origin/research/agent-kanban-models` → `docs/research/agent-kanban-models.md`（ACP 分类 A=PTY / B=stream-json·SDK / C=ACP；Emdash 双路径 = 两个入口）
- #39 `origin/research/codeg-vs-taskboard` → `docs/research/codeg-vs-taskboard.md`（codeg 用 ACP 与 adapter 包 SDK 的事实）
