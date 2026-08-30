# Tracker 只在可见或动作节点刷新，上次数据只供展示

Issue Tracker 是 Issue 状态与认领的唯一真源。Host 为每个 Project 保存一份**上次数据**，但这份数据只用于先画出看板和离线查看；判断 Issue 是否关闭、认领或自动推进前，必须先成功读取 Issue Tracker，不能拿上次数据执行动作。

## 刷新时机

- 打开或切到 Project、可见窗口或标签回到前台、手动刷新时，立即刷新。
- 至少有一个可见 Client 正显示该 Project 时，默认每 60 秒刷新；间隔可设置。界面必须显示下一次自动刷新的倒计时，具体样式由原型决定。
- 没有可见 Client 正显示 Project 时，不为展示而轮询。
- 绑定该 Project 的 Run 结束时立即刷新，不受 Client 是否可见影响。
- 认领、判断 Issue 是否关闭、自动推进认领下一张，以及冷启动恢复自动推进时，只刷新涉及的 Project；其它已登记 Project 不跟着刷新。
- local markdown 文件内容变化触发刷新；Host 在自己的 tick 上检测已登记 Project 的内容 revision，不要求窗口位于前台。v1 不依赖 webhook 作为远端 Tracker 的主路径。

## 上次数据

Host 按 Project 持久保存最近一次成功刷新得到的数据：所有未关闭 Issue 的列表摘要、认领、标签、父 Issue、Dependency，以及最近完成 N 张。正文和评论不要求全部进入快照，查看详情时可以按需读取。

快照没有按时间自动删除的期限。刷新成功后整份替换；刷新失败、离线或限流时保留原快照。它位于 Host 数据根下的 `projects/<id>/tracker-snapshot`；macOS 与 Windows 的 Host 数据根由“决策：设置与数据存放位置”确定。

## 离线与限流

刷新不清空当前看板。有上次数据时继续展示，并明确标出数据截至时间：

- 刷新进行中：旧数据仍可见，同时显示正在刷新。
- 连不上 Tracker：显示 Project 已离线及上次成功时间。
- 429 或次级限流：只暂停该 Project 的自动刷新；优先按服务端给出的 `Retry-After` 或重置时间恢复，并显示大约何时可再刷新。没有明确恢复时间时保持暂停，直到手动刷新成功。人可以手动试一次；再次限流则继续暂停。
- 401、403 或凭据失败仍按“决策：本机凭据与远端鉴权策略”做项目级降级，不混写成离线或限流。
- 从未成功刷新且没有上次数据时，不展示像是真实数据的四列。

离线或限流未恢复时，人可以查看上次数据、查看或停止已有 Run、向已有 Run 输入，以及启动游离 Run。Host 不认领、不放领，自动推进不领下一张；需要先认领才能启动的绑定 Issue Run 不启动。写操作不排队，也不先修改上次数据。

## Tracker 写入边界

从 #111 起，桌面与电脑浏览器 Client 可通过 Host 使用 [Tracker Adapter 统一能力面](./0002-tracker-adapter-capability-surface.md) 的必选写操作：创建、改标题正文、开关票、认领/放领、评论、父 Issue 与 Dependency。每次写入前仍须成功读取涉及的 Project；写入直接落到 Tracker 真源，失败则保留表单输入并显示错误，不先改上次数据、不离线排队。认领失败仍不得启动对应绑定 Run。

Agent 在 Embedded Terminal 中通过 `gh`、`glab` 或其它方式写 Tracker，仍是 Agent 自己的行为；看板不拦截，也不把 Run 结束、拖列或本地投影当成 Tracker 写入。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| Host 常驻时轮询所有 Project | v1 没有跨 Project 总览；没人看时会无谓消耗远端额度 |
| 完全不轮询，只靠切换和手动刷新 | 人持续看板时，Tracker 上的变化不会自动出现 |
| 上次数据也参与自动推进 | 远端 Issue 可能已经关闭或删除，会重复认领或启动 Run，形成第二真源 |
| 不持久保存上次数据 | Host 重启或断网后只能展示空白，无法说明最近一次已知数据 |
| 离线写入排队，恢复后回放 | 对端可能已经变化，需要另一套冲突与同步模型 |
| 刷新时清空看板 | 慢网和失败时无法继续查看最近一次已知数据 |
| 把限流当成离线或鉴权失败 | 原因和恢复方式不同，会误导用户并影响不相关 Project |

## Consequences

- Tracker Adapter 的读取结果必须带成功时间，并区分离线、限流和鉴权错误。
- 自动推进的每个判断与认领动作都以涉及 Project 的一次成功读取为前置条件。
- Client 需要展示数据截至时间、刷新中、下次自动刷新倒计时、离线和限流恢复时间；具体摆位与视觉层级采用“刷新状态栏”。
- 上次数据属于 Host 数据，不属于任何 Client，也不是可编辑的 Issue 数据库。

这钉住了 [决策：Tracker 刷新、离线与速率限制](https://github.com/youjiaxing/agent-taskboard/issues/29)，视觉层级由 [原型：刷新倒计时与离线/限流提示](https://github.com/youjiaxing/agent-taskboard/issues/33) 定稿。
