# 开 Run 走配置表，不绑票从 Project 行加号进

绑票开工不再「一步认领并启动」。点票上的「执行」（英文 Run）先填该 Agent 自己的启动配置，点启动才创建 Run 并认领。不绑票从当前 Project 行右侧加号「新建」（英文 New）进同一张表，不认领任何 Issue。有记忆则跳过选 Agent，直接进这个 Project 上次成功用过的那家；换一家可回名单。中文按钮不把 Run 嵌进去。进行中卡片只露一行最近动作和有限的只读最近输出（在改哪个文件、在跑哪条命令、在想、等你确认），完整 Embedded Terminal 不再停靠在看板底部；点击卡片仅在有活跃 Run 时进入专注工作区。没有活跃 Run 时仍停留在 Issue/看板，历史 Run 必须从默认展开的运行记录中明确选择。钉于 [原型：开 Run 配置与游离 Run 入口](https://github.com/youjiaxing/agent-taskboard/issues/38)。

这改写了 [ADR 0015](0015-codex-shell-ia.md) 里底栏「开 Run（一步认领并启动）」；底栏仍跟当前 Issue，有活跃 Run 仍切那条 PTY。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 底栏变配置台 | 启动后完整官方 TUI 只属于专注工作区；表单用居中填表，和选 Agent 名单同一套 |
| 表单铺在票详情 | 窄栏塞不下各家字段 |
| 每次都先选 Agent | 同一 Project 多半还用上一家；有记忆就跳过 |
| 按钮写「开 Run / 打开 Agent / 游离」 | 中英夹杂，或看起来像第二种工作单元 |
| 卡片上刷思考全文或只写「进行中」 | 看板会跳，或看不出此刻在干什么；因此只保留 Adapter 可见的最近一步和有界只读输出 |

## Consequences

- `/to-spec` 按「执行 / 新建、先选 Agent 可跳过、表单摊开具体值、不绑票不加号以外的工作单元」写，不要写回一步启动或「开 Run」这种夹杂文案。
- 现场一行只写 Adapter 能看见的最近一步；官方 TUI 没暴露的不编。
- Board 不承载完整 PTY；活跃 Run 的有限最近输出由 Host 快照投影提供，Focus Workspace 才挂载可输入的 Embedded Terminal。
- Issue 没有活跃 Run 时，点击 Issue 不隐式打开历史 Run；同一 Issue 的历史 Run 在运行记录中默认展开，明确点击后进入只读输出。
