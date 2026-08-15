# Tauri 2 作为桌面壳，Host 住在同一核心进程

v1 桌面 Client 用 Tauri 2。Host 住在这份桌面应用的 Rust 核心进程里：关窗口只藏窗口，核心进程和托盘还在；设置里「本机不起 Host，只当 Client」是这次启动不跑 PTY / Tracker / 监听，不是再开一个别的程序。macOS 与 Windows 同一套栈、同一版本——可以先在 Mac 上做、再到 Windows 上验，不能做成两套产品。本机窗口和浏览器走同一套 Host 协议（本机就是本机地址）；Tauri 只做窗口、托盘、登录自启、系统通知，不把业务 API 做成 Tauri 专用命令。

Embedded Terminal 是 Host 上我们自己开的 PTY，不打开、不嵌入用户的终端 App。Client 画终端默认用 xterm.js（见 [调研：Tauri 2 在 macOS/Windows 嵌入真实 PTY 的可行性](https://github.com/youjiaxing/agent-taskboard/issues/3)）。用户装了几种终端或几种 shell，不构成换壳理由。启动 Run 时如何继承日常终端的 PATH / 环境，见 [决策：启动 Run 时如何获得用户日常终端里的 PATH 与环境](https://github.com/youjiaxing/agent-taskboard/issues/23)。

只有下面这类能力做不到，才改 Electron 或其它壳：关最后一扇窗口后保不住 Host / Run；无法把 Host 上的 PTY 流转发到远程桌面和浏览器并达到可交互 TUI；登录自启、单实例、托盘里「退出 Host」在任一必交付系统上做不稳。手感、样例多少、包体积、xterm 显示瑕疵都不算换壳理由。

这落地了 [Host 常驻、配对与远程 Client](./0006-host-resident-pairing-remote-client.md) 对壳的三条验收：关窗口 ≠ 停 Host、PTY 流转发到远程桌面/浏览器、macOS 与 Windows 同形态。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 现在就改 Electron | PTY 调研已排除「只有 Electron 才能嵌终端」；浏览器 Client 已迫使终端 UI 做成普通网页；换壳消不掉 xterm 层的坑，还多拖一整颗 Chromium |
| 不要独立桌面壳，Host 自带托盘、人只用浏览器 | 本机窗口仍是一等 Client；托盘还要「打开窗口 / 退出 Host」 |
| Host 做成独立守护进程，桌面应用永远只是 Client | v1 要管两份进程、谁持有托盘、本机 Client 怎么连本机 Host，过重 |
| Host 做成 sidecar，随桌面启动再决定是否 detach | sidecar 默认跟父进程同生共死；要 detach 就变成独立守护，只是更绕 |
| 两端形态一样但技术栈可以不同 | 两套壳意味着两份崩溃、打包、托盘和 PTY 胶水 |
| v1 只保证 Mac | 会把 Host 长成 Mac 专用，和已钉的双端约束对着干 |
| 本机走 Tauri IPC，远程才走 HTTP/WS | 业务会在两条通道上分叉，远程一定落后 |
| 先只做 Tauri IPC，浏览器以后再接翻译 | 等于宣布浏览器不是 v1 一等公民 |
| 把用户的 Terminal / iTerm2 / Windows Terminal 当产品终端 | 和 Embedded Terminal 冲突；PTY 必须只在 Host 上，Client 只是遥控器 |

## Consequences

- `/to-spec` 按「Rust 核心进程 = Host，WebView 窗口 = Client」写桌面形态；远程桌面和浏览器是同一套 Host 协议的另外两个 Client。
- 画终端默认 xterm.js，不写进词表。换画布不是换壳。
- Windows 因 ConPTY 实际底线是 Windows 10 1809+；产品声明最低系统，过旧给明确错误。
- 打包分发与自动更新仍未钉，见 map 的 Not yet specified。
