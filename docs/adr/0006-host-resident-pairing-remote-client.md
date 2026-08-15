# Host 常驻、配对与远程 Client

Host 是常驻进程，不是那台电脑。关窗口只藏窗口，进程留在托盘；只有「退出 Host」才停进程，那时若有活跃 Run 必须选返回或停掉全部 Run。每台电脑都可以有一份独立 Host；桌面应用默认拉起本机 Host，也可只当 Client。一个 Client 窗口可同时连本机 Host 和已配对的远程 Host。无产品账号、无自建中继：配对是地址 + 一次性码，之后长期令牌，连通走用户自己的网络。Agent CLI 和 PTY 只在 Host 上；Client 是遥控器。

这改写了 [决策：Run 生命周期与 Issue 的绑定方式](https://github.com/youjiaxing/agent-taskboard/issues/9) 的「应用退出即停全部 Run」：那里的退出只指退出 Host。冷启动后默认不自动推进；「冷启动后恢复自动推进」受自动推进开关约束，见 [完成信号与可选自动推进](./0005-completion-signal-and-auto-advance.md)。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 全世界只有一份 Host，其它电脑只能当 Client | 挡掉「笔记本自己干活 + 同时遥控 mini」 |
| 一条 Run 跨两台机器 | PTY、目录、进程只在一处 |
| 关窗口就停 Host | 远程遥控没有意义 |
| 最后一个 Client 断开就停 Host | 合盖 / 切后台会误杀 Run |
| 局域网自动发现、无需配对 | 会连错；跨网也用不上 |
| 手机一等验收官方 TUI + 锁屏推送 | TUI 在手机上交不出；推送另开一摊且接近中继 |
| 每个 Client 各自内嵌 Agent CLI | 集成点必须只有 Host 上那一份 |

## Consequences

- 壳选型必须满足：关窗口 ≠ 停 Host、PTY 流转发到远程桌面/浏览器、macOS 与 Windows 同形态。见 [决策：是否坚持 Tauri 2 作为桌面壳](https://github.com/youjiaxing/agent-taskboard/issues/12)。
- 主界面要能面对「一个窗口连多份 Host」；怎么摆交给信息架构原型/定稿。
- 两份 Host 登记同一仓库时，v1 不加跨 Host 锁。
