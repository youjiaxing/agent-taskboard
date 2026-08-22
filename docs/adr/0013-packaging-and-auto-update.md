# GitHub 发未签名安装包，更新须确认，本机页固定 10529

v1 桌面应用的主交付是本仓库 [GitHub Releases](https://github.com/youjiaxing/agent-taskboard/releases)：macOS 两份 dmg（Apple Silicon 与 Intel，均为 ad-hoc 签名），Windows 一份 NSIS `setup.exe`。不上商店，v1 不做苹果公证或 Windows 代码签名；第一次打开自己过 Gatekeeper / SmartScreen。自己从源码构建永远可以，但不是主路径。v1 不打 Linux 桌面包、不验收 Linux Host——Linux 上用浏览器当 Client。

同一份程序仍按 [Tauri 2 桌面壳](./0007-tauri2-desktop-shell.md)：「本机起 Host」或「只当 Client」是启动开关，不是两套安装包。双端同一版本号。

**自动更新**走 GitHub Releases 的 `latest.json` 与 Tauri updater 验签（和系统签名不是一回事；私钥只放在发版侧）。没有产品账号。检查到新版本先告诉人，人确认后再下载安装；不后台偷换。本机窗口从无到有出现时检查一次（只挂托盘不查），关于页/设置也可手动检查；启动检查失败不弹窗。有活跃 Run 就不让装。装完按更新前的模式再拉起。更新器只换程序：失败或跨大版本，Host 数据与 Client 设置都留在 [设置与数据存放](./0012-settings-and-data-location.md) 已钉的原目录。浏览器 Client 不能给另一台机器上的 Host 换包。

浏览器入口不是安装包。Host 起来后在 `http://127.0.0.1:10529/` 端出本机网页；本机浏览器打开这个源免配对，见 [0012](./0012-settings-and-data-location.md)。端口写死。被占用则本机网页入口起不来并说清原因，桌面窗口照常。「本机不起 Host」时不提供这个入口。

这钉住了 [决策：打包分发与自动更新](https://github.com/youjiaxing/agent-taskboard/issues/28)。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 每台电脑自己构建，不发安装包 | 「先在 Mac 做、再在 Windows 验收」会变成每台机器编一次 |
| v1 就做苹果公证 + Windows 代码签名 | 把规格卡在年费和证书上；以后要加仍走同一 Releases |
| 上架 Mac App Store / Microsoft Store | 沙箱和审核，和常驻 Host、无账号对着干 |
| 一个 Universal dmg 塞两种 Mac 芯片 | 包更大；官方更新清单按 `darwin-aarch64` / `darwin-x86_64` 分条更干净 |
| Windows 再发一份 MSI | 同一版本两个包，更新也对不齐 |
| v1 把 Linux 桌面包当一等交付或「顺便」出 AppImage | 目的地只钉 macOS 与 Windows；发出去就会被当正式版 |
| 不做应用内更新，人自己去 Releases 翻 | 你要的就是不用自己找包 |
| 只提示并打开 Releases 页 | 省掉更新密钥，但体验退回浏览器下文件 |
| 后台静默下载安装 | Windows 会强退进程；有活跃 Run 等于掐掉 Agent |
| 有活跃 Run 仍允许强行装 | 给「退出 Host」开了不经停 Run 的后门 |
| 只在进程冷启动时检查更新 | Host 常驻时会连续几天查不到 |
| 运行期间定时检查 | 个人工具偏吵 |
| 装完再问一次要不要打开，或不拉起 | 人刚点了更新就是想继续用；Windows 上不拉起桌面是空的 |
| 本机页用随机或可改端口 | 和「只认这一种本机 URL」拧着，书签和免配对来源都会坏 |
| 用 Tauri localhost 插件给浏览器当入口 | 官方给 WebView 用，还带安全警告，不是给外部浏览器的 |

## Consequences

- `/to-spec` 按「Releases 上两份 Mac dmg + 一份 Windows NSIS、未系统签名、确认后更新、回环页 `http://127.0.0.1:10529/`」写交付与更新。不要写商店、Linux 桌面验收或静默换包。
- 发版必须保管 Tauri 更新私钥（CI secret）。丢了，已装版本无法再走应用内更新，只能重装。
- `latest.json` 只列 macOS / Windows。不要把未交付的 Linux 写进去——Tauri 会校验整份文件。
- 回环端口见本 ADR，不再悬空；[0012](./0012-settings-and-data-location.md) 的地址写成 `http://127.0.0.1:10529/`。
