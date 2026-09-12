# Project 切换先显示上次数据，Issue 写入按最小真源范围校验

Project 切换立即返回目标 Project 的上次数据或空加载状态，并由 Host 在后台合并为一次 Tracker 全量刷新；`focusProject`、`setClientView` 和前端请求队列都不得等待或重复这次刷新。人主动发起的 Issue 写入不再统一执行 Project 全量读取：创建、评论和幂等状态操作直接写 Tracker，正文修改先读取目标 Issue，父 Issue 与 Dependency 只读取该操作需要的关系数据；同字段冲突保留输入并要求重新编辑或明确覆盖。Tracker 仍是唯一真源，自动推进、绑定 Run 前认领及判断 Issue 是否关闭等自动决策仍须先成功完成涉及 Project 的全量读取。

本决策替代 [ADR 0014](./0014-tracker-refresh-offline-cache.md) 中“每次写入前须成功读取涉及的 Project”的统一门槛。刷新结果按 Project 隔离；后台读取开始后若有较新的 Issue 写入，旧结果不得覆盖该写入。当前 GitHub 与本地 Markdown Tracker 没有统一的原子版本写入，因此字段冲突校验是尽力而为，不包装成完全原子的保证。
