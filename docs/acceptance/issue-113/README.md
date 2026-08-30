# Issue #113 依赖图概览、中心 Issue 与稳定展开验收

验收日期：2026-08-30

## 结论

Issue #113：**PASS**。

- 自动化验证概览模式只展示未关闭 Issue、硬上限 200、截断提示数据、Dependency 参与者稳定优先、概览与中心模式切换，以及两次“从此处展开”的 viewport anchor 保持。
- 真实体验使用本轮源码重新构建的 Release Tauri `.app`、live GitHub Tracker 和本仓实际 Issue；没有把 Vite、RPC、fixture 或单元测试升级成桌面体验证据。

## Release Tauri 桌面壳：PASS

验收 bundle：

`target/release/bundle/macos/Agent Taskboard.app`

- 2026-08-30 15:55（Asia/Shanghai）使用最终源码重新执行本地 Release app bundle 构建；本地验收构建关闭 updater artifact 生成，不改仓库发布配置。
- `codesign --verify --deep --strict` 通过。
- 停止此前同一路径的旧验收进程后，15:56 启动最终 bundle 的独立进程 PID 21692；可执行文件来自上述 bundle。
- `/Applications/Agent Taskboard.app` 的已安装版本没有作为本轮证据。

### 真实未关闭 Issue 概览

直接从看板切到依赖图后进入“依赖图概览”，可见 8 张真实未关闭 Issue：#45、#97、#100、#101、#113、#114、#115、#116。数量与同一时刻 `gh issue list --state open` 一致，已关闭 Issue 没有进入概览。

截图：[8 张真实未关闭 Issue 的概览](issue-113-overview-release-tauri.png)

### 概览、中心 Issue 与返回

1. 在概览点击 #113，标题变为“中心 Issue：#113 …”，Issue Inspector 同步打开 #113，并出现“返回依赖图概览”。
2. 点击“返回依赖图概览”后无需刷新或重新选择 Project，8 张未关闭 Issue 立即恢复。
3. 在概览点击有真实 Dependency 的 #101，中心模式显示直接上游 #100，并提供“查看完整上下游（4 个 Issue）”。

截图：[以真实 #101 为中心并显示直接上游](issue-113-centered-101-release-tauri.png)

### 从看板 Issue 详情跳转

返回看板，打开真实 #100，在 Issue Inspector 的“依赖关系”区点击“查看依赖图”。依赖图直接以 #100 为中心，显示直接上游 #98、#99 与直接下游 #101；没有先落到概览，也没有要求重新选择 Project。

截图：[从 #100 Issue 详情直接进入中心模式](issue-113-board-to-centered-100-release-tauri.png)

### 多次从此处展开

在同一原生窗口连续执行：

1. #101 中心 → 点击 #100 的“从此处展开”；#100 保持在点击前的可见位置，新增的 #98、#99 排到其上游，视野没有回到画布顶部或突然居中。
2. #100 中心 → 点击 #99 的“从此处展开”；#99 保持在点击位置，#100 重排到右侧，视野仍停留在原处。

截图：

- [第一次原地展开到 #100](issue-113-expanded-100-release-tauri.png)
- [第二次原地展开到 #99](issue-113-expanded-99-release-tauri.png)

## 200 上限与稳定保留规则：PASS

HostKernel 行为测试构造 205 张未关闭 Issue 与 1 张已关闭 Issue：

- 概览 `totalCount = 205`、实际返回 200 个节点、`truncated = true`；
- 全部节点均为未关闭 Issue；
- 较旧的 Dependency 参与者 #1、#2 在新编号普通 Issue 超过上限时仍被保留；
- 选择规则固定为 Dependency 参与者优先，其次 Issue 编号降序，最后以完整 Issue id 打破平局；
- 截断文案明确显示 `{shown}/{total}`，并说明按稳定规则优先保留 Dependency 参与者。

## 自动化验证

专项回归覆盖：

- 直接切换依赖图进入概览；
- 概览一次渲染 Host 稳定选择的全部节点；60 张未关闭 Issue 时不出现中心图专用的 48 节点“显示更多”；
- 概览点击 Issue 进入中心模式；
- 返回概览；
- Remote Host 从中心/完整图切回看板后，再进入依赖图会先恢复远端概览；
- Issue Inspector 入口直接进入中心模式；
- 两次“从此处展开”前后，被点击 Issue 的 viewport x/y 偏差不超过 2px；
- 旧协议 payload 的新增字段默认值兼容；
- 图仍只画 Dependency，不把父子关系画成边。

最终验证命令记录在实现提交与 PR 中；浏览器和内核回归只计自动化证据，不能替代上面的 Release Tauri 实测。
