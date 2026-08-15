# Tracker Adapter 统一能力面

三个 Tracker 的原生能力不对齐，但不能让内核在某一端假装「没有阻塞」或「没有孩子」。内核把 Issue 读写、认领、评论、Dependency、父 Issue、分类记号定为必选：有原生 API 用原生，否则只认该端已有约定（正文 `Blocked by` / `Part of`、本地目录与 `Status`/`Type` 字段）。同一能力在同一 Project 上只有一份真源，禁止原生和正文双写。

## 必选与可选

**必选：** 列表与读取、创建、改标题正文、开关票（或写成终态）、认领/放领、追加评论、读写真源阻塞边、读写真源父子、分类记号（GitHub/GitLab 为标签；本地经 Label Mapping 落到 `Status`/`Type`，不发明多标签列表）。

**可选（无则藏，不降级成假数据）：** 相关但不挡、跨 Project 写依赖、跳进对端 Project、原生阻塞计数、Issue Type / 自定义字段、GitLab Epic/Task 树、推送通知。

## 真源与 Frontier

- Dependency 与父 Issue 是两套边，互不冒充。
- 原生可用时只信原生；正文只在探测到原生不可用时当真源。
- Frontier 按 Project 全量计算；父只是视图过滤。
- 读到跨 Project 阻塞就计入门闩；对端状态看不清则该票不进 Frontier。
- 「相关」不进内核，也不挡 Frontier。
- 每个 Project 记录阻塞/父子当前走原生还是约定，并在连接状态里可见。

## Considered options

| 选项 | 未采纳原因 |
|------|------------|
| 没有原生阻塞就不做依赖 | 关掉北极星能力 |
| 算不准就整板标未知 | 日常板经常不可用 |
| 标签/父子做成可选、没有就藏 | GitLab `Part of` 与本地目录树明明读得到 |
| 本地发明 `Labels:` 列表 | 不是既有约定 |
| 原生与正文双写 | 并发下两份真源必漂 |
| 只信正文 | 浪费 GitHub/GitLab 一等能力 |
| Frontier 相对当前父 | 没挂父的票从可干列表消失 |
| 丢掉跨 Project 边 | 在 GitLab 上隐瞒真实阻塞 |

## Consequences

- 各 Tracker Adapter 必须能探测并固定该 Project 的真源，失败时走约定，不能静默当成「无边」。
- 规格与 UI 不得把父树画成阻塞，也不得把 relates_to 画成门闩。
- `/to-spec` 按此能力面写 Adapter 合同即可，不必再为三端各编一套领域模型。
