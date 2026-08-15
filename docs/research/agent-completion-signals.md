# 调研：Agent「完成」信号与误判完成

- **服务决策票**: [#20](https://github.com/youjiaxing/agent-taskboard/issues/20)
- **Branch**: `research/agent-completion-signals`
- **Date**: 2026-08-15
- **Skill**: `research`
- **Scope**: 官方 CLI（Grok Build / Codex / Claude Code）和同类看板/编排器，如何区分「一轮说完 / 会话结束 / 进程没了 / 活做完了」；尤其是会不会把 API 中断或还在收尾误当成完成
- **资料原则**: 官方 hook 文档 + 仓库源码；不采二手榜单作证据
- **已决约束**（不重开）: Run 结束 ≠ Issue 完成；退出码 0 ≠ 任务完成；执行已停 ≠ 做完了开下一张；不能扫终端字；hook「停手」≠ 验收通过

---

## 1. 问题

用户要把可选自动推进做成：处理某张 Issue 的 Agent **完成且正常结束** 后，自动认领下一张 `ready-for-agent` 并开跑。最怕的是 **假阳性**——还没做完就被当成做完，包括：

- 正常：一轮回复结束，其实还在收尾 / 等人 / 准备下一轮
- 不正常：API 不稳中断、进程却像「正常退出」

须钉清事实（不是替 #20 拍板）：

1. 壳不重写 Agent 时，到底能看见哪些信号
2. 这些信号各自表示什么、会误报什么
3. 竞品把哪个信号当成「去做下一张」
4. 有没有人真正解决了「准确判断任务完成」

---

## 2. 结论摘要

**没有一家官方交互 TUI，也没有一家只包官方 TUI 的看板，能准确判断「这张 Issue 的活做完了」。**

能可靠分开的只有四层，必须当四件事，不能合并：

| 层 | 典型信号 | 能说明 | 不能说明 |
| --- | --- | --- | --- |
| 一轮结束 | `Stop` hook、Codex `notify` 的 `agent-turn-complete` | 模型这一轮不再吐字 | 会话还在；可能在等人、收尾、马上再开一轮 |
| API / 这一轮失败 | Claude / Grok 的 `StopFailure` | 这一轮是接口错误停的（限流、过载、账单、5xx） | 进程一定非 0 退出；下一轮会不会自己恢复 |
| 会话结束 | `SessionEnd` hook | 官方认为这次会话收摊了 | 人验收通过；工作做完 |
| 进程没了 | PTY / pid 退出 + 退出码 | 壳里的 CLI 进程不在了 | 同上；Grok 还可能有后台子代理没收完 |

包官方 TUI 的竞品（Cline Kanban、KanVibe、Emdash、Claude Squad）以及 **Claude-Code-Board**（包 stream-json）把「进程退出 / Stop」最多推到 **Review / IDLE / 等人**，**从不**据此关卡或自动开下一张。OpenHands 的 `FINISHED` 名字像做完，源码注释写的是「一轮结束，等人」。Routa 把 ACP `turnComplete` 命名成 `agent_completed`，默认就可以自动推列——这是误判完成的完整反例。会认真自动开下一张的产品，靠的是另一套它们自己拥有的协议（人点 Done、或 Agent `PATCH` 内部票为 `done`），而且 Paperclip 仍专门做了 watchdog，因为 **Agent 会谎报做完**。

---

## 3. 官方 CLI 能看见什么

### 3.1 Claude Code

主源：[Hooks reference](https://code.claude.com/docs/en/hooks)

官方把事件分成三种节奏：

- 每个会话一次：`SessionStart` / `SessionEnd`
- 每一轮一次：`UserPromptSubmit` / `Stop` / `StopFailure`
- 工具循环里反复：`PreToolUse` / `PostToolUse` 等

和「完成」最容易混淆的：

| 事件 | 官方定义 | 假阳性 |
| --- | --- | --- |
| `Stop` | 「When Claude finishes responding」；**每一轮**都触发 | grilling 问完一题、权限框弹出前、准备跑测试前，都会 `Stop`。官方写明 `Stop` 可拦截停手、让模型继续 |
| `StopFailure` | 「When the turn ends due to an API error」 | **这是接口不稳的专用信号**，不是完成。matcher 含 `rate_limit` / `overloaded` / `server_error` / `authentication_failed` / `billing_error` / `max_output_tokens` / `unknown`。输出和退出码被忽略 |
| `SessionEnd` | 会话终止 | matcher 原因是 `clear` / `resume` / `logout` / `prompt_input_exit` / `bypass_permissions_disabled` / `other`——**没有「任务成功」** |
| `Notification` | 系统通知 | matcher 含 `idle_prompt`、`agent_needs_input`、`agent_completed`、`permission_prompt`。`agent_completed` 仍是模型侧「我告一段落」，不是 Tracker 上的 Issue 完成 |
| `TaskCompleted` | Claude 自己的 Task 工具把内部 task 标完成 | 可被 hook 拦住；**不是**本产品的 Issue |
| `SubagentStop` | 子代理结束 | 父会话可能还在跑 |

`Stop` **不会**在用户打断时触发；API 错误走 `StopFailure`（见第三方对官方文档的归纳，与官方表一致：[Blake Crosley](https://blakecrosley.com/blog/claude-code-hooks-explained)）。

`SessionEnd` 默认只有约 1.5s 预算，适合记一笔「会话收了」，不适合做重验收。

### 3.2 Codex CLI

主源：[Hooks](https://developers.openai.com/codex/hooks)；Cline 仓库对 Codex `notify` 的第一手摘录：[`cline/kanban` `.plan/docs/hooks-update/codex-hooks-research.md`](https://github.com/cline/kanban/blob/main/.plan/docs/hooks-update/codex-hooks-research.md)

| 信号 | 定义 | 假阳性 |
| --- | --- | --- |
| `Stop` | 官方归在 **During a turn** | 一轮结束，不是会话结束 |
| `SessionEnd` | 「When the main thread ends」（子代理不跑） | 文档写 end reason **Currently only `other`**——分不清成功 / 崩溃 / 人退出 |
| `notify` / `agent-turn-complete` | 可通过 `codex -c 'notify=[...]'` 按次注入，不改用户全局配置 | 名字就写了 **turn** complete；payload 有 `thread-id` / `turn-id` / `last-assistant-message` |
| 官方 issue [#20603](https://github.com/openai/codex/issues/20603) | 「Stop hook is useful for turn completion/status, **it is not a reliable signal that the interactive Codex session/process has exited**」 | 集成方已踩过这个坑 |

Codex 公开 hooks 表里 **没有** 与 Claude `StopFailure` 对等的「这一轮是 API 挂了」事件。接口不稳时，壳更可能只看见进程退出或一个普通 `Stop`/`SessionEnd`。

### 3.3 Grok Build

主源：[Hooks](https://docs.x.ai/build/features/hooks)、[Changelog](https://x.ai/build/changelog)

| 信号 | 定义 | 假阳性 |
| --- | --- | --- |
| `Stop` / `StopFailure` | 「A turn ends, or ends with an API error」 | 与 Claude 同层：`Stop` = 一轮；`StopFailure` = 接口错误 |
| `SessionEnd` | 「A session starts or ends」 | changelog：非 leader TUI 和 headless 退出时会跑；**没有成功/失败原因表** |
| changelog 相关修复 | 「Fix agent waiting on background task and subagent completion」；「Don't auto-wake the model on cancelled/killed tasks or subagents」；Stop 会终止先前轮次的后台子代理 | 说明 **主进程看起来闲了，子活可能还在收尾**——正好对上「还在收尾就被当成完成」 |

Grok 的 `PreToolUse` 是唯一可拦截事件；`Stop`/`SessionEnd` 是被动观察。changelog 另写：Stop hook 现在可以喂回模型、让它别停——再次证明 `Stop` ≠ 任务完成。

### 3.4 进程退出码

三家都没有「exit 0 = 任务验收通过」的合同。Cline 源码把 exit 0 标成 `reviewReason=exit`，非 0 标 `error`，被打断标 `interrupted`——全部进入 **待审**，不进 Done。见 [`session-state-machine.ts`](https://github.com/cline/kanban/blob/main/src/terminal/session-state-machine.ts)。

API 中断会不会仍 exit 0：**没有官方保证**。所以只看退出码，会把「接口挂了但进程自己退了」收成「正常结束」。Claude/Grok 必须靠 `StopFailure` 才能把这类和「干净收摊」分开；Codex 这一层更盲。

---

## 4. 竞品怎么做

### 4.1 Cline Kanban — 退出只到 Review

源码：[`src/terminal/session-state-machine.ts`](https://github.com/cline/kanban/blob/main/src/terminal/session-state-machine.ts)、[`src/core/api-contract.ts`](https://github.com/cline/kanban/blob/main/src/core/api-contract.ts)

会话状态：`idle / running / awaiting_review / failed / interrupted`。

`process.exit`：

- `exitCode === 0` → `awaiting_review` + `reviewReason=exit`
- 非 0 → `awaiting_review` + `reviewReason=error`
- `interrupted` → 状态 `interrupted`

`hook.to_review` 同样只到 `awaiting_review`。

自动开下一张发生在卡片 **进 Done** 之后（`use-linked-backlog-task-actions` 的 `trashTaskAndGetReadyLinkedTaskIds`），不是进程退出时。可选 `autoReview` 会在 Review 列自动 commit/PR，再进 Done；有 **500ms** 去抖（`AUTO_REVIEW_ACTION_DELAY_MS`），防「还在收尾」的抖一下。

**对 Taskboard**：可抄「退出 / Stop = 待审，不是完成」；可抄按次注入 `notify`、不改用户家目录；应拒「exit 0 就开下一张」。

### 4.2 KanVibe — Stop → REVIEW，DONE 是另一回事

源码：[`src/lib/claudeHooksSetup.ts`](https://github.com/rookedsysc/kanvibe/blob/main/src/lib/claudeHooksSetup.ts)、[`src/lib/codexHooksSetup.ts`](https://github.com/rookedsysc/kanvibe/blob/main/src/lib/codexHooksSetup.ts)

写入各家 hooks：

- `UserPromptSubmit` → 列 **PROGRESS**
- Claude `PreToolUse(AskUserQuestion)` / Codex `PermissionRequest` → **PENDING**
- `Stop` → **REVIEW**（注释原文：响应完成，不是验收）

有独立的 `DoneStatusButton`。hook 服务 [`hookService.ts`](https://github.com/rookedsysc/kanvibe/blob/main/src/desktop/main/services/hookService.ts) 只是把 hook 上报的列名写进 SQLite，**不会**因为 Stop 写 DONE。

**对 Taskboard**：可抄「Stop = 忙闲/待看」；应拒把 hook 推列当成 Issue 完成。

### 4.3 Emdash — idle 故意丢掉，completed ≠ 关外部 Issue

源码：[`tui-agent-status-transition.ts`](https://github.com/generalaction/emdash/blob/main/apps/emdash-desktop/src/main/core/agent-status/tui-agent-status-transition.ts) 及同目录测试。

TUI Agent 状态：`working / completed / error / awaiting-input / idle`。

- `working` → `start`
- `completed` → 信号类型 `stop`（一轮停手）
- `awaiting-input` → `notification`
- **`idle` 显式不投影成任何事件**（测试名：`does not project idle as an AgentEvent`）
- 状态来自 hooks，**禁止扫终端字**（先前调研已引 `providers.md`）

外部 LinkedIssue 不因 Task/Session 结束而关。

**对 Taskboard**：可抄「idle ≠ 事件」「进程态和语义忙闲分开」；`completed` 在他们那里仍是 stop，不是关票。

### 4.4 Claude Squad — 退出 ≠ 任务完成

产品交互：`c` 提交并暂停、`s` 推分支、`D` 杀掉。进程退出本身不关任务。先前调研已记；与 Cline/KanVibe 同档。

### 4.5 Paperclip — 自己的 Tracker + Agent PATCH done，仍不信

主源：[`docs/guides/agent-developer/heartbeat-protocol.md`](https://github.com/paperclipai/paperclip/blob/master/docs/guides/agent-developer/heartbeat-protocol.md)、[`doc/execution-semantics.md`](https://github.com/paperclipai/paperclip/blob/master/doc/execution-semantics.md)、[`doc/TASK-WATCHDOG.md`](https://github.com/paperclipai/paperclip/blob/master/doc/TASK-WATCHDOG.md)

完成信号是 **Agent 调用他们的 API**：

```http
PATCH /api/issues/{id}
X-Paperclip-Run-Id: {runId}
{ "status": "done", "comment": "..." }
```

Run 活跃（`queued/running/succeeded/failed/timed_out/cancelled`）和 Issue 状态是两套。心跳协议要求：先 checkout，再干活，再 PATCH。

即便如此，watchdog 文档第一段就写：Agent 会「**declaring done without proof**」、误读阻塞、可恢复失败却放弃。所以他们又做了：

- **Task watchdog**：整棵子树停住后，换一个 Agent 复查「这次停是否合法」
- **Silent active-run watchdog**：进程还在但一段时间没输出
- **Liveness recovery**：`in_progress` 但没有活路径

**对 Taskboard**：这证明「让 Agent 自己报完成」也解决不了假阳性，只是换了一种谎报方式。本产品若抄 `gh issue close` 当完成，风险同类。地图已把心跳编排器划出范围。

### 4.6 Cyrus — 「做完」= 验证脚本 + 开 PR，不是进程退出

技能 [`skills/verify-and-ship/SKILL.md`](https://github.com/cyrusagents/cyrus/blob/main/skills/verify-and-ship/SKILL.md)：跑测试 / lint / typecheck（失败最多重试 3 次），再 commit、push、开草稿 PR。派活是「指派即跑」，地图已拒。

他们把完成定义成 **交付物**（PR 在），不是 hook。假阳性变成「测试绿了但活不对」；接口中断则根本走不到 ship。

### 4.7 Claude-Code-Board — 进程退出只回 IDLE，COMPLETED 必须人手点

源码：[`backend/src/types/session.types.ts`](https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/types/session.types.ts)、[`backend/src/services/SessionService.ts`](https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/services/SessionService.ts)、[`backend/src/services/ProcessManager.ts`](https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/services/ProcessManager.ts)、[`UnifiedStreamProcessor.ts`](https://github.com/cablate/Claude-Code-Board/blob/master/backend/src/services/UnifiedStreamProcessor.ts)

对接：`npx claude-code -p --output-format=stream-json`（自研聊天包一层，不是官方 TUI）。

Session 状态：`processing / idle / completed / error / interrupted / crashed`。

结束判定：

- 子进程 `close` → `processExit`。`code === 0` 时 **故意保持 IDLE**。源码注释原文：「不再将 code === 0 的情况设为 COMPLETED；COMPLETED 状态应该只在用户明确结束 session 时才设置」。
- `code !== 0` → `ERROR`。
- stream-json 的 `result` 类型 **直接忽略**（注释写「如 vibe-kanban」）。
- `completeSession()` 只有人手调用，且要求当前已是 IDLE 或 ERROR。

**对 Taskboard**：和 Cline 同档，而且写得更白——exit 0 曾经被他们当成 COMPLETED，后来改掉了。

### 4.8 OpenHands — `FINISHED` 是一轮说完，在等人

源码：Canvas 侧 [`ExecutionStatus`](https://github.com/OpenHands/OpenHands/blob/main/src/types/agent-server/core/base/common.ts)、[`use-agent-state.ts`](https://github.com/OpenHands/OpenHands/blob/main/src/hooks/use-agent-state.ts)；真正改状态在 SDK [`response_dispatch.py`](https://github.com/OpenHands/software-agent-sdk/blob/main/openhands-sdk/openhands/sdk/agent/response_dispatch.py)、[`acp_agent.py`](https://github.com/OpenHands/software-agent-sdk/blob/main/openhands-sdk/openhands/sdk/agent/acp_agent.py)。ACP 说明见 [`docs/ACP_AGENTS.md`](https://github.com/OpenHands/OpenHands/blob/main/docs/ACP_AGENTS.md)。

对接：自研 Agent Server，或 ACP 子进程包 Claude/Codex/Gemini（**不是**官方 TUI）。

`ExecutionStatus`：`idle / running / paused / waiting_for_confirmation / finished / error / stuck`。

结束判定：

- 内置 Agent：模型吐出一段纯文本、不再调工具 → `FINISHED`。注释原文：「LLM produced a message response - awaits user input」。
- ACP 路径：`conn.prompt()` 返回（一轮远程 turn 结束）→ 发 `FinishAction` → `FINISHED`。源码写明这是 **turn 边界**，不是整段 Conversation 验收。
- 没有用户消息可送 → 也标 `FINISHED`（「No user message found; finishing conversation」）。
- ACP **idle timeout**（一段时间没有任何 `session_update`）→ `ERROR`，文案承认「也可能已经做完但响应没收到」——他们选择宁可当错误，也不当完成。
- `STUCK` 单独存在，Canvas 把它画成 error，不当完成。
- `IDLE` 在 UI 映射成 `AWAITING_USER_INPUT`，不是关 Conversation。

**对 Taskboard**：`FINISHED` 这个词会骗人，语义是「这一轮停手，等人」。接口挂 / 僵死走 `ERROR`/`STUCK`，不自动开下一张。

### 4.9 Routa — ACP `turnComplete` 被命名成 `agent_completed`，默认可自动推列

源码：[`agent-event-bridge.ts`](https://github.com/phodal/routa/blob/main/src/core/acp/agent-event-bridge/agent-event-bridge.ts)、[`http-session-store.ts`](https://github.com/phodal/routa/blob/main/src/core/acp/http-session-store.ts)、[`workflow-orchestrator.ts`](https://github.com/phodal/routa/blob/main/src/core/kanban/workflow-orchestrator.ts)、[`lifecycle-notifier.ts`](https://github.com/phodal/routa/blob/main/src/core/acp/lifecycle-notifier.ts)、[`board-session-supervision.ts`](https://github.com/phodal/routa/blob/main/src/core/kanban/board-session-supervision.ts)、[ADR 0004](https://github.com/phodal/routa/blob/main/docs/adr/0004-kanban-driven-automation.md)。

对接：ACP session（create / prompt / cancel / reconnect），列切换排队开 session。

他们自己把两层分开写了：

- `notifyIdle`：「一轮结束、没活了」
- `notifyCompleted`：「所有指派的活做完了」

但 ACP 桥把 `session_update.turnComplete` **直接映射成** `agent_completed`（带 `stopReason`）。编排器默认 `completionRequirement: "turn_complete"`；非 `ralph_loop` 模式下，成功事件就当 `completionSatisfied`。`autoAdvanceOnSuccess` 为真时，会自动推下一列 / 开下一张排队卡。

另有：

- 无活动超过 `inactivityTimeoutMinutes`（默认 10）→ `AGENT_TIMEOUT`，可 watchdog 重试
- ACP `error` → 失败，不推进
- `ralph_loop` 可把门槛升到「必须有 completion_summary / verification_report」——仍是 Agent 自报
- 仓库自己的 issue（2026-03-14）写过：编排器等的是 `AGENT_COMPLETED`，但普通 Kanban ACP 会话主要靠 session 状态，两边曾对不齐导致泳道卡住

**对 Taskboard**：这是「把一轮结束当成完成、再自动往下走」的完整反例。他们后来用 10 分钟闲置和可选摘要门槛补洞，说明默认 `turn_complete` 会误判。

### 4.10 其它（先前 #17 已覆盖，不重复深挖）

Routa 列切换 / Fitness Gate、Vibe Workspace Idle、Nimbalyst 人标 complete：完成权都在人或另一套自研会话上，不在官方 TUI 退出码。

---

## 5. 误判场景对照（按用户担心的两类）

| 场景 | 壳会看见什么 | 若把 Stop 或 exit 0 当完成 | 较稳的读法 |
| --- | --- | --- | --- |
| 一轮说完，还要跑测试 / 写收尾 | `Stop`；进程仍在 | **假阳性** | 进程还在 = 未结束 |
| grilling / 权限询问 | `Stop` 或 `Notification(permission_prompt\|agent_needs_input)`；进程在 | **假阳性** | 等人，不是完成 |
| 子代理 / 后台命令还在收 | 主会话 `Stop` 甚至像 idle；Grok 曾因此修过 bug | **假阳性** | 要等 `SessionEnd` + 宽限期，最好再看子代理 hook |
| 用户 `/exit`，活只做了一半 | `SessionEnd(prompt_input_exit)` + 常为 exit 0 | **假阳性** | 会话结束 ≠ 完成；缺第二道门 |
| API 限流 / 5xx | Claude/Grok：`StopFailure`；进程可能仍在或随后 0 退出。Codex：多半只有 Stop/退出 | **假阳性**（只看出码时） | 见过 `StopFailure` → 执行已停，禁止开下一张 |
| 进程被杀 / Host 崩溃 | 非正常退出；无干净 `SessionEnd` 或原因是 `other` | 若只看「没有活跃 Run」会误判 | 已钉：执行已停 |
| Agent 自己 `gh issue close` | Tracker 已关；凭据是用户的，分不清人和 Agent | 关错票会连锁开下一张 | 关票单独不能当完成信号 |
| Agent PATCH/评论「我做完了」 | 结构化，但仍是模型自报 | Paperclip 已证明会谎报 | 最多当弱证据，不能单独开下一张 |

---

## 6. 可抄 / 应拒（只给 #20 grilling）

**可抄**

1. 四层分开：一轮结束 / API 失败 / 会话结束 / 进程退出。完成是第五层，官方 CLI 不提供。
2. 按次注入只读 hook（Cline 对 Codex 的 `-c notify=...`；Claude `--settings`），不改用户家里的全局 hooks。
3. `Stop` / `idle` / `notify(turn-complete)` 只更新忙闲 UI。
4. `StopFailure`（Claude/Grok）→ 标异常，禁止自动推进。
5. 自动开下一张若要做，至少 `SessionEnd` ∧ 进程正常退出 ∧ 短宽限期无新活动；仍接受「人提前退出」这类假阳性，除非再加第二道门。
6. 去抖 / 宽限期（Cline 500ms；Grok 子代理收尾）专门对付「还在收尾」。

**应拒**

| 应拒 | 原因 |
| --- | --- |
| `Stop` / `idle` / exit 0 单独开下一张 | 官方和 Cline/KanVibe/Emdash 都当一轮或待审 |
| 扫终端找 “done” / “完成” | 已钉禁止；误报更多 |
| 把 Agent 关 Issue 单独当完成 | 与人关票不可分；幻觉关票会连锁 |
| 发明跨三家的「我做完了」协议当充分条件 | Paperclip 有真协议仍要 watchdog |
| 没有 hook 能力的 Agent 用更松的门闩 | 应更严（只看出码 → 不要自动推进），不是更松 |

---

## 7. 给 #20 的短答

壳能准确判断的是 **「这次会话是不是干净地停了」**，不能准确判断 **「活做完了」**。

用户要的无人串行，竞品里没有「又包官方 TUI、又零假阳性自动开下一张」的先例。能做的是把假阳性压到「会话确实收摊了」这一层，并用 `StopFailure` + 宽限期挡住接口中断和收尾中；剩下的「人提前退 / Agent 自以为做完」要么再加第二道门，要么接受。

---

## 8. 证据索引

| 源 | 用途 |
| --- | --- |
| https://code.claude.com/docs/en/hooks | Claude 事件节奏、`Stop`/`StopFailure`/`SessionEnd`/`Notification` |
| https://developers.openai.com/codex/hooks | Codex `Stop` 属 turn、`SessionEnd` reason 仅 `other` |
| https://github.com/openai/codex/issues/20603 | Stop ≠ 会话/进程退出 |
| https://docs.x.ai/build/features/hooks | Grok `Stop`/`StopFailure`/`SessionEnd` |
| https://x.ai/build/changelog | Grok 子代理收尾、Stop 可续跑 |
| https://github.com/cline/kanban `session-state-machine.ts` | exit 0 → awaiting_review，不是 Done |
| https://github.com/cline/kanban `.plan/docs/hooks-update/codex-hooks-research.md` | `notify` = `agent-turn-complete`，可按次注入 |
| https://github.com/rookedsysc/kanvibe `claudeHooksSetup.ts` / `codexHooksSetup.ts` | Stop → REVIEW |
| https://github.com/generalaction/emdash `tui-agent-status-transition.ts` | idle 不投影；completed → stop |
| https://github.com/paperclipai/paperclip heartbeat-protocol / TASK-WATCHDOG | PATCH done + 仍不信自报 |
| https://github.com/cyrusagents/cyrus `skills/verify-and-ship/SKILL.md` | 完成 = 验证 + PR |
| https://github.com/cablate/Claude-Code-Board `SessionService.ts` | exit 0 → IDLE；COMPLETED 仅人手 |
| https://github.com/OpenHands/software-agent-sdk `response_dispatch.py` / `acp_agent.py` | FINISHED = 一轮结束等人；idle timeout → ERROR |
| https://github.com/phodal/routa `agent-event-bridge.ts` / `workflow-orchestrator.ts` | turnComplete → agent_completed；默认可自动推列 |
| 本仓库 #9 / #10 / #17 | Run≠Issue；执行已停；编排应拒项 |
