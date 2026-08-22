# 查看改动是独立能力，人闸不卡在关票

v1 在 Run 旁提供只读的 **查看改动**：现场对这次 Run 工作目录算 git 对比，不另存快照，也不写成 Tracker 状态或看板列。它和票开着/关了、认领、自动推进脱钩；做到一半也能看这时的文件变更。自动推进默认关着时它更常用，但开着也不藏、不当闸。待确认仍只否决「要不要领下一张」。

人闸不卡在关票。Agent 可以用 Tracker 关 Issue；看板不代关，也不把拖列或进程退出当成验收。人不满意就再开 Run，或把票重新打开。这收回 [调研：Agent 看板类产品的工作单元与编排模型](https://github.com/youjiaxing/agent-taskboard/issues/17) 里「只有人能关 Issue」那句，与 [完成信号与可选自动推进](./0005-completion-signal-and-auto-advance.md) 对齐。该 ADR 里「票已关但 hook 异常只验货」= 打开同一份查看改动，不自动 reopen，也不领下一张。

默认看「这一轮」（相对这次 Run 启动时记下的 commit，含已提交与未提交），可切「未提交」（相对现在的 `HEAD`）。自动查找 Run 目录自身、以及底下有限几层里独立的 git 仓库（外层 `.gitignore` 常会忽略这些子目录）；每个仓库一块对比，各自记启动 commit。跳过 `node_modules` 一类目录。启动后才出现的仓库只有「未提交」。找不到任何 git 仓库就不展示，只说明原因。隔离树没了、或某个子仓库当时不在树里：提示看不了，不跑回主目录乱比。隔离执行目录仍只是外层仓库的第二份目录，看板不把子仓库拷进新树（与 [并行 Run 默认共用主目录，隔离只走 Agent 原生 git worktree](./0004-native-worktree-isolation.md) 一致）。

人可以在某一行写下 **改动备注**。话只留在看板，带着哪个仓库、哪个文件、哪一行。下次开跑时，开场白 = [Run 生命周期](https://github.com/youjiaxing/agent-taskboard/issues/9) 已钉的 Issue 名字和地址 + 这些话，整段可改可删，再进官方 TUI。成功开跑后，待送出的备注清掉。不写回 Tracker，也不灌进还在跑的那次。游离 Run 没有 Issue 定位，只带这些话。

v1 不做「开 PR」产品入口，不搜 GitHub/GitLab 上的 PR，写回白名单不加 PR 地址，PR 不是做完的证据。桌面应用和电脑浏览器要完整查看改动；手机不要。面板在主界面里怎么摆，交给信息架构原型/定稿，本 ADR 不画布局。

这钉住了 [决策：审阅面做到哪一层](https://github.com/youjiaxing/agent-taskboard/issues/22)。

## Considered options

| 选项 | 未采纳原因 |
| --- | --- |
| 只有人能关 Issue | 自动推进已把「票已关」当必要条件；Tracker 分不清人和 Agent；等于重开完成信号 |
| 按票类型拆关票权 | 自动池已经不捞 grilling / prototype / 需人处理的票，再拆一套关票权收益很小 |
| 不做查看改动，只靠 TUI / GitHub / `git` | 隔离目录和被子仓库藏住的改动，看板上会整片失踪 |
| 做成 GitHub 式完整评审（讨论串、approve） | 地图已把「完整替代 GitHub/GitLab Web UI」划出范围 |
| 只要未提交，或只相对默认分支 | 前者在 Agent 一 commit 后变空；后者会把这次 Run 之前的提交混进来 |
| 只看最外层仓库 | 外层 ignore 掉的子仓库完全看不见 |
| 启动时手勾子仓库名单 | 1～3 个固定结构不值得每次勾 |
| 待确认必须先打开对比，或单独「待审」列/标签 | 「看过」说不清；列一出现人就会拖列当完成；也不自动改流转标签 |
| 改动备注写回 Tracker，或替换掉 Issue 定位 | 滑成第二套评审；Agent 可能找不到要办哪张 Issue |
| 备注一直留着当历史，或另做「送给 Agent」按钮 | 开场白堆旧账；和继续/再开拆成两条路容易漏 |
| 产品化开 PR，或 Adapter 可选「一键开 PR」 | 绑死 GitHub；又在官方 TUI 旁加一条控制面 |
| 手机也做完整或简化对比 | 扩大手机验收面；已钉手机不是完整工作台 |
| 给每次 Run 打对比快照 | 薄审阅变厚；树没了还要留副本 |

## Consequences

- `/to-spec` 按「查看改动 ≠ 状态、人闸不卡关票、启动时记每仓库 SHA、现场现算」写，不必再发明待审列或完成闸。
- Run 元数据须记下：启动时各 git 仓库的路径与 commit；这次用的工作目录（主目录或隔离执行目录）。
- 主界面原型必须画出这块独立面板（干活中也能看），且不得画成「待审」列。见 [原型：主界面信息架构低保真方案](https://github.com/youjiaxing/agent-taskboard/issues/14)。
- [决策：Tracker Adapter 统一能力面](https://github.com/youjiaxing/agent-taskboard/issues/11) 不因此补 PR / diff 能力；对比在 Host 上对本地 git 算。
