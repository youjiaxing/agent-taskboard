# Agent Taskboard

个人本地优先的桌面效率工具：把多个工作项目里由 Matt Pocock skills（wayfinder / to-spec / to-tickets）产出的 Issue 看清、理顺依赖，并交给本机 Agent CLI 执行。

## Language

**Agent Taskboard**:
本产品的名称：个人使用的本地效率工具。每台电脑上都可以有一份 Host，跑该机上的 Project、Tracker 访问和 Agent 执行；桌面应用与浏览器作为 Client，可连本机 Host，也可配对连其它电脑上的 Host。
_Avoid_: Wayboard, Skills Console, 团队协作平台

**Project**:
某个 Host 上的一个工作项目绑定：对应该 Host 所在电脑上的本地目录，并关联一个 Issue Tracker。
_Avoid_: Repository（除非特指 git 仓库本身）, Workspace（易与编辑器工作区混淆）

**Issue Tracker**:
某 Project 的 Issue 存取后端。v1 优先打通 GitHub；架构上按可替换适配器设计，预期后续包含 GitLab 与本地 markdown。
_Avoid_: Provider（过泛）, Forge（不含 local markdown）

**Issue**:
某 Issue Tracker 上的一条工作项（含 wayfinder map、decision ticket、规格与实现票等），是 Taskboard 展示与分派的基本单位。
_Avoid_: Ticket（对外沟通可口语化，领域词统一用 Issue）, Task（易与 OS/通用待办混淆）

**Dependency**:
Issue 之间的阻塞关系：被阻塞方在阻塞方完成前不可进入 Frontier。不是父子。
_Avoid_: Link, relation（过宽）, 父 Issue, 相关

**父 Issue**:
一张 Issue 至多一个父，表示拆分和归属，不表示阻塞。没有父的 Issue 仍是一等 Issue。
_Avoid_: Dependency, Sub-issue（那是 GitHub 落点）, Epic（那是 GitLab 落点）

**认领**:
Tracker 上占用一张未关闭 Issue 的标记。有认领则该 Issue 离开 Frontier。不是「有活跃 Run」，也不是「正在看详情」。
_Avoid_: Assignee（那是 GitHub/GitLab 落点）, claimed（那是本地 Markdown 的 Status 值）

**Frontier**:
某个 Project 上当前可被认领/执行的 Issue 集合：未关闭、无未完成阻塞、且尚未被认领。不含 triage，也不看有没有活跃 Run。点进某个父 Issue 只是过滤，不是第二种 Frontier。
_Avoid_: Backlog（含未解锁项）, Queue（暗示严格顺序）, 把 ready-for-agent 写进定义

**Triage Role**:
Issue 上的五角色之一：needs-triage、needs-info、ready-for-agent、ready-for-human、wontfix。只用于筛选和分组，不是 open/closed，也不是独立对象。
_Avoid_: Status（open/closed 才是 Tracker 状态）, 看板列, Workflow

**Label Mapping**:
每个 Project 上一套「规范名 → 该 Tracker 真实标签或字段」的对应。默认用 Matt 官方字符串；一个都没对上，则该 Project 不做 skills 识别，只当普通 Issue 板。
_Avoid_: Taxonomy, Issue Type（不是 GitHub Issue Type）

**Agent**:
本机安装的编码 Agent 命令行工具，必须能在 Embedded Terminal 里以官方交互 TUI 运行。v1 内置优先级：Grok Build → Codex → Claude Code；名单不封闭，再接入一家是新的 Agent Adapter。只有本机 Web UI、没有官方交互 TUI 的不算。
_Avoid_: Bot, Assistant, LLM（模型只是 Agent 的配置维度之一）, 把 dsh web 这类浏览器入口先算进 Agent

**Agent Adapter**:
一份稳定合同的一种实现：在启动环境里探测本机是否装了可执行文件、声明可配置项（如 model、effort）、组装交互 TUI 的启动参数、说明能力（含能否原生创建隔离执行目录，以及本家已知安装位置）。内核只认合同，不认具体名单；加一家是新模块，不是改启动表单或 Run 生命周期。各家云账号、API key、登录态不是看板的职责，Adapter 不去管、不拿来禁启动。
_Avoid_: Plugin（可口语化，不要做成插件市场）, Integration（过宽）, 通用 CLI（无合同就开跑）, 把 Agent 登录态当看板功能

**Tracker Adapter**:
将某一种 Issue Tracker（GitHub / GitLab / local markdown 等）的读写与依赖模型适配进 Taskboard 的边界。
_Avoid_: Plugin, Connector

**启动配置**:
某次启动 Run 时，用户在表单上确认过的、该 Agent Adapter 声明的那些值（如 model、effort、权限）。点启动时表单上是什么，这次 Run 就带着什么。按 Project×Agent 记住的那份默认不含隔离执行目录。
_Avoid_: RunConfig（假统一成三家同一套字段）, 把「用默认」当成用户看得见的值, 把是否隔离当成和 model 一样可记忆的偏好

**启动环境**:
Host 为探测和某次 Run 准备的那份进程环境：在目标目录用用户默认壳拍到的整份快照，叠上 Host 的 PATH 前缀和已知安装位置，并覆盖必须钉死的键。不是 Host 从图标拉起时自带的那份。
_Avoid_: 继承终端, Host 环境（那是图标进程自己的）, login env（那是取样方式）, 把用户终端 App 的环境当启动环境

**Run**:
一次在某个 Project 中启动 Agent CLI 的可观察执行会话。Run 通常绑定一个 Issue，也允许不绑定 Issue；一个 Issue 可按时间关联多次 Run，但同一时刻最多有一个活跃 Run。Run 的生命周期与 Issue 状态彼此独立：Issue 关闭不会终止活跃 Run，Run 结束也不会表示或触发 Issue 完成。默认在 Project 主目录执行；可选使用隔离执行目录。
_Avoid_: Job（偏 CI）, Session（易与编辑器/聊天会话混淆）

**隔离执行目录**:
某次 Run 可选的第二份工作目录：由该 Agent 的官方 CLI 用 git worktree 从 Project 主目录建出，共享同一仓库历史，文件改动互不覆盖。默认不用；只有 Adapter 声明能原生建树时，用户才能在这次 Run 上打开。看板不替 CLI 建树。
_Avoid_: Worktree（实现机制，说明里要写明，不当领域词）, Workspace（那是别的产品更重的对象）, 沙箱（那是写盘/网络权限，不是第二份目录）

**执行已停**:
Issue 仍被认领，但没有活跃 Run，且最近一次绑定 Run 是异常结束、被停止或 Host 崩溃后捡回。仍算出局于 Frontier（认领还在），必须能继续（优先恢复原生会话）或释放认领。
_Avoid_: Failed（那是 Run 结束原因）, Blocked（那是依赖未完成）, 等待操作（那时 Run 仍活跃）

**自动推进**:
可选能力（默认关）。开关记在 **Project** 上；这台 Host 另有总开关（默认关），关掉则这台 Host 上所有 Project 都不推进。当前 Issue 已关且状态正常、待确认未被否决之后，自动认领下一张 `ready-for-agent` 并开 Run。Host 冷启动后默认不推进；「冷启动后恢复自动推进」记在 Project 上（默认关，且仅当 Host 总开关和该 Project 开关都开着才生效），到点后等 N 秒（默认 60）。不是默认无人值守编排器。
_Avoid_: 编排器, 心跳认领, 依赖完成即开跑, 启动 Host 就开工

**等待操作**:
Run 仍活跃，但 Agent 因权限确认、提问或选项选择而等待用户操作。不是「执行已停」，也不是被 Dependency 阻塞。
_Avoid_: 在等人, 等待输入（易误解为只等文字）, 需人工介入（像异常升级）, Blocked

**待确认**:
自动推进里的一段短等待：当前票已关，并且状态正常（看见 SessionEnd、没有 StopFailure、进程正常退出）之后，倒计时 60 秒内人可以否决。无人否决才领下一张 `ready-for-agent`。grilling / prototype / needs-info / ready-for-human / needs-triage 不进自动池。不是验收，也不强迫人先打开查看改动。
_Avoid_: 完成, Review（那是别的产品的列名）, 把查看改动当验收

**自检**:
干活的 Run 停了之后，仅当这张 Issue 还没关，或状态不正常（缺 SessionEnd、有 StopFailure、或进程非正常退出）时，才要求同一个 Agent 检查现状：该继续就继续，确认做完再关票。票已关且状态正常则不再开 Run。进程已退则新开 Run 并尽量恢复原会话；进程还在且刚发生 StopFailure 时，往同一条 Run 的官方 TUI 注入一句。自检后仍没关或仍异常则停下，不开下一张。票已关但 hook 异常时只打开查看改动、不自动 reopen。
_Avoid_: 关票前复查（那是上一版误写成「每次必跑第二条 Run」的说法）, 验收通过（自检仍是模型判断）, 验货

**查看改动**:
针对某次 Run 工作目录的只读对比：现场对本地 git 现算，默认相对启动时记下的 commit，可切到未提交。自动包含目录里有限几层的独立子仓库。不是 Tracker 状态，也不是看板列；和票是否关闭、认领、自动推进无关，做到一半也能看。桌面应用和电脑浏览器要完整能力；手机不要。
_Avoid_: 审阅面（可口语）, 行评, Review 列, 待审, 验货

**改动备注**:
人在查看改动里针对某一行写下的一句话，带着哪个仓库、哪个文件、哪一行。只留在看板，不写回 Tracker，也不灌进还在跑的那次。下次开跑并进开场白后，从待送出里清掉。
_Avoid_: 行评, review comment, 批注

**最近完成**:
主界面最右一列：只展示最近关闭的 N 张 Issue。N 默认 5，人可以改。用来找回刚关、还想再看的票。不是全部已关闭，也不是看板 Done 列；不能拖进这一列来关票。
_Avoid_: Done 列, 已关闭列, 完成列, 待审

**Embedded Terminal**:
Host 上我们自己开的真实终端（PTY），用来跑 Agent 官方 CLI。不是用户系统里的终端 App，也不是自研聊天 UI。桌面和浏览器 Client 只是把这块终端画出来、把按键送回去；PTY 不在 Client 上。
_Avoid_: Console panel（可指日志面板）, Chat UI, 把 Terminal / iTerm2 / Windows Terminal 当产品终端

**Host**:
跑 Issue Tracker 访问、Agent CLI 和 Run 的常驻进程。PTY 只存在于 Host 上。它所在的电脑就说「跑 Host 的那台电脑」。
_Avoid_: Server（易理解成我们提供的云）, 后台（过糊）, 把 Host 说成那台机器

**Client**:
连上 Host、用来看态势、开停 Run、向 Run 注入输入、看终端的界面。v1 有两类一等 Client：桌面应用，以及浏览器（含手机）。一个 Client 窗口可以同时连本机 Host 和已配对的远程 Host。手机只当 Client，不在手机上起 Host。关掉 Client 不等于停掉 Host 上的 Run。
_Avoid_: 只把 Client 说成浏览器, 前端（只说技术层）

**配对**:
Client 获准连上某个 Host 的一次性手续：交换可到达地址和一次性配对码，之后靠长期令牌。不是产品账号。Host 上可以撤销某个 Client。连通走用户自己的 Tailscale / VPN / 局域网。
_Avoid_: 登录, 账号, 我们的中继

**界面语言**:
某个 Client 上产品壳（按钮、分区名、空状态、设置、配对说明、托盘）用的语言。本机桌面窗口和 Host 托盘共用一份；v1 只有简体中文与英语两项具体语言。不改变 Tracker 原文、官方 TUI 或原始报错。
_Avoid_: 用户语言偏好（像登录账号）, 跟随系统（不是可选项）, locale

**主题**:
某个 Client 上产品壳的一套外观。默认主壳骨架是 Codex 气质的原生左侧栏 + 白主区，不是纸面书桌。v1 清单仍是：暖纸（仅白天）、素纸、素纸夜间。没有暖纸夜间。记在每个 Client，值是一份具体主题；设置里没有「跟随系统」。第一次按系统浅/深匹配：浅 → 暖纸，深 → 素纸夜间。不改变 Tracker 原文或官方 TUI。
_Avoid_: 皮肤（可口语）, 跟随系统, 暗色模式（那只是素纸夜间，不是第二种产品）
