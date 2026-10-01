# 并行 Run 默认共用主目录，隔离只走 Agent 原生 git worktree

同一 Project 里多个 Run 可以并行，默认都在 Project 绑定的本地工作区。隔离是单次 Run 的显式选择，且只在该 Agent Adapter 能把官方 CLI 的建树开关传出去时可用。v1：Grok Build / Claude Code 传 `--worktree`（不代起名字）；Codex CLI 没有对等开关，隔离不可用，等官方。看板不执行 `git worktree add`，也不做通用 worktree 工作台。不分配端口或本地数据库，也不因并行本身显示推测性风险提醒。启动表单只提示已确认共用工作目录的文件覆盖风险，以及实际检测到的 Project 主目录 `.git/index.lock`；这些提示不禁止启动。看板只记录这次 Run 用上的那棵树，并提供显式删除；何时删树沿用各家 CLI 默认。「继续」不再传 `--worktree`，只回到仍属于原 Project 仓库的已记录 worktree；目录缺失或身份变化时拒绝继续，不回退到主目录。

## Considered options

| 选项 | 未采纳原因 |
|------|------------|
| 每个 Run 一律由看板建 worktree | 与 Grok/Claude 原生 `--worktree` 双层；非 git 痛；看板要自管一整套生命周期 |
| 永远共用主目录，只靠 sandbox | sandbox 限制的是写出项目外 / 上网，挡不住两个 Run 改同一文件 |
| 看板给 Codex 建树，Grok/Claude 走原生 | 看板仍要自建创建与清理；Codex 隔离也没有 Claude 那种禁止改回主目录的墙；决定等官方 CLI |
| 按 Issue 类型自动隔离 | research / grilling 也会改文档，类型不是「会不会写文件」 |
| 容器或独立 Workspace 对象 | 超出 Embedded Terminal + 本机官方 TUI；比 Run 更重 |

## Consequences

- Agent Adapter 必须声明能否原生创建隔离执行目录；不能则开关禁用，原因放在二级隐藏提示里，不在旁边平铺。
- 启动表单第一层有隔离开关：默认关，不按 Project×Agent 记忆，不按票类型预勾。说明里写明机制是 git worktree。
- 共用目录提示仅针对同一 Project 的活跃 Run：本次没有有效隔离请求、已有 Run 的目录已确认且与本次目录相同才提示。有效隔离请求仍由请求值与 Adapter/Project 支持能力共同决定。未知或待确认目录不当作已确认共用，也不因此承诺安全；请求隔离不等于创建成功。
- 非 git Project 不能隔离，不自动 `git init`。
- 不随 Issue 关闭或 Run 结束自动删树。Claude 交互退出时仍可能按它自己的规则删干净的未命名会话树。
- ChatGPT 桌面应用里的 Codex Worktree 模式不在嵌入式 CLI 合同内。
- Continue 在启动前同时核实已记录路径仍是原 Project 仓库登记的 worktree，且其 git common-dir 与 Project 相同。
