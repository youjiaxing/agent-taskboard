# 默认主壳用 Codex 气质骨架，Host 在左侧，底栏跟当前 Issue

v1 每天盯的那一屏学 Codex Desktop 的气质和折叠方式，不学它的产品模型。工作单元仍是 Issue；中间默认四列看板；Embedded Terminal 仍跑官方 CLI。

**左侧栏**列出这台窗口连上的 Host（一次一台）和当前 Host 上的 Project，Project 下列进行中的 Run。Host 不再用顶栏切换器。

**中间**只跟当前 Project：看板 | 依赖图。列序仍是阻塞中 → Frontier → 进行中 → 最近完成。

**总览**跟 Host，不跟当前 Project：铺开这台 Host 上的 Run 缩略图，按终端状态分组，可按 Project 过滤。不是 Frontier 聚合板，也不是跨 Host 总览。入口在左侧 Host 区；侧栏收起后留在顶栏。

**底栏**跟当前选中的 Issue：有进行中的 Run 就切到那条 PTY；没有就收起终端，给出「开 Run」（一步认领并启动）。这台 Host 上的全部 Run 到总览或「所有 Run」里找。

**进入 Run**时，从看板点击已有 Run 的 Issue 会直接把 Embedded Terminal 抬到中间，自动收起左侧，右侧保留完整 Issue；终端约占中间与详情区域的三分之二。「返回看板」恢复原看板和自动收起的左侧。依赖图节点仍只换 Issue 详情，不触发这条流转。

**手机**默认只突出当前 Project、刷新状态、进行中和 Frontier；Host / Project 清单收进「切换」面板。底栏是「看板 | 票 | Run」。进行中优先展示，并提供打开 Run；手机仍不以完整官方 TUI 为验收。

**折叠**把区域从布局里拿掉，不留占位条。左侧和 Issue 的开关钉在不会消失的顶栏上，收起后坐标不变。

这改写了 [决策：主界面信息架构定稿](https://github.com/youjiaxing/agent-taskboard/issues/15) 里「Host 顶栏切换器」和「底栏列出这台 Host 全部 Run」；能力仍在，摆位变了。钉于 [原型：对照 Codex Desktop 再调默认主壳](https://github.com/youjiaxing/agent-taskboard/issues/31)。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 把 Codex 的会话列表和聊天当首页 | 产品模型是 Issue，不是线程 |
| 不改 #15 结构、只换气质 | 学不像；提出者要结构可以改、已钉能力不能丢 |
| 总览和看板、依赖图并排 | 看板/图跟当前 Project，总览跟 Host |
| 折叠后留一条身份条 | 仍占布局；Codex 是整栏消失，只留开关 |
| 自动推进只记在 Host 上 | 提出者要跟 Project；Host 只留总开关 |

## Consequences

- `/to-spec` 按「左侧 Host/Project、中间看板|图、Host 级总览、底栏跟 Issue、看板点 Run Issue 直达中间终端、折叠不占位」写，不要写回顶栏 Host 条或默认列出全部 Run 的底栏。
- 手机按当前 Project 聚焦，不平铺全部 Host / Project；切换范围是次级面板。
- 四列、依赖图、空状态三件套、最近完成 N、查看改动、Embedded Terminal 官方 CLI 仍按已钉能力做。
