# 按目标目录拍用户默认壳环境，再绝对路径 exec Agent

从图标 / 托盘拉起的 Host 只有一份瘦环境，和日常终端不是同一份。探测和 Run 都不用 Host 自己的环境，也不把 Agent 包进 `zsh -lc '…'` 当启动器。做法是：在**目标目录**用用户默认壳拍一份整环境，再按绝对路径 exec Agent，把这份环境交给 PTY 子进程。壳只负责取样，不留在 PTY 里当父进程。

目标目录 = 这次 Run 的 cwd：默认是 Project 主目录；开了隔离执行目录则是那个 worktree；还没选 Project 时（Host 级「已安装」列表）则是用户家目录。家目录那份只用来看本机装了谁，真正开 Run 必须按该次 cwd 重拍，两份不能混用。

macOS 用 `$SHELL`，登录 + 交互（`.zprofile` + `.zshrc` 一类），设超时；失败则只登录，再退回已知目录。Windows 没有登录 / 交互之分：注册表用户环境 + `$PROFILE`；拍照壳优先 `pwsh`，否则 `powershell.exe`。Git Bash / MSYS 不是拍照壳，也不是一等启动环境——只在找不到原生 exe、只剩 POSIX 脚本时，用 `bash.exe` + 参数数组去跑那个脚本。

短缓存（键含目标目录）：打开已安装列表、启动表单、点启动时过期就重拍；设置里可手动刷新；失败沿用该目录上一份**内存**快照。快照不落盘。整份快照当基底；Host 只有一份全局 PATH 前缀叠在最前；各 Adapter 声明的已知安装位置（如 `~/.grok/bin`、`~/.local/bin`）也 prepend。我们钉死 `TERM` / `COLORTERM`。找不到可执行文件时列出命令名、已搜 PATH、已知位置，并指向系统用户环境 / 默认壳的 login 文件（Windows 先指用户环境变量）；不说「先开一个终端 App」，也不暗示重开窗口会刷新。

这钉住了 [决策：启动 Run 时如何获得用户日常终端里的 PATH 与环境](https://github.com/youjiaxing/agent-taskboard/issues/23)，并接上 [Agent Adapter 配置合同与默认值](./0003-agent-adapter-config-and-defaults.md) 的探测合同、[并行 Run 默认共用主目录，隔离只走 Agent 原生 git worktree](./0004-native-worktree-isolation.md) 的 cwd、以及 [Tauri 2 作为桌面壳](./0007-tauri2-desktop-shell.md) 里「不嵌入用户终端 App」。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 每次用 login shell 把 Agent 当字符串命令包起来启动 | 附加参数会被壳再拆；TUI 多一层解释器；`-i` 可能卡在 `.zshrc`。`exec "$@"` 能避开引号问题，但 `.zshrc` 欢迎语 / tmux 会污染 Agent 屏，Windows 当启动器更脏 |
| 在家目录拍一份全局快照，所有 Project 共用 | 丢掉进仓库才有的 `.nvmrc` / direnv；还会把 A 仓的 node 漏到 B 仓 |
| 只用 Host 从图标带来的环境，外加写死几个常见目录 | 对不上 nvm/fnm/公司私有 PATH；本机 Codex 已在 `~/.nvm/.../bin` |
| 以看板手填 PATH 为主 | 让人维护第二份 PATH；手填只当 Host 级补洞 |
| Git Bash 作为 Windows 一等启动 / 拍照壳 | 三家官方 Windows 安装是原生 exe；Claude 文档里 Git Bash 是 Bash **工具**，不是启动 `claude` 的前提 |
| 只拍登录壳 / 只合并 PATH | nvm 等多在交互配置里，且依赖 `NVM_DIR` 等；只抄 PATH 会跑残 |
| 只在 Host 启动时拍一次，或每次强制重拍 | 前者和「关窗口 ≠ 停 Host」打架；后者打开列表就要连等慢 `.zshrc` |
| 每 Project 一份 PATH 前缀，或快照落盘 | 再造一套 per-repo PATH；落盘等于把壳里的 token 写成文件 |

## Consequences

- `/to-spec` 按「启动环境 ≠ Host 环境」写探测和 spawn：同一套规则，目标目录随有没有 Project / 是否隔离而变。
- 完成信号仍看 Agent 进程退出：PTY 里没有长期活着的用户壳。
- 实现时取样必须能从带噪声的壳输出里抽出环境，并给超时；具体 TTL / 超时秒数不在本 ADR 钉死。
- 设置与数据存放位置仍未钉；PATH 前缀作为 Host 设置项，落点跟那条走。
