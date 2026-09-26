# Run 置顶与归档是 Host 持有的正交整理维度

Run 执行状态仍只有 `starting | running | ended`，`waitingForUser` 仍是活跃属性。Host 在持久 Run 记录上用可缺省的 `pinnedAtMs` 与 `archivedAtMs` 保存整理元数据；旧记录缺字段时按未置顶、未归档读取。产品文案统一使用“置顶 / 取消置顶”。

未归档的活跃或已结束 Run 都可置顶。置顶归属于 Run 所在的 Project，只在该 Project 的 Run 导航和历史中生效；置顶 Run 排在未置顶 Run 前，置顶组按 `pinnedAtMs` 倒序，未置顶组保持原页面顺序。Project 和 Issue 历史持续显示置顶标志，不提供脱离 Project 的 Host 级置顶页。只有 `ended` Run 可以归档；活跃或等待操作 Run 的归档请求明确失败且不得停止进程。归档清除置顶，恢复只清除归档，不启动进程、不恢复 PTY、不改变执行状态。

Host 内部保留完整 Run 集合，默认快照只投影未归档 Run。归档详情通过 `listArchivedRuns` 按需读取；普通导航、控制、通知跳转、Issue Run 引用和用量明细不能产生归档 Run 句柄。归档当前聚焦 Run 时，Host 自身与各 Client 导航镜像都清除该 Run，无目标的 Run 工作区回到 Project。

移除 Project 前必须没有活跃 Run；移除时将该 Project 的已结束未归档 Run 批量归档并清除置顶，保留 Run 历史和原 Project 墓碑。归档写入先于 Project/tombstone 设置写入；后者失败时 Project 仍保留，Run 已归档，重复移除可以重试剩余步骤。

归档不改变 Issue 认领、执行已停、Continue、自动推进、telemetry、隔离目录或历史关联。用量 totals 继续包含归档 Run 样本；普通归档保留 pending confirmation 的来源 Run ID 作为项目级非导航历史，可继续倒计时与 veto。移除 Project 会同时取消该 Project 的 pending confirmation，因为它已没有可继续推进的 Project。

`runOrganization` capability 属于当前 focused Host。远程 Host 快照独立携带并缓存该能力；缺字段按不支持处理，不能回退到本机 Host 的能力。这样新 Client 面对旧 Host 会隐藏整理操作，旧 Client 面对新 Host 也只会看到已过滤的默认 Run 快照。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 把 archived 加入 `RunStatus` | 会把整理维度误写成执行生命周期，并污染 active / ended 判断 |
| 归档活跃 Run 时自动停止 | 停止是独立危险操作，归档不能产生执行副作用 |
| 每次快照携带全部归档历史 | 归档历史可持续增长，普通导航不需要这份数据 |
| 归档后从用量汇总移除 | 会改写已经发生的 token 消耗事实 |
| 远程 Host 缺 capability 时沿用本机能力 | 会让 Client 对旧 Host 显示无法执行的操作 |
| 提供脱离 Project 的 Host 级置顶页 | 会破坏 Run 与 Project 的归属关系，并让用户难以判断置顶影响范围 |

## Consequences

- [完成信号与可选自动推进](./0005-completion-signal-and-auto-advance.md) 的 pending confirmation 不因来源 Run 归档而改变。
- [主壳使用固定区域、内部页面状态和当前 Host 侧栏](./0015-codex-shell-ia.md) 的普通 Run 导航只消费未归档投影；置顶只在 Project/Issue 历史中生效，归档页面仍服从当前 Host 作用域。
- [Host 用量页同时看 token 账和通路快慢](./0016-host-usage-page.md) 的 totals 包含归档样本，默认流水排除归档 Run。
- Project 墓碑重建由 [0021](./0021-project-tombstone-run-restore.md) 补充；`runs.json` 基础数据恢复模式以及桌面/手机页面仍由后续票实现，不属于本决策的 Host 核心合同。
