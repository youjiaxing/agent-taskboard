# 调研：GitHub Issues 作为 Tracker Adapter 的能力边界

> 对应 wayfinder 票：[#4](https://github.com/youjiaxing/agent-taskboard/issues/4)  
> 调研日期：2026-08-10  
> 范围：GitHub.com 官方 REST / GraphQL / GitHub CLI（`gh`）可见能力；对照 Agent Taskboard v1 与本仓库 wayfinder 约定所需的读写与依赖语义。  
> 资料原则：以 GitHub 官方 REST/GraphQL 文档、产品文档与 changelog 为主；辅以本仓库 live 探针校验字段是否真实返回。

## 1. 结论摘要

对 **Agent Taskboard v1（GitHub 优先）**，GitHub Issues **已足以一等建模** 下列核心能力，无需再靠 body 约定作为主路径：

| 能力 | 一等建模？ | 首选接口 | 备注 |
|------|------------|----------|------|
| 列表 / 筛选 | ✅ 一等 | REST list / Search / GraphQL / `gh issue list` | 依赖「是否被阻塞」的搜索有文档 qualifier；**可靠前沿计算建议用 summary 字段** |
| 创建 | ✅ 一等 | REST `POST .../issues` / GraphQL `createIssue` / `gh issue create` | REST 创建**不能**同请求设 parent/依赖；需二段式 |
| Label 与 open/closed 状态 | ✅ 一等 | REST labels + `state`/`state_reason` | 与 wayfinder 五角色 label 词表完全契合 |
| Sub-issues（map→child） | ✅ 一等 | REST `/sub_issues` / GraphQL `addSubIssue` / `gh --parent` | 单 parent；每 parent 最多 100；嵌套最多 8 层 |
| Native `blocked_by` 依赖 | ✅ 一等 | REST `/dependencies/blocked_by` / GraphQL `addBlockedBy` / `gh --blocked-by` | GA；每侧关系最多约 50；**开/关阻塞计数有区分** |
| Assignee（认领） | ✅ 一等 | REST assignees / GraphQL / `gh --add-assignee @me` | 每 issue 最多 10 个 assignee |
| 评论 | ✅ 一等 | REST comments / GraphQL `addComment` / `gh issue comment` | Resolve 写回答案无障碍 |
| 关闭 | ✅ 一等 | PATCH `state=closed` + `state_reason` / `closeIssue` / `gh issue close` | `completed` / `not planned` / `duplicate` |

**必须降级或用约定弥补的场景**（非 v1 常态，但 Adapter 应预留）：

1. **旧版 GitHub Enterprise Server** 或未开通 sub-issues / dependencies 的环境 → 回落本仓库已有约定：map 正文 task list + 子票 `Part of #map`；子票正文 `Blocked by: #n`。
2. **REST 创建「一枪打完」** → parent / blocked_by 需后续 REST 或改用 GraphQL `createIssue(parentIssueId)` + `addBlockedBy` / `gh issue create --parent --blocked-by`。
3. **仅靠 Search 做 Frontier** → `is:blocked` 等 qualifier 有官方 changelog 说明，但 search 索引与二次限流使「唯一真相」不可靠；**应以 `issue_dependencies_summary.blocked_by`（仅计 open blocker）+ `assignees` 为空** 为门闩。
4. **跨仓库层级** → sub-issue 的 REST 约束为 **同一 owner**；依赖跨仓能力产品侧可链 URL，Adapter 应假设 **默认同仓**，跨仓作扩展。
5. **Issue Types / Issue Fields** → org 级能力，用户个人仓上 fields 受限；wayfinder 的 `wayfinder:*` **label 词表仍是可移植的主分类**，不必绑 Issue Type。

**对 Tracker Adapter 的直接建议**：v1 GitHub Adapter 可把 **Issue / Label / State / Assignee / Comment / Sub-issue / Blocked-by** 全部做成一等领域操作；body 约定仅作 **兼容降级** 与 **人类可读冗余**，不再是主存储。

---

## 2. Taskboard / wayfinder 所需语义对照

来源：`CONTEXT.md` 领域词 + `docs/agents/issue-tracker.md` wayfinding 操作。

| 领域概念 | 本仓库约定 | GitHub 原生对应 |
|----------|------------|-----------------|
| Map | 单条 issue，label `wayfinder:map` | Issue + Labels；children = **sub-issues** |
| Child ticket | map 的 sub-issue；`wayfinder:<type>` | Sub-issue + Labels（或 org Issue Type） |
| Dependency | native blocked_by；否则正文 `Blocked by:` | **Issue dependencies**（`blocked_by` / `blocking`） |
| Frontier | 未关闭 ∧ 无 open blocker ∧ 无 assignee | `state=open` + `issue_dependencies_summary.blocked_by==0` + `assignees=[]` |
| Claim | `gh issue edit --add-assignee @me` | Assignees API |
| Resolve | 评论答案 → close → 回写 map | Comments + Close（`state_reason=completed`） |

---

## 3. 能力分项

### 3.1 列表 / 筛选

#### REST

- **List repository issues**  
  `GET /repos/{owner}/{repo}/issues`  
  查询参数（官方文档）：`state`（open/closed/all）、`labels`（逗号分隔）、`assignee`（login / `none` / `*`）、`creator`、`mentioned`、`milestone`、`type`、`sort`、`direction`、`since`、分页等。  
  文档：<https://docs.github.com/en/rest/issues/issues#list-repository-issues>

- **重要**：Issues 端点会把 **PR 也当 issue 返回**；用是否存在 `pull_request` 键区分。

- **列表/单条响应已含摘要字段**（本仓库 live 验证 + OpenAPI `issue` schema）：
  - `issue_dependencies_summary`: `{ blocked_by, blocking, total_blocked_by, total_blocking }`
  - `sub_issues_summary`: `{ total, completed, percent_completed }`
  - `parent_issue_url`（子票时指向 parent 的 API URL；非嵌套对象）

- **Search**  
  `GET /search/issues?q=...`  
  文档：<https://docs.github.com/en/rest/search/search#search-issues-and-pull-requests>  
  Changelog（Dependencies GA，2025-08-21）声明搜索 qualifier：
  - `is:blocked` / `is:blocking`
  - `blocked-by:` / `blocking:`（可指向具体 issue）  
  来源：<https://github.blog/changelog/2025-08-21-dependencies-on-issues/>

#### GraphQL

- `repository.issues` / `search(type: ISSUE)`  
- `Issue` 上可一次取：`labels`、`assignees`、`state`、`parent`、`subIssues`、`blockedBy`、`blocking`、`issueDependenciesSummary`、`subIssuesSummary`（schema introspection + live query，2026-08-10）。

#### CLI（`gh` ≥ 2.94.0；本机探针 2.95.0）

- `gh issue list --state --label --assignee --search --type --json ...`  
- JSON 字段含：`blockedBy`, `blocking`, `parent`, `subIssues`, `subIssuesSummary`, `assignees`, `labels`, `state`, …  
- Changelog：<https://github.blog/changelog/2026-06-10-manage-sub-issues-types-and-dependencies-from-github-cli/>

#### Adapter 含义

| 需求 | 支持度 | 建议 |
|------|--------|------|
| 按 label / state / assignee 列表 | 一等 | REST list 即可 |
| Map 下 children | 一等 | `GET .../issues/{map}/sub_issues` 或 GraphQL `subIssues`（保序，见 3.4） |
| Frontier（无阻塞、未认领） | 一等（字段级） | **读 `issue_dependencies_summary.blocked_by`（open only）+ assignees 空**；不要单独信任 search |
| 「被 #n 阻塞」的全局搜索 | 文档级一等 | 可用 `blocked-by:`；实现时要处理 search 延迟与 secondary rate limit |

**`blocked_by` vs `total_blocked_by`（关键）**

GraphQL `IssueDependenciesSummary` 字段说明（introspection）：

- `blockedBy`：**当前仍打开**的 blocker 数量（live gate）
- `totalBlockedBy`：open + closed 合计
- `blocking` / `totalBlocking` 同理

REST 的 `issue_dependencies_summary.blocked_by` 与上述 live 语义一致（wayfinder 文档亦写「open blockers only」）。  
**关闭 blocker 后，被阻塞方会自动「解锁」计数，无需再删边。**

---

### 3.2 创建

#### REST

`POST /repos/{owner}/{repo}/issues`  
Body：`title`（必填）、`body`、`labels`、`assignees`、`milestone`、`type`、`issue_field_values` 等。  
**无** `parent` / `blocked_by` 字段。  
文档：<https://docs.github.com/en/rest/issues/issues#create-an-issue>

注意：无 push 权限时 labels/assignees/milestone 会被 **静默丢弃**。

#### GraphQL

`createIssue` input 含：`repositoryId`, `title`, `body`, `labelIds`, `assigneeIds`, **`parentIssueId`**, `issueTypeId`, `issueFields`, …  
**创建时仍无 blocked_by**；依赖用后续 `addBlockedBy`。

#### CLI

```text
gh issue create --title ... --body ... --label ... --assignee @me \
  --parent <n> --blocked-by <n>,<n> --blocking <n>
```

#### Adapter 含义

- **一等创建** title/body/labels/assignees。  
- **层级 + 依赖**：优先 `gh` 封装或 GraphQL 多 mutation；纯 REST 需：create → add sub-issue → add blocked_by。  
- 返回值务必同时存 **`number`（人类/#）**、**`id`（REST 依赖/sub-issue 用 database id）**、**`node_id`（GraphQL）**。

---

### 3.3 Label 与状态

#### Labels

REST：

- 仓级：`GET/POST /repos/{owner}/{repo}/labels` …
- Issue：`GET/POST/PUT/DELETE .../issues/{issue_number}/labels`  
  文档：<https://docs.github.com/en/rest/issues/labels>

GraphQL：`addLabelsToLabelable` / `removeLabelsFromLabelable` / `clearLabelsFromLabelable`；`updateIssue(labelIds|labels)`。

CLI：`gh issue edit --add-label` / `--remove-label`；create 时 `-l`。

**一等**。wayfinder 的 `needs-triage` / `wayfinder:map` / `wayfinder:research` 等 **完全可落在 labels 上**，无需 Projects 自定义字段。

#### State

- 模型：`open` | `closed`（二态；**不是**看板多列 status）。
- `state_reason`（REST PATCH / GraphQL close）：`completed` | `not_planned` | `duplicate` | `reopened` | null。  
  文档 Update an issue：<https://docs.github.com/en/rest/issues/issues#update-an-issue>
- CLI：`gh issue close -r completed|not planned|duplicate`；`--duplicate-of`。

**与 Taskboard**：「可执行 / 已完成」用 open-closed 即可；更细的 triage 用 **labels**，不要指望 GitHub Issue 自带多状态机（Projects 单选字段是另一条线，v1 可不绑）。

---

### 3.4 Sub-issues

#### 产品语义

- 把大工作拆成子工作；**树形层级**（非依赖 DAG）。  
- 限制：**每个 parent 最多 100 个子 issue**；**最多嵌套 8 层**。  
- 权限：至少 triage。  
- 文档：<https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/adding-sub-issues>

#### REST（OpenAPI `api.github.com` + 官方 docs）

| 方法 | 路径 | 作用 |
|------|------|------|
| GET | `/repos/{owner}/{repo}/issues/{issue_number}/sub_issues` | 列出子 issue |
| POST | 同上 | 添加子 issue；body: `sub_issue_id`（**database id**）, 可选 `replace_parent` |
| DELETE | `/repos/{owner}/{repo}/issues/{issue_number}/sub_issue` | 移除；body: `sub_issue_id` |
| PATCH | `/repos/{owner}/{repo}/issues/{issue_number}/sub_issues/priority` | 重排；`sub_issue_id` + `after_id`/`before_id` |

文档索引：<https://docs.github.com/en/rest/issues/sub-issues>

REST 说明：`sub_issue_id` 必须属于 **与 parent 相同的 repository owner**（可跨同 owner 下不同 repo；UI 亦支持选其他仓）。

#### GraphQL

- 读：`parent`, `subIssues`, `subIssuesSummary`
- 写：`addSubIssue`, `removeSubIssue`, `reprioritizeSubIssue`
- `createIssue(parentIssueId: ...)` 可创建时挂 parent

#### CLI

- create：`--parent`
- edit：`--parent` / `--remove-parent` / `--add-sub-issue` / `--remove-sub-issue`
- list/view JSON：`parent`, `subIssues`, `subIssuesSummary`

#### Adapter 含义

| 点 | 结论 |
|----|------|
| Map → children | **一等**，应用 sub-issues，而不是 task list 主路径 |
| 单 parent | 一等约束：一个 child 只能有一个 parent（DAG 依赖请用 dependencies） |
| 顺序 | **一等**（priority API / GraphQL reprioritize）；Frontier「map 顺序第一个」可依赖 list 顺序 |
| 进度 | `sub_issues_summary` 可展示 map 完成度 |
| 降级 | API 404/410 或功能不可用 → task list + `Part of #map`（现有约定） |

本仓库 live：`GET .../issues/1/sub_issues` 返回 #2–#15；GraphQL `issue(1).subIssues` 带 `parent.number=1`。

---

### 3.5 Native `blocked_by` 依赖

#### 产品语义

- Relationships：**Mark as blocked by** / **Mark as blocking**。  
- 计划：Free / Pro / Team / Enterprise Cloud。  
- 权限：至少 triage。  
- 每侧关系类型最多链 **约 50** 个 issue（Dependencies GA changelog）。  
- 文档：<https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-issue-dependencies>  
- GA changelog：<https://github.blog/changelog/2025-08-21-dependencies-on-issues/>（API + webhooks 全支持）

#### REST

| 方法 | 路径 | Body / 说明 |
|------|------|-------------|
| GET | `.../issues/{issue_number}/dependencies/blocked_by` | 列出阻塞本 issue 的 issues |
| POST | 同上 | `{ "issue_id": <blocker database id> }` |
| DELETE | `.../dependencies/blocked_by/{issue_id}` | 按 blocker 的 **database id** 删除 |
| GET | `.../dependencies/blocking` | 列出本 issue 阻塞的 issues |

文档：<https://docs.github.com/en/rest/issues/issue-dependencies>

**ID 陷阱（本仓库 wayfinder 已写明，API 再确认）**：

- 路径里的 `{issue_number}` = `#` 号  
- POST/DELETE 的 `issue_id` = 响应里的 **`id` 字段**（database id），**不是** `number`，也不是 `node_id`  
- 取法：`GET .../issues/{n}` → `.id`

#### GraphQL

- 读：`blockedBy`, `blocking`, `issueDependenciesSummary`
- 写：`addBlockedBy(issueId, blockingIssueId)`，`removeBlockedBy`（均为 **global node ID**）

#### CLI

- create：`--blocked-by` / `--blocking`
- edit：`--add-blocked-by` / `--remove-blocked-by` / `--add-blocking` / `--remove-blocking`（接受 **number 或 URL**，CLI 内部解析 id）
- view/list JSON：`blockedBy`, `blocking`

#### 与 Taskboard Dependency / Frontier

| 语义 | GitHub | 是否足够 |
|------|--------|----------|
| A 阻塞 B（B blocked by A） | POST B/dependencies/blocked_by issue_id=A.id | ✅ |
| 是否仍被阻塞 | `summary.blocked_by > 0`（仅 open） | ✅ 与「blocker 关闭即解锁」一致 |
| 列出阻塞边 | GET blocked_by / GraphQL `blockedBy` | ✅ |
| 多阻塞方 / 多被阻塞方 | 多对多边（非 tree） | ✅ 补 sub-issue 的不足 |
| 关边 | DELETE / removeBlockedBy | ✅ |

**降级**：功能不可用 → 正文首行 `Blocked by: #n, #n`；解析 open 状态自行计算（现有约定）。

本仓库 live：#4 `blocking` → #11；#11 `blockedBy` → #4,#5,#6；summary `blocked_by: 3`。

---

### 3.6 Assignee（认领）

#### REST

- 创建/更新 issue 时可设 `assignees`  
- `POST/DELETE .../issues/{issue_number}/assignees`，body `{ "assignees": ["login"] }`  
- 文档写明：**最多 10 个 assignee**  
  <https://docs.github.com/en/rest/issues/assignees>

#### GraphQL

`addAssigneesToAssignable` / `removeAssigneesFromAssignable` / `replaceActorsForAssignable`；`updateIssue(assigneeIds|assignees)`。

#### CLI

`gh issue edit --add-assignee @me` / `--remove-assignee`；`@me`、`@copilot` 特殊值。

#### Adapter 含义

- **Claim = 写入 assignee**：**一等**，与 wayfinder 一致。  
- Frontier 条件「尚未被占用」= **assignees 为空**（或产品定义是否允许「仅自己」——v1 建议：任何 assignee 即占用）。  
- 列表筛未分配：REST `assignee=none` 或 search `no:assignee`。

---

### 3.7 评论与关闭

#### 评论

- REST：`GET/POST .../issues/{issue_number}/comments`（body 必填）  
  <https://docs.github.com/en/rest/issues/comments>
- GraphQL：`addComment`；读 `comments`
- CLI：`gh issue comment -b "..."`；`gh issue view --comments`

**一等**。Resolve 时「先评论答案再关闭」无缺口。

#### 关闭

- REST PATCH：`{ "state": "closed", "state_reason": "completed" }`（或 not_planned / duplicate + `duplicate_issue_id`）
- GraphQL：`closeIssue(stateReason, duplicateIssueId, rationale, …)` / `reopenIssue`
- CLI：`gh issue close [-c comment] [-r reason] [--duplicate-of n]`

**一等**。关闭 **不会**自动改 labels；若 triage 词表要变，Adapter 需显式改 label。

---

## 4. 接口对照速查（实现向）

### 4.1 REST 最小集合（GitHub Adapter v1）

```
GET    /repos/{o}/{r}/issues?state&labels&assignee&per_page&page
GET    /repos/{o}/{r}/issues/{n}
POST   /repos/{o}/{r}/issues
PATCH  /repos/{o}/{r}/issues/{n}          # state, state_reason, title, body, labels, assignees

GET    /repos/{o}/{r}/issues/{n}/labels
POST   /repos/{o}/{r}/issues/{n}/labels
DELETE /repos/{o}/{r}/issues/{n}/labels/{name}

POST   /repos/{o}/{r}/issues/{n}/assignees
DELETE /repos/{o}/{r}/issues/{n}/assignees

GET    /repos/{o}/{r}/issues/{n}/comments
POST   /repos/{o}/{r}/issues/{n}/comments

GET    /repos/{o}/{r}/issues/{n}/sub_issues
POST   /repos/{o}/{r}/issues/{n}/sub_issues          # sub_issue_id
DELETE /repos/{o}/{r}/issues/{n}/sub_issue           # sub_issue_id
PATCH  /repos/{o}/{r}/issues/{n}/sub_issues/priority

GET    /repos/{o}/{r}/issues/{n}/dependencies/blocked_by
POST   /repos/{o}/{r}/issues/{n}/dependencies/blocked_by  # issue_id
DELETE /repos/{o}/{r}/issues/{n}/dependencies/blocked_by/{issue_id}
GET    /repos/{o}/{r}/issues/{n}/dependencies/blocking
```

API 版本头示例：`X-GitHub-Api-Version: 2026-03-10`（文档当前 latest）。

### 4.2 GraphQL 读写要点

| 操作 | 字段 / Mutation |
|------|-----------------|
| 读 Frontier 批量 | `subIssues { number state assignees issueDependenciesSummary { blockedBy } }` |
| 挂 child | `addSubIssue` / `createIssue(parentIssueId)` |
| 加依赖 | `addBlockedBy(issueId, blockingIssueId)` |
| 认领 | `addAssigneesToAssignable` |
| 关闭 | `closeIssue(stateReason: COMPLETED)` |
| 评论 | `addComment` |

### 4.3 CLI 与「Agent 驱动」路径

本仓库 agent 文档以 `gh` 为主路径。`gh` ≥ **2.94.0** 后，sub-issues / dependencies / types 已是一等 flag，Agent 脚本可少写 `gh api`。  
仍建议 Adapter 内核走 REST/GraphQL（可测、可在无交互环境跑）；`gh` 作调试与 agent skill 层。

---

## 5. Frontier 查询推荐算法（GitHub）

在 **map = issue M** 上：

1. `children = list_sub_issues(M)`（保持 API 返回顺序；需要时用 priority API 调整）  
2. 过滤 `state == open`  
3. 过滤 `issue_dependencies_summary.blocked_by == 0`（**不要**用 `total_blocked_by`）  
4. 过滤 `assignees` 为空  
5. 取序首（或全部）作为可认领前沿  

可选优化：GraphQL 单查询拉齐 children + summary + assignees，减少 N+1。  
Search 仅作 UI 辅助，不作为解锁真相源。

---

## 6. 一等 vs 降级总表

| 能力 | 一等？ | 降级 / 约定 |
|------|--------|-------------|
| 列表 + label/state/assignee 筛选 | ✅ | — |
| Search `is:blocked` 等 | ⚠️ 文档支持，实现慎用 | 以 summary 字段为准 |
| 创建 issue | ✅ | — |
| 创建时绑定 parent/依赖 | ✅（GraphQL/gh）；REST 需二段 | REST 二段式 |
| Labels | ✅ | — |
| open/closed + reason | ✅ | 多列看板 status → Projects 字段（非 v1 必选） |
| Sub-issues 树 | ✅ | task list + `Part of #` |
| 子票排序 | ✅ | 正文序号 / 创建时间 |
| blocked_by DAG | ✅ | 正文 `Blocked by: #n` |
| 关闭 blocker 自动解锁 | ✅（summary 只计 open） | 约定路径需自查 blocker state |
| Assignee 认领 | ✅ | 正文 `Claimed-by:`（不推荐） |
| 评论 / 关闭 | ✅ | — |
| 跨仓 parent/依赖 | ⚠️ 有限 | 默认同仓；跨仓显式探测 |
| Issue Type / Fields | ⚠️ org/计划相关 | 继续用 labels |
| Webhooks 推送依赖变更 | ✅（changelog 声明） | 轮询 list + `since` |

---

## 7. 风险与实现注意

1. **三种 ID**：`number` / database `id` / GraphQL `node_id` 不得混用。  
2. **PR 混入 issue 列表**：Frontier 与 map children 应排除 `pull_request`。  
3. **权限静默失败**：labels/assignees 无权限时可能不报错；Adapter 写后应 re-fetch 校验。  
4. **限流**：依赖/评论创建触发通知，易 secondary rate limit；批量建票需退避。  
5. **功能探测**：对目标仓试 `GET .../issues/1/dependencies/blocked_by` 与 `.../sub_issues`；404/410/403 时切换降级策略。  
6. **GHES**：勿假设与 github.com 同版本；能力矩阵按探测结果配置。  
7. **关系上限**：~50 依赖 / 侧，100 sub-issues / parent；超大 map 需拆 map 或分页策略。  
8. **单一 parent**：子票不能表达「多归属」；多归属用 labels 或 dependencies，不要硬套 sub-issue。

---

## 8. 对后续票的影响（不修改 map 正文）

- **#11 决策：Tracker Adapter 统一能力面**：GitHub 侧可承诺的能力面应包含  
  `list/filter`, `get`, `create`, `update`, `labels`, `assignees`, `comments`, `close`, `list_children`, `add/remove_child`, `list_blocked_by`, `add/remove_blocked_by`, `dependency_summary`。  
- 统一接口上 **Dependency 与 Parent 必须分模型**（DAG vs tree），与 GitHub 原生一致。  
- GitLab / local markdown 调研应对照同一能力面，标出各自一等/降级。

---

## 9. 主要来源

### 官方 REST

- Issues：<https://docs.github.com/en/rest/issues/issues>  
- Issue dependencies：<https://docs.github.com/en/rest/issues/issue-dependencies>  
- Sub-issues：<https://docs.github.com/en/rest/issues/sub-issues>  
- Labels：<https://docs.github.com/en/rest/issues/labels>  
- Assignees：<https://docs.github.com/en/rest/issues/assignees>  
- Comments：<https://docs.github.com/en/rest/issues/comments>  
- OpenAPI（路径与 body 字段）：`github/rest-api-description` → `api.github.com`（本地拉取核对 2026-08-10）

### 官方 GraphQL

- 端点 schema introspection（`Issue`, `IssueDependenciesSummary`, `SubIssuesSummary`, mutations：`addBlockedBy`, `removeBlockedBy`, `addSubIssue`, `closeIssue`, `createIssue`, …）via `gh api graphql`，2026-08-10  
- 文档入口：<https://docs.github.com/en/graphql>

### 产品文档

- Adding sub-issues：<https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/adding-sub-issues>  
- Creating issue dependencies：<https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-issue-dependencies>  
- Filtering and searching issues：<https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/filtering-and-searching-issues-and-pull-requests>

### Changelog

- Dependencies GA（API/webhooks/search/50 上限）：<https://github.blog/changelog/2025-08-21-dependencies-on-issues/>  
- CLI sub-issues / types / dependencies（v2.94.0）：<https://github.blog/changelog/2026-06-10-manage-sub-issues-types-and-dependencies-from-github-cli/>

### 本仓库约定与探针

- `docs/agents/issue-tracker.md`（wayfinder 操作）  
- Live：`youjiaxing/agent-taskboard` issues #1/#4/#11 的 REST + GraphQL 字段（2026-08-10）  
- 本机 `gh version 2.95.0` 的 `issue create|edit|list|view|close` help

---

## 10. 一句话答案（回票）

> GitHub Issues 对 Taskboard 所需的列表筛选、创建、label/状态、sub-issues、native blocked_by、assignee 认领、评论与关闭 **均可一等建模**（REST + GraphQL + gh≥2.94）；实现时用 **database id / node_id** 写依赖与子票、用 **`issue_dependencies_summary.blocked_by`（open only）** 做 Frontier 门闩。仅在 GHES/未开通高级 Issues、或 REST 单请求创建要附带层级依赖时，才需要 **二段式 API** 或 **正文约定降级**。
