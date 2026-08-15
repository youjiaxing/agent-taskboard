# Agent Adapter 配置合同与默认值

三家官方 CLI 的启动参数、枚举和鉴权方式对不齐，而且以后还要接新的 Agent。内核只认一份稳定合同：探测本机可执行文件、按 Adapter 声明字段、组装交互 TUI 的 argv、说明能力。字段集合、枚举和 flag 拼法分家维护，不造假统一的 `permissionMode` 或统一 effort 五档。v1 内置 Grok Build → Codex → Claude Code，名单不封闭；加一家是新模块，不改启动表单、默认值记忆或 Run 生命周期。各家云账号、API key、登录态不是看板职责。

## 默认值与启动表单

- Taskboard 按 `(Project, Agent)` 记住上次成功启动的启动配置；该 Project 第一次用该 Agent 时，回退到同一 Agent 在其它 Project 的记忆，再没有则只读各家 CLI 自己的配置当种子。
- 叠层只负责预填。打开表单时第一层必须已经是具体值；点启动只认表单上的值。
- 不写回用户家里的 CLI 配置文件。
- 允许折叠的附加参数，追加在 Adapter 组装结果之后；禁止整条命令可编辑。附加参数可记入本地 Run，禁止写入 Tracker（与 [决策：Run 生命周期与 Issue 的绑定方式](https://github.com/youjiaxing/agent-taskboard/issues/9) 的写回白名单一致）。
- 命令预览可以有，但不是一等展示，设置里能关掉。
- model / effort 尽量问本机 CLI；未知枚举警告但不拦启动。
- 隔离执行目录见 [并行 Run 默认共用主目录，隔离只走 Agent 原生 git worktree](./0004-native-worktree-isolation.md)；不在本合同里再拍板。

## 明确不做

- 把 Codex 的 approval × sandbox 压成与 Grok / Claude 相同的一个权限模式
- 管理或探测 Agent 登录态并据此禁启动
- 用户丢一个二进制名就能开跑的「通用 CLI」
- 运行时插件市场
- v1 实现 DeepSeek Harness 或 Antigravity CLI（等它们有官方交互 TUI 再按同一合同加；仅有 `dsh web` 不算 Agent）

## Considered options

| 选项 | 未采纳原因 |
|------|------------|
| 统一 RunConfig / 统一 permissionMode | 三家语义不同，调研已见文档外 effort；假统一会在第 4 家爆掉 |
| 只按 Agent 记一份全局默认 | 玩具仓的宽松权限会带到工作仓 |
| 每次从 CLI 默认开始、看板不记 | 和「按 Project/Agent 记默认」冲突 |
| 写回 ~/.grok 等用户配置 | 和 TUI `/model`、用户手改配置打架 |
| 整条启动命令可编辑 | 容易离开交互 TUI，探测和写回失真 |
| 未登录则禁启动 | 登录不是看板职责；走 API key 时未登录也能用；Grok 也没有可靠登录查询 |
| v1 只做 Grok | 把实现顺序误当成产品范围 |

## Consequences

- `/to-spec` 按本合同写 Agent Adapter，不必为三家各编一套内核模型。
- 启动表单按各 Adapter 的字段声明渲染；第一层字段清单见对应决策票，不在内核写死。
- 默认选中：已安装列表里按 Grok Build → Codex → Claude Code 挑第一个。找不到可执行文件则不能启动；这不是在管登录。
