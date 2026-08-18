# 设置与数据分 Host 数据与 Client 设置，落在 Tauri Local，秘密用 JSON 文件

桌面壳是 Tauri 2，Host 与本机窗口同一进程。持久数据走 Tauri `appLocalDataDir`（macOS：`~/Library/Application Support/<identifier>`；Windows：`%LOCALAPPDATA%\<identifier>`，不进漫游盘），其下固定两棵树：`host/` 是 **Host 数据**，`desktop-client/` 是本机窗口与托盘共用的 **Client 设置**。日志走 `appLogDir`（macOS：`~/Library/Logs/<identifier>`；Windows：实现为 Local 树下的 `logs`）。以后 Tracker 缓存若落盘，走 `appCacheDir`。目录按官方 path 解析，应用自己创建；桌面应用第一次启动就建好两棵树和日志目录，「本机不起 Host」只是开关，不改文件夹形状。

具体 identifier 跟 `tauri.conf`，本 ADR 不钉字符串。改 identifier 等于换一套空目录，不自动迁移。

文件用 JSON（Rust + 网页栈零额外库）。秘密（Tracker PAT、桌面 Client 的配对长期令牌）和普通设置分开文件，权限收到仅当前用户可读。不用操作系统钥匙串：个人工具要「打开文件夹能看懂、复制目录能搬家」，钥匙串拆掉这条路径，浏览器也没有。这改写了 [本机凭据与远端鉴权](./0001-local-credentials-remote-auth.md) 的存储介质和第四来源。

一份 Host = 这台电脑、这个系统用户的这一份。v1 不做备份/导出按钮。搬家 ≈ 复制 `appLocalDataDir` 下的 `host/`（以及要用的 `desktop-client/`）；Project 本机路径对不上要手改。日志和缓存不必一起搬。

浏览器 Client：语言/主题/远程配对令牌记在该站点的持久存储。清站点数据或访问地址变了 = 重新配对，或粘贴事先复制的连接信息。本机浏览器打开 Host **自己端出来的**稳定回环页（规范为 `http://127.0.0.1:10529/`，见 [打包分发与自动更新](./0013-packaging-and-auto-update.md)）免配对，和本机窗口一样；必须校验来源是这个源。用 Tailscale / 局域网地址打开，或本机上其它网站去打 Host，一律要长期令牌。

这钉住了 [决策：设置与数据存放位置](https://github.com/youjiaxing/agent-taskboard/issues/27)。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 自己拼 Application Support / `%APPDATA%`，不走 Tauri path | 和壳脱节，换 identifier 要改业务代码 |
| 家目录点文件夹 `~/.agent-taskboard` | macOS 上不像原生应用；日志也不进系统 Logs |
| 跟每个 Project 仓库走 | Host 级登记、配对不属于某一个仓库 |
| Host 数据放 Windows 漫游 `appDataDir` | 里面全是本机绝对路径，漫游同步会拖坏 |
| 只当 Client 时不建 Host 树 | 「起过又关掉」仍必须留 Host 数据，懒建会变成三种状态 |
| v1 做导出/导入向导 | 个人工具先能说清复制哪个文件夹 |
| 云同步 / 多机同一份 Host 数据 | 无账号、无云同步，已在地图范围外 |
| 秘密进 OS 钥匙串 | 搬家不对齐；浏览器没有；用户更熟本地文件 |
| YAML / TOML | 对人手改略友好，但栈上要额外库；文件主要由程序读写 |
| 本机任意浏览器、含 Tailscale 地址都免配对 | 本机其它网页更容易打到「像本机」的入口 |
| 浏览器每次重配对 | 和「之后靠长期令牌」打架 |

## Consequences

- `/to-spec` 按「`appLocalDataDir` 下 `host/` + `desktop-client/`，日志 `appLogDir`，缓存 `appCacheDir`」写目录职责。秘密与普通设置分文件，JSON，不用钥匙串。
- Tracker 凭据顺序见改写后的 [0001](./0001-local-credentials-remote-auth.md)：专用覆盖（应用 env，其次文件里显式 PAT）→ `gh`/`glab` → 通用 env。
- [界面语言](./0010-interface-language.md) 与 [主题](./0011-shell-theme.md) 落在 Client 设置；PATH 前缀落在 Host 数据，见 [0008](./0008-run-launch-environment.md)。
- 本机回环端口为 `10529`，见 [0013](./0013-packaging-and-auto-update.md)。Tracker 刷新票若要持久缓存，文件放 `appCacheDir`。
