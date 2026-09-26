# 归档 Run 通过 Project 墓碑恢复原归属

移除 Project 时，Host 先在 `runs.json` 中将该 Project 的已结束未归档 Run 批量归档并清除置顶，再在 `settings.json` 的同一次原子替换中删除 active Project 并写入 `projectTombstones`。墓碑只保存原 Project ID、名称、本地目录、Tracker 类型、GitHub host/repository 和不可复用的 revision，不保存凭据。旧设置没有该字段时按空列表读取。含活跃 Run 的 Project 不允许移除。

归档 Run 的恢复先由 `prepareRestoreRun` 预检。Project 仍存在时可直接恢复；Project 缺失时，Host 返回墓碑展示信息、revision，以及墓碑缺失、目录缺失、目录已属于另一 Project 或 active Project 与墓碑同 ID 等结构化冲突。Run 的 `workingDirectory` 可能指向隔离执行目录，不能用于猜测或改绑 Project。

`restoreRun` 在 Project 缺失时要求 `confirmProjectRecreate: true` 和匹配的 `expectedTombstoneRevision`。Host 在持有内核锁时重新执行预检，使用墓碑中的原 Project ID 重建登记。Project 与墓碑不能以同一 ID 同时作为正常状态；加载到这种旧数据时，Host 在快照中暴露 `dataConflicts`，恢复保持阻塞，等待人工修复设置文件。

重建采用两个有序提交：先在一次 `settings.json` 原子写入中添加 Project 并删除墓碑，成功后更新内存；再写 `runs.json` 清除同一 Run 的归档标记。第二步失败时保留已重建 Project 和已归档 Run，重试只完成剩余的 Run 写入。移除的第二阶段若失败，则保留 active Project 和已归档 Run，重复移除只完成设置写入。多个 Client 的请求由同一 HostKernel 锁串行化，并在锁内重检；首个请求完成重建，后续请求只取消尚未取消的归档或返回已恢复，因此不会产生重复 Project 或 Run 改绑。

Tracker 探测失败或暂时离线不会撤销已确认的本地登记。重建 Project 保留探测得到的连接状态，后续正常刷新继续收敛 Tracker 数据。

## Consequences

- Host protocol 提供 `projectRestore` capability、`runRestore` 结构化结果和远程转发；Client 页面与确认弹窗不在本决策范围内。
- 墓碑必须参与每一条 Host settings 写入路径，避免修改其它设置时被遗漏。
- Host 启动或成功恢复 `runs.json` 后，会把所属 Project 已不存在的未归档 Run 自动归档并清除置顶；不会凭空创建缺失的 Project tombstone。
- 本决策不实现 `runs.json` 损坏或缺失时的基础数据恢复模式，也不改变 Run 置顶、归档、认领、自动推进或 PTY 生命周期。
