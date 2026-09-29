# Host 归档同步 Agent 原生会话

Host 的「归档 Run」仍以 `runs.json` 中的 `archivedAtMs` 为本地真源，但对支持该能力的 Agent Adapter 同步调用 Agent 官方 CLI 的原生会话归档。Host 先提交本地整理状态和同步意图，再由后台 worker 执行外部 CLI；外部失败不回滚 Host。

`RunSummary.nativeSync` 是兼容旧 Client 的可选摘要，包含目标状态、同步状态、尝试次数、下次重试时间和最后错误。revision、启动执行上下文和 lease 只存在于同一条 `runs.json` 记录的 Host 私有字段中，避免独立 outbox 文件与 Run 状态跨文件不一致。旧版直接保存的 `RunSummary[]` 可读取；没有私有启动上下文的历史 Run 不用当前环境猜测执行。

Codex Run（包括不绑定 Issue 的 Run）统一安装 hook recorder。recorder 将 hook stdin 中的 `session_id` 与 SessionEnd、StopFailure、等待用户信号合并写入单个原子 hook 状态文件；Host 在运行期间和启动恢复时反复读取，避免 hook 到达顺序造成会话 ID 丢失。

原生同步只保存绝对可执行文件或绝对解释器加脚本参数、稳定工作目录回退位置和最小的非敏感配置根路径，不保存完整启动环境、PATH 或凭据。CLI 缺少明确子命令时标记 `unsupported`；没有可靠会话 ID 时标记 `missing-native-session`；泛化非零退出标记 `failed` 并有限退避，不猜测会话不存在。Host 崩溃后将 `syncing` 恢复为 `pending`，每个 Run 通过持久 lease 和 revision 避免旧 worker 与新 Host 反向覆盖。

## Consequences

- Host 归档/恢复和 Project 批量归档继续保持本地提交优先、批量写入一次。
- 非 Codex 或未声明原生同步能力的 Agent 不受外部 CLI 影响，只显示 `unsupported`。
- 归档同步不会阻塞 snapshot、observe 或 RPC；用户可显式重试终态同步。
- Codex Desktop 是否立即刷新 SQLite/rollout 变化仍需真机验证。
