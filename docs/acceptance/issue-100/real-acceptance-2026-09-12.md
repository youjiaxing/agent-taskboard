# #100 用户授权的真实产品验收

日期：2026-09-12。用户明确委托 Agent 测试，并授权使用 agent-taskboard 的测试 Issue #124 与独立测试 Project；随后说明 Claude Code API 不可用，无需测试该部分。

## 结论与边界

- **PASS：核心 Codex 连续任务。** 从原生空 Host 登记 GitHub Project、加载真实 Issue，到从产品配置表启动官方 Codex、真实认领 #124、完成指令、手机注入、查看实际文件改动、停止、PTY 中断后继续与放领，均有产品 UI 和实际副作用证据。
- **PASS：修复回归。** 真实测试发现手机 ANSI 原样显示、注入无法提交、隔离目录延迟出现后误读主目录；均已修复。备用屏结束保留、尺寸上限、Host 重启与隔离归属保护另经回归验证。
- **SKIPPED：Claude API。** 按用户明确要求停止测试，不计 PASS。此前一次原生 CLI 已创建 worktree，但选用模型返回不可用/无访问权限；该结果不代表任务成功。随后只对已停止的历史记录做本地读取保护复核，没有再次请求 Claude API。
- **BLOCKED：Grok 原生隔离创建的环境路径。** 实际 CLI 停留在 `Creating worktree...`，未完成创建，已停止。隔离目录延迟出现/消失/恢复的 Host 合同使用受控 MemorySession 与真实 Git worktree 验证，不将其称为 Grok API 成功。
- **Completion gate 尚未满足。** 此轮是提出者授权 Agent 操作的验收，尚无提出者对新结果的明确 ACCEPTED。依 #100 的 Completion gate，PR #109 与 Issue #100 保持 OPEN；不替用户签署接受，不把跳过或环境阻塞计入 PASS。

## 测试载体

- 使用当前分支源码构建的真实 macOS Tauri `.app`，产品名 `Agent Taskboard Acceptance`，独立 bundle ID `com.youjiaxing.agent-taskboard.acceptance100`，独立持久化目录。并非已发布安装包验收。
- GitHub Project 指向独立测试 clone，origin 为真实 `youjiaxing/agent-taskboard`；代码主工作目录未交给测试 Run 修改。
- Local Markdown Project 为独立 Git 仓库，含 5 张测试 Issue，覆盖 blocked、Frontier、claimed、resolved、parent/Dependency 与 30 节长正文。
- 电脑浏览器连接同一真实 Host；手机为该产品 Client 的 **390×844 浏览器视口**，不是实体手机 Safari/网络配对测试。原生窗口与浏览器的导航分离、Run 状态共享。
- 工作目录与原始证据：`/tmp/issue-100-real-20260912`。下文精选截图已纳入仓库。

## Required user tasks

| 任务 | 本轮真实结果 | 观察与可复查证据 |
| --- | --- | --- |
| 1 空 Host 首个 Project | PASS（原生） | 真正系统目录选择器选测试 clone；推断先展示候选，明确采纳后填入；手填名称保持；首批真实 GitHub Issue 可见。[空 Host/端口提示](real/01-native-empty-host-port-occupied.png)、[首次真实数据](real/03-native-first-project-real-github.png)。 |
| 2 新增、编辑、移除 | PASS（原生） | 从已有侧栏新增 Local Project，编辑名称且重启仍保留；活跃 Run 拒绝移除；执行已停提示 Tracker 认领保留；移除当前 Local Project 后回退 GitHub 且旧 Issue 清空，测试文件仍保留。[活跃 Run 保护](real/08-native-active-run-removal-blocked.png)、[保留认领提示](real/22-native-stopped-claim-removal-warning.png)、[回退](real/20-native-project-removal-fallback.png)。 |
| 3 找工作与上下文 | PASS（原生/浏览器） | Local 四列为 1 blocked / 2 Frontier / 1 claimed / 1 resolved；父 Issue、依赖概览、单 Issue 一跳与展开闭包、返回概览；30 节正文滚动跨轮询与图/看板切换未跳顶。原生停在 Local Issue，浏览器选 GitHub #124，互不抢焦点。[概览](real/09-native-dependency-overview.png)、[长文](real/12-native-long-document-scrolled.png)、[切页后](real/13-native-scroll-after-board-switch.png)。筛选与键盘细项另有自动化，不扩称全部真人手工覆盖。 |
| 4 绑定 Issue 开 Run | PASS（真实 GitHub + Codex） | 打开配置表时 #124 未认领且无新 Run；选择 Codex/model/effort/权限，提交后才认领；官方 CLI 实际执行限定的测试指令。[CLI 完成](real/05-browser-real-codex-comment-success.png)。 |
| 5 游离 Run | PASS（原生入口/认领边界）；隔离 CLI 环境 BLOCKED | Project 行“新建”使用同一表单，初始空白，填写后产生未绑定 Run；前后各 Issue 认领状态不变。Grok CLI 已实际启动但未完成原生隔离创建，停止后未改变 Issue。[提交前表单](real/14-native-unbound-before-start.png)。 |
| 6 运行中 | PASS（真实 Codex/PTY） | Terminal 与完整 Issue 身份一致；官方授权提示可操作；手机一行注入收到真实 assistant 回复 `MOBILE_FIXED_OK`，无需额外 Enter；Codex 在 clone 创建 `acceptance-output.txt`，查看改动显示 `CHANGE_VIEW_OK`；停止、继续与放领完成。[手机实际回复](real/16-mobile-fixed-injection-real-reply.png)、[真实文件 diff](real/19-browser-real-agent-file-diff.png)。 |
| 7 结束后 | PASS（原生/真实 Tracker） | 停止与异常退出保留 OPEN/认领；未进入最近完成；放领后回 Frontier。原生改动行备注仅存在 Host 本地，下一次游离开场白带入路径/行/备注；隔离开关下次恢复关闭。[本地备注](real/17-native-real-diff-note.png)、[下一次开场白](real/18-native-next-opening-notes.png)。 |
| 8 异常恢复 | PASS（真实 PTY/原生部分）；其余为自动化合同 | 向经产品创建的真实 Codex PTY 子进程发 SIGKILL，UI 显示执行已停、继续和释放，#124 仍 OPEN；点击继续启动真实新 PTY 并保存 previousRunId。SIGTERM 另观察到正常 exit=0，不错误计为 abnormal。原生端口占用提示和无效 Local 元数据修复后 watcher 恢复亦已执行。网络离线/限流/鉴权、未安装 Agent、启动失败及隔离目录消失沿用可运行的受控 E2E，不冒充真实第三方故障。[真实硬中断](real/21-browser-real-pty-hard-disconnect.png)、[实际继续](real/23-browser-real-pty-continued.png)。 |
| 9 手机 | PASS（390×844 浏览器） | 看板/票/Run、完整正文、开停 Run、可读最近输出与一行注入；完整 Terminal 作为逃生入口，手机无完整 diff 面板。停止后最近输出保留。[实际注入结果](real/16-mobile-fixed-injection-real-reply.png)。 |

## 实测失败与修复

1. **原始 ANSI 控制码与 TUI 重绘泄漏到手机文本。** [修复前](real/07-mobile-raw-ansi-output-failure.png)。Host 现在保留原始 PTY 流供官方 Terminal，同时用增量 VT 屏幕投影提供可读输出；按 Run/Host 隔离手机缓存，防止旧请求覆盖新 Run。备用屏退出前保存最后答案，停止和 Host 重启后仍可读。PTY 与投影共同限制在 512 列×256 行，避免异常尺寸导致过大分配。
2. **LF 不等于官方 TUI 提交；单独 CR 仍被粘贴缓冲。** 真实 Codex 先后复现两种失败；一行注入改为标准 bracketed paste 包裹正文后提交 CR。正常产品按钮最终获得 `MOBILE_FIXED_OK` 回复。开场白与 self-check 使用同一提交格式；完整 Terminal 原始输入通路保持独立。
3. **原生隔离目录创建晚于启动。** [修复前错误主目录 diff](real/25-native-late-worktree-wrong-diff-root.png)。Host 在后台确认新 worktree，不占导航锁；未确认时展示原因、不给出主目录 diff，也不能静默在主目录继续。旧版错误主目录记录同样受保护。保存启动前的 commit，再映射到确认后的目录，避免吞掉快速提交。
4. **隔离恢复与归属。** Host 重启仍能发现原 Run 目录；历史失败 Run 不阻塞新 Run，也不能抢后续 Run 的目录。同物理 Project 有活跃且未确认的隔离启动时，第二次隔离启动在认领/创建 PTY 前拒绝；待确认或停止前一个 Run 后可重试，普通游离 Run 并发仍允许。真实 Git worktree 延迟创建及已提交修改有独立回归；MemorySession 仅用于控制 Agent 时序，不计真实 Agent API 成功。
5. **明确错误反馈。** 原生复查发现 `viewChanges` 直接返回错误会让面板不开；未确认目录现在返回 `available=false` 的查看结果，由现有面板显示恢复原因，且返回零仓库改动。[最终原生提示](real/26-native-isolation-unconfirmed-fixed.png)已从真实历史 Run 入口复验。

## 实际副作用与清理

- #124 最终 **OPEN，assignees=[]**。这轮官方 Codex 仅新增一条获准测试评论：[ok 2026-09-12 06:22:42 UTC](https://github.com/youjiaxing/agent-taskboard/issues/124#issuecomment-5644169158)。Taskboard 本身仍只对执行生命周期认领/放领；该评论由真实 CLI 在测试指令下执行。
- 真实异常 Run `09ad5f29699813bdadac2cd1fa96d23b` 的结束原因为 abnormal；产品“继续”产生 `2e0f20d9e2ea3ca43b22e2b70544096e`，previousRunId 指向前者。之后已停止并放领。
- 所有验收 Run 已停止；专用验收 Host 已停止，原 `npm run dev` Host 已恢复并从浏览器确认原 Project；Local Project 注册已移除，测试仓库、独立 Host 历史和 GitHub 测试 clone 保留，便于复查。#100 原认领状态未改动。
- 一次 Codex 升级提示误触发了全局 npm 更新，立即停止更新进程并恢复原 **Codex 0.153.4**，命令版本复核一致；未保留版本变更。后续升级提示通过分步核对选择，未再次触发更新。

## 验证与后续门槛

`cargo test --workspace --all-targets` **367 项独立测试、0 失败**，其中 Board 浏览器 38 项；日志含一个子进程重复 PASS。最终命令退出结果见 [验证记录](validation-real-2026-09-12.txt)。Standards 与 Spec 两路 review 的已确认问题均已修复；最终复核均无剩余代码阻断项。此次交付不把 #90 的系统安装、自启、通知、updater 等平台项提升为 PASS，仍按主矩阵保留 BLOCKED/不属于 #100 范围。

用户授权 Agent 完成操作，已落实为本报告，不再要求用户重复全部步骤。但 Issue 原有明确接受门槛仍需提出者对这些具体结果给出结论；未获得该结论前不合并关票。
