# Issue #100 核心用户路径验收

验收日期：2026-08-29。

## 结论

自动化结果：**PASS**。九条 Required user task 都有从产品入口出发、通过真实 `HostKernel` loopback 与 Playwright 驱动的场景；断言最终可见结果和 Tracker 认领 / Run / Project 等关键副作用。2026-08-28 真人验收提出的四项缺陷也已补回归覆盖并修复。

Completion gate：**BLOCKED**。仍需提出者按本文最后的 ≤15 分钟脚本，在真实产品壳和真实 GitHub Project 上完成一次连续任务并明确接受。在此之前 Issue #100 与 PR 保持 OPEN，不把 fixture 或逻辑测试升级成提出者验收。

## Required user task 结果

| # | 结果 | 用户路径与证据 |
| --- | --- | --- |
| 1 空 Host 登记首个 Project | PASS | `project-registration.mjs`：从空 Host 的「登记 Project」进入，目录推断只产生候选，用户确认后看到首批 Issue；失败保留草稿并可重试。 |
| 2 已有 Project 新增、编辑、移除 | PASS | `project-management.mjs`：从已有 Project 的桌面侧栏新增并看到首批 Issue，再由行尾菜单完成编辑与移除；活跃 Run 明确禁止移除；执行已停提示 Tracker 认领仍保留；移除当前 Project 后回退且不残留旧 Issue。 |
| 3 找工作 | PASS | `board.mjs`：四列、标题搜索、triage/open/closed 筛选、父子过滤、Project open Issue 依赖概览、单 Issue 一跳上下游、完整连通闭包、图中心与详情分离、返回同一 Issue 上下文、键盘 `j` / Enter / `?`。 |
| 4 绑定 Issue 开 Run | PASS | `run-launch-resilience.mjs` + `board.mjs`：从「执行」进入同一启动配置表；选择 Agent、预填来源、命令预览和隔离说明可见；打开表单后再次核对未认领且无 Run，提交成功后断言同一 Issue 已认领并只创建一条 Run。 |
| 5 游离 Run | PASS | `board.mjs`：从 Project 行「新建」进入同一表单，初始指令可明确填写；打开表单前后及启动成功后逐 Issue 比较认领状态不变，同时出现未绑定 Issue 的运行中 Run。 |
| 6 运行中 | PASS | `run-lifecycle.mjs`：从进行中卡片进入 Terminal，按需加载并保留完整 Issue；等待操作、注入一行、查看改动、停止、继续和释放认领均从可见入口完成；注入与改动备注另覆盖提交中、防重复、失败保留草稿和显式重试。 |
| 7 结束后 | PASS | `run-lifecycle.mjs`：停止 Run 后仍为已认领、未关闭的 Issue，留在进行中；释放认领后回 Frontier；不会进入最近完成。`board.mjs` 另覆盖真正关闭的 Issue、最近完成和结束 Run 的查看改动。 |
| 8 异常恢复 | PASS | `shell-edge-state.mjs` 对离线、限流、鉴权分别断言网络检查、可重试时间或凭据修复位置，并实际触发手动刷新；`agent-unavailable.mjs` 列出 command、PATH 与已知安装位置；`run-launch-resilience.mjs` 覆盖启动失败重试；`run-lifecycle.mjs` 覆盖全部轻量表单、PTY 异常和隔离目录消失恢复；`loopback-occupied.mjs` 覆盖端口占用。 |
| 9 手机 | PASS | `board.mjs` 在 390×844 覆盖看板 / 票 / Run、Host/Project 切换、完整 Issue、启动/停止 Run、只读最近输出与注入一行；不出现完整查看改动，完整 Terminal 仅作逃生入口。 |

## 本票修复的链路断点

- Run 启动表单增加提交中状态，整表禁用并阻止重复 RPC；协议失败和 Host 业务失败都保留完整草稿供显式重试。
- 绑定 Run 成功启动时，Host 原子同步当前 Issue；不会出现 Run 已创建但 Terminal 因身份不一致被隐藏。
- 从 Run 卡片进入 Terminal 时按需加载同一 Issue 正文；不把正文加入看板全量快照。
- Agent 选择页对未安装 Adapter 展示 command、搜索 PATH 与已知安装位置；动作禁用但不再静默。
- 搜索、Terminal 注入、改动备注和自定义用量表单共用异步提交保护：提交中禁用、防重复，协议失败保留完整草稿与错误，原按钮可显式重试。

## 2026-08-28 NOT ACCEPTED 后的修复结果

| 真人反馈 | 结果 | 修复与回归证据 |
| --- | --- | --- |
| 浏览器操作会同步改变桌面 App 当前界面 | PASS | Client 以独立 ID 在本地持有 Host / Project / Issue / Run、看板/依赖图、搜索、父过滤和图模式；每次 RPC 冻结调用当下的 `clientView`。`multi-client-navigation.mjs` 用两个独立浏览器 Client 选择不同 Issue，跨多个 tick 后互不抢焦点；本机与远程 Host Snapshot 都按显式 Client 视图生成。不完整读取会合并保留上次已知 Issue，只有后续成功完整刷新确认 Issue 不存在时，当前 Client 才清理自己的悬空选择。 |
| 依赖图只看到一张 Issue | PASS | 顶栏直达进入最多 200 张 open Issue 的概览；Dependency 参与者优先保留，超限显示总数、展示数与截断提示。点击节点进入一跳上下游，可展开完整闭包；看板卡片有「查看依赖」，单 Issue 模式有「返回依赖概览」，无 Dependency 有明确说明。节点每 50ms 原地追加一批，不重建 toolbar 与 canvas。 |
| 看板 / 依赖图切换延迟数秒 | PASS | Client 导航只发 Snapshot，不触发 Tracker 刷新；刷新、tick 和正文读取在 Host 短锁内创建任务，Tracker 读取在线程中执行，完成后再短锁应用。阻塞 Tracker 时，第二个 Snapshot 的自动化硬门为 250ms；图首批绘制 48 节点，其余每 50ms 自动补齐。 |
| Issue 详情松开鼠标后回到顶部 | PASS | 仅倒计时变化的 tick 跳过全量 DOM 重建；确需重绘时按同一 Issue 恢复 `.detail-scroll`，并按稳定字段定位恢复 Terminal、Usage、搜索与设置表单的焦点、选区和输入内滚动。E2E 将详情滚到 240px，按住鼠标跨两个 tick，松开后仍保持 240px；另强制业务 snapshot 重绘并断言 Usage 时间输入仍聚焦。 |

## 截图

- [活跃 Run 禁止移除 Project](issue-100-active-run-removal-blocked-1280x840.png)
- [移除当前 Project 后回退并编辑](issue-100-project-fallback-1280x840.png)
- [启动失败保留草稿并可重试](issue-100-launch-retry-1280x840.png)
- [绑定 Run 启动后进入 Terminal](issue-100-bound-run-1280x840.png)
- [Agent 未安装的可恢复说明](issue-100-agent-unavailable-1280x840.png)
- [Terminal 与完整 Issue 保持同一身份](issue-100-terminal-and-issue-1280x840.png)
- [运行中查看真实工作树改动](issue-100-view-changes-1280x840.png)
- [Run 结束不伪装成 Issue 完成](issue-100-run-ended-issue-open-1280x840.png)
- [390×844 手机票页](../../../apps/desktop/e2e/baselines/issue-99-mobile-390x844.png)

## 真实 / fixture 边界

- 自动化使用真实产品 `dist`、真实 Client 代码、真实 `HostKernel`、loopback HTTP/RPC、真实 PTY 合同与真实 git 工作树读取。
- Tracker 使用 `MemoryTracker` / `SeamTracker` fixture；只验证 Taskboard 的认领 / 释放认领写边界，不把它称为真实 GitHub 写操作证据。
- Playwright 同一套 Client 代码覆盖桌面浏览器与 390×844 手机；Tauri 系统目录选择、系统通知、自启和 updater 仍属于真实平台边界。
- 截图中的仓库、Issue 和 Run 为 deterministic fixture，不代表真实 GitHub #100 已被认领或关闭。
- 250ms 响应门使用阻塞 `TrackerSeam` fixture，证明慢 Tracker 读取不会占住 Host Snapshot 锁；不把该 fixture 称为真实 GitHub 网络延迟数据。

## #90 壳层 PARTIAL 重新判定

| #90 项 | 2026-08-29 判定 | 当前证据 |
| --- | --- | --- |
| #5 系统自启 | BLOCKED | 设置壳与 Host mode 合同存在；真实登录后自启必须由 macOS/Windows 平台验收，且 #100 明确不做安装/自启。 |
| #20 桌面 Project 行尾编辑/移除 | PASS | `project-management.mjs` 从已有 Project 的侧栏新增 Project，并从行尾菜单完成编辑、活跃 Run 阻止、执行已停警告、移除与回退。 |
| #50 Host 总览分组/过滤 | PASS | `board.mjs` 断言 running/stopped/ended 默认、跨 Project 数据、Project filter 和无 Run 时的 Project 态势。 |
| #86 端口占用提示 | PASS | `loopback-occupied.mjs` 从产品壳看到占用端口、网页入口不可用和桌面窗口可继续用。 |
| #128 用量趋势/慢样本/边界文案 | PASS | `board.mjs` 断言 TTFT 与生成速率分轨、慢样本视觉色差及“不管理代理节点”边界；kernel 测试继续验证按模型近期中位数判定。 |
| #129 Terminal 遥测胶囊/展开 | PASS | `board.mjs` 从 Terminal 胶囊展开模型分轨卡与边界文案；390px 保留主模型胶囊和简表。 |
| #136 系统通知与声音 | BLOCKED | Host 四类事件、跳转与设置合同已有测试；真实系统横幅/声音依赖平台人工验收，不用浏览器权限替代。 |
| #143 主题快切 | PASS | `board.mjs` 逐个切换三主题并断言信息架构/几何不变；390px 断言本 Client 持久化且不覆盖 Host Client。 |
| #144 发布资产安装 | BLOCKED | 属于真实平台安装票；#100 明确不完成安装、托盘、自启、通知与 updater。 |
| #145 updater 下载/确认/安装/拉起 | BLOCKED | 内核门禁和浏览器不可安装边界可自动化；真实替换包和按原模式拉起仍需签名平台环境。 |
| #146 updater 失败/跨版本数据不变 | BLOCKED | 数据目录边界有合同证据；真实 updater 回滚/跨版本迁移未在 #100 执行。 |
| #149 手机用量/遥测精简 | PASS | `board.mjs` 断言隐藏完整筛选/趋势、保留 totals 与 1–3 个 Project 行，以及主模型胶囊和多模型简表。 |

## 自动化命令

```sh
npm --prefix apps/desktop run build
cargo test -p host-kernel --test board -- --nocapture
cargo test -p host-kernel --test refresh --test host_kernel -- --nocapture
cargo test --workspace --all-targets
npm --prefix apps/desktop run verify:release
```

新增核心场景可以单独复跑：

```sh
cargo test -p host-kernel --test board browser_registers_the_first_project_from_an_empty_host_and_retries_failures -- --nocapture
cargo test -p host-kernel --test board browser_manages_projects_from_the_desktop_sidebar_without_losing_context -- --nocapture
cargo test -p host-kernel --test board browser_prevents_duplicate_run_launch_and_preserves_the_failed_draft_for_retry -- --nocapture
cargo test -p host-kernel --test board browser_explains_why_an_agent_is_unavailable_before_launch -- --nocapture
cargo test -p host-kernel --test board browser_keeps_issue_and_run_lifecycles_distinct_through_terminal_actions -- --nocapture
cargo test -p host-kernel --test board browser_clients_keep_independent_issue_navigation -- --nocapture
cargo test -p host-kernel --test refresh loopback_snapshot_stays_responsive_while_tracker_refresh_is_blocked -- --nocapture
cargo test -p host-kernel --test refresh loopback_snapshot_stays_responsive_while_issue_document_load_is_blocked -- --nocapture
cargo test -p host-kernel --test refresh autonomous_host_tick_does_not_follow_a_persisted_remote_focus -- --nocapture
cargo test -p host-kernel --test tracker_seam incomplete_refresh_keeps_a_missing_selection_until_a_complete_read_confirms_deletion -- --nocapture
cargo test -p host-kernel --test host_kernel remote_host_snapshots_honor_each_clients_explicit_issue_navigation -- --nocapture
cargo test -p host-kernel --test board browser_explains_an_occupied_loopback_port_without_disabling_the_client -- --nocapture
```

## 提出者 ≤15 分钟真实验收脚本

1. 在本分支运行 `npm --prefix apps/desktop run tauri -- dev`，打开真实产品壳；不要使用一次性原型或已安装旧版本。
2. 若 Host 为空，登记本仓 Project；否则选择本仓。确认目录推断候选后提交，并等到真实 GitHub Issue 出现或看到明确可重试错误。
3. 同时保留桌面 App 和浏览器 Client：在两边分别打开不同 Issue，切换看板 / 依赖图并等待两轮自动刷新，确认两边不会互相抢 Project、Issue、Run 或视图焦点。
4. 在任一 Client 顶栏直接打开依赖图，确认先看到未关闭 Issue 概览；点击 Issue 进入其 Dependency 上下游，再用「返回依赖概览」退出。回看板后通过卡片「查看依赖」再次进入单 Issue 模式。
5. 搜索并打开 Issue #100；阅读 Problem、Required user tasks、Acceptance criteria 与 Completion gate。把正文向下滚动，按住鼠标跨过至少两个刷新倒计时后松开，确认滚动位置不跳顶。
6. 点击「执行」，查看 Agent、预填来源、工作目录、隔离说明和命令预览；选择已安装 Agent，填写一条可安全停止的指令后点击「启动」。确认只有此时 GitHub #100 被认领并创建一条 Run。
7. 进入 Terminal，确认右侧仍是 Issue #100；向 Run 注入一行；打开「查看改动」，然后停止 Run 或返回看板。
8. 确认 Run 停止没有把 Issue #100 伪装成最近完成；如不继续本票，释放认领。检查 Project / Issue / Run 身份在侧栏、主区、详情与 Terminal 一致。
9. 在本 PR 留言明确 `ACCEPTED` 或列出失败步骤、截图和期望。只有明确接受后才 merge PR 并由 `Closes #100` 关闭 Issue。
