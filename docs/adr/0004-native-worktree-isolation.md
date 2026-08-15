# 并行 Run 默认共用主目录，隔离只走 Agent 原生 git worktree

同一 Project 里多个 Run 可以并行，默认都在 Project 绑定的本地工作区。隔离是单次 Run 的显式选择，且只在该 Agent Adapter 能把官方 CLI 的建树开关传出去时可用。v1：Grok Build / Claude Code 传 `--worktree`（不代起名字）；Codex CLI 没有对等开关，隔离不可用，等官方。看板不执行 `git worktree add`，也不做通用 worktree 工作台。端口、本地数据库、锁文件只警告，不分配、不禁止并行。看板只记录这次 Run 用上的那棵树，并提供显式删除；何时删树沿用各家 CLI 默认。「继续」不再传 `--worktree`，回到已记录目录，树没了则回主目录并说明。

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
- 非 git Project 不能隔离，不自动 `git init`。
- 不随 Issue 关闭或 Run 结束自动删树。Claude 交互退出时仍可能按它自己的规则删干净的未命名会话树。
- ChatGPT 桌面应用里的 Codex Worktree 模式不在嵌入式 CLI 合同内。
