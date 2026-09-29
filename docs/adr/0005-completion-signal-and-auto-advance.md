# 完成信号与可选自动推进

官方 CLI 没有「任务做完了」信号。可选自动推进只在「票已关且状态正常」之后领下一张 `ready-for-agent`；默认关。状态正常 = 看见 SessionEnd、没有 StopFailure、进程正常退出。票没关或状态不正常才要求同一 Agent 自检，不每次多开一条复查 Run。看板不代关票。误判完成时宁可停下。

干活的 Run 按次挂只读 hook，不改用户家里的长期配置；Antigravity 只能在 Project 的 `.agents/hooks.json` 临时加入带 Run 唯一名的 recorder，结束或失败后恢复原配置；挂不上则这次不自动推进。进程还在且刚 StopFailure 时，往同一条官方 TUI 注入一句自检（与 Client 注入一行同一能力）；进程已退出时，新 CLI/PTY 只能用该 Run 保存的精确 Agent 原生会话 ID 恢复，不重放 Taskboard 启动默认。支持启动确认的 Agent 在两秒确认窗内等待 hook 回报同一个原生会话 ID；Antigravity 没有启动阶段 hook，使用原生 `--conversation` 恢复，并在第一次实际 invocation 时由 hook 回收会话 ID。缺少 ID、隔离 worktree 身份失效、hook 无法重新挂载、ID 不符或确认超时时，拒绝创建后继 Run 并把诊断留在来源 Run；不静默启动新会话。自检后仍不正常则停下。票已关但 hook 异常只验货、不自动 reopen。待确认 60 秒。grilling / prototype / needs-info / ready-for-human / needs-triage 不进自动池。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 人点完成才开下一张 | 交不出「可选自动推进」 |
| 每次干完再强制开一条复查 Run | 一切正常时多余 |
| Stop / exit 0 单独开下一张 | 一轮结束或接口挂了会被当成做完 |
| Agent 关票或约定评论单独当完成 | 幻觉关票会连锁；Tracker 分不清人和 Agent |
| 看板代关票 | 推断信号上改 Tracker，误判难挽回 |
| 不挂 hook，只看退出码 | 分不出 SessionEnd 与 StopFailure |

## Consequences

- `/to-spec` 按「状态正常 / 自检 / 待确认 / 自动池」写，不必再发明完成协议。
- Agent Adapter 必须声明能否按次注入 SessionEnd / StopFailure 门铃，以及能否往仍在跑的 PTY 写入一行。
- 有运行期 hooks 的 Adapter 在 Continue 时复挂 Host recorder，以收取恢复后的原生会话 ID 和生命周期信号；这些 recorder 不携带或重放用户启动表单配置。Antigravity 的 recorder 通过 Project 级 `.agents/hooks.json` 的临时命名 hook 注入，并在 Run 结束、启动失败或 Host 重启捡回时清理。
- [决策：Run 生命周期与 Issue 的绑定方式](https://github.com/youjiaxing/agent-taskboard/issues/9) 的「v1 不自动推进 Issue」收窄为：默认仍不推进；打开自动推进后走本 ADR。
- Host 冷启动后默认不推进。另有「冷启动后恢复自动推进」（默认关，且仅当自动推进开着才生效），到点后等 N 秒（默认 60）再按本 ADR 推进。见 [Host 常驻、配对与远程 Client](./0006-host-resident-pairing-remote-client.md)。
