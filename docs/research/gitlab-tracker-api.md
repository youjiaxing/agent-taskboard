# 调研：GitLab Issues 的依赖与关联模型及 API

- **Ticket**: [#5](https://github.com/youjiaxing/agent-taskboard/issues/5)
- **Date**: 2026-08-10
- **Scope**: 为统一 Tracker Adapter 接口预留 GitLab 提供事实；**不**实现适配器
- **Primary sources**（官方文档，调研日可访问）:
  - [Linked issues](https://docs.gitlab.com/user/project/issues/related_issues/)
  - [Issue links API](https://docs.gitlab.com/api/issue_links/)
  - [Linked items (work items)](https://docs.gitlab.com/user/work_items/linked_items/)
  - [Child items](https://docs.gitlab.com/user/work_items/child_items/)
  - [Tasks](https://docs.gitlab.com/user/tasks/)
  - [Issues API](https://docs.gitlab.com/api/issues/)
  - [Quick actions](https://docs.gitlab.com/user/project/quick_actions/)
  - GraphQL reference: `WorkItemWidgetLinkedItems` / `workItemAddLinkedItems` / `Issue.blockedByIssues` 等
  - 对照：GitHub [Issue dependencies REST](https://docs.github.com/en/rest/issues/issue-dependencies)、[Sub-issues REST](https://docs.github.com/en/rest/issues/sub-issues)

---

## 1. 结论摘要

GitLab 把 **「关联 / 阻塞」** 与 **「父子层级」** 拆成两套一等模型，不要混用：

| 概念 | GitLab 表达 | 典型用途 | 与 Taskboard Domain 的对应 |
|------|-------------|----------|---------------------------|
| 横向关系（含阻塞） | **Linked issues / Linked items** | `relates_to` / `blocks` / `is_blocked_by` | **Dependency**（仅 `blocks` / `is_blocked_by`） |
| 纵向层级 | **Child items / Hierarchy** | Epic→Issue→Task；Epic 多层 | 更接近 GitHub **sub-issue** 的「map/child」，但类型与深度规则不同 |

公开 **REST Issue links API** 对 issue↔issue 的三类关系均可 **List / Get / Create / Delete**。  
`blocks` / `is_blocked_by` 在产品文档中标明 **Premium / Ultimate**；`relates_to` 自 13.4 起在 **Free** 可用。

对统一 Tracker Adapter 的核心约束：

1. **Dependency 能力必须按 tier / 实例能力协商**：Free 可能没有原生 blocked 边。
2. **Hierarchy ≠ Dependency**：Frontier 门闩只能用阻塞边，不能用 parent/child 代替。
3. **GitLab 没有「Issue 作为任意 Issue 的 sub-issue」的 GitHub 同构树**；wayfinder map 的子票在 GitLab 上需要显式选型（Epic 子 Issue / Issue 子 Task / 仅链接+标签 / 正文约定）。
4. **跨 project 链接是一等公民**（`target_project_id` + `target_issue_iid`），适配器 ID 空间必须是 **(project, iid)** 或全局 `id`，不能只用 `#number`。

---

## 2. GitLab：阻塞 / 依赖 / 关联如何表达

### 2.1 Linked issues（横向，双向）

- 任意两个 issue 之间的 **双向** 关系；可跨 project / group。
- UI：「Linked items」区块；仅当用户对两端都有可见权限时才显示。
- 关闭「仍有 open blockers」的 issue 时：**警告 / 确认**，不是硬性禁止关闭。
- 关系类型（创建时三选一）：
  1. **`relates_to`**：一般关联，不表示阻塞。
  2. **`blocks`**：源阻塞目标。
  3. **`is_blocked_by`**：源被目标阻塞（与 `blocks` 互为镜像表述；底层仍是一条双向边）。

**Blocking issues**（产品小节）：

- **Tier: Premium, Ultimate**
- 列表与 board 上被阻塞 issue 显示 blocked 图标；阻塞方关闭或关系解除后图标消失。

**Work items 泛化**（Linked items 文档）：

- 同类关系可扩展到 epic / task / OKR / incident 等 work item 类型。
- 关系类型同样是：Relates to / Blocks / Is blocked by。
- 阻塞跟踪在文档中再次标注 **Premium, Ultimate**。
- 快捷命令：`/relate`、`/blocks`、`/blocked_by`、`/unlink`。

### 2.2 Child items / Hierarchy（纵向，树）

与 Linked **正交**：

| 层级边 | 说明 | Tier 要点（文档） |
|--------|------|-------------------|
| Epic → 子 Epic | 最多约 7 层嵌套 | multi-level hierarchy：**Ultimate** |
| Epic → Issue | Issue **最多一个** parent epic；换 epic 会拆旧链 | Epic 本身偏 Premium/Ultimate 规划能力 |
| Issue → Task | Task 为 work item，在 issue 的 Child items 中展示 | Tasks：**Free+** |
| Issue → Issue（GitHub sub-issue 式） | **不是**主路径 | 不能假设与 GitHub sub-issues 同构 |

语义上：**child 完成进度 ≠ blocked 门闩**。Child 用于拆分与进度；blocked 用于协调依赖。

快捷命令：`/set_parent`、`/add_child`、`/remove_child`、`/remove_parent`（及历史 `/epic` 等）。

### 2.3 其它相关但不等于 Dependency 的机制

- **Markdown 交叉引用 / crosslinking**：正文里的 `#n` / URL，非结构化依赖图。
- **Duplicate**：关闭为另一 issue 的重复（Issues API 有 `closed_as_duplicate_of` 一类链接），不是 blocked_by。
- **Related merge requests**：`GET .../issues/:iid/related_merge_requests`，issue↔MR，不是 issue↔issue 依赖。

---

## 3. 公开 API：能读写哪些关系

### 3.1 REST：Issue links API（issue↔issue，稳定主路径）

Base：`/api/v4`

| 操作 | Method | Path | 说明 |
|------|--------|------|------|
| 列出某 issue 的全部链接 | `GET` | `/projects/:id/issues/:issue_iid/links` | 按关系创建时间升序；按调用者授权过滤 |
| 取单条链接 | `GET` | `/projects/:id/issues/:issue_iid/links/:issue_link_id` | 15.1+；返回 `source_issue` / `target_issue` / `link_type` |
| 创建链接 | `POST` | `/projects/:id/issues/:issue_iid/links` | **双向**关系；需有权更新 **两端** issue |
| 删除链接 | `DELETE` | `/projects/:id/issues/:issue_iid/links/:issue_link_id` | 两端同时消失 |

**Create 参数**：

| 参数 | 必填 | 说明 |
|------|------|------|
| `id` | 是 | 源 project ID 或 URL-encoded path |
| `issue_iid` | 是 | 源 issue 的 project 内 iid |
| `target_project_id` | 是 | 目标 project |
| `target_issue_iid` | 是 | 目标 issue iid |
| `link_type` | 否 | `relates_to` \| `blocks` \| `is_blocked_by`；默认 `relates_to` |

**List 响应要点**（每条为「另一端」issue + 关系元数据）：

- 常规 issue 字段：`id`, `iid`, `project_id`, `title`, `state`, …
- `issue_link_id`：关系 ID（删除/点查用）
- `link_type`：相对 **当前被查询 issue** 的视角
- `link_created_at` / `link_updated_at`

**读写能力边界**：

- ✅ 可读全部三类 `link_type`
- ✅ 可写全部三类（受许可与 **许可层** 约束；Free 上 blocking 可能失败——产品层标注 Premium）
- ✅ 跨 project
- ❌ 此 API **不**管理 Epic/Task 父子
- ❌ List **不**单独提供「仅 open blockers」聚合字段；需客户端按 `link_type` + 对端 `state` 过滤
- ⚠️ Issues 资源体上可见 `blocking_issues_count`（该 issue **阻塞了多少**），**不能**替代完整 `…/links` 列表；**没有**与 GitHub `issue_dependencies_summary.blocked_by` 对等的「open blocked_by 计数」嵌在 list issues 默认字段中（至少在 Issues API 文档示例中未作为可过滤/可靠门闩字段强调）

### 3.2 REST：Issues API 中间接相关字段

- `issue_type`：`issue` \| `incident` \| `test_case` \| `task`（过滤/创建维度）
- `epic_id`：关联 epic（写：Premium/Ultimate；可读过滤）
- `has_tasks` / `task_completion_status`：子 task 进度摘要，**不是** dependency
- `blocking_issues_count`：见上

### 3.3 GraphQL（work item 方向，部分 Experiment）

文档 schema 中与适配器相关的能力：

| 能力 | 入口 | 备注 |
|------|------|------|
| Issue 被谁阻塞 | `Issue.blockedByIssues` / `blockedByCount` | 直接服务 Frontier |
| Issue 阻塞谁 | `Issue.blockingCount` 等 | 对称视图 |
| 通用 linked items | `Issue.linkedWorkItems`（17.8+，Experiment）；filter：`RELATED` / `BLOCKED_BY` / `BLOCKS` | 与 REST `link_type` 对齐 |
| Widget | `WorkItemWidgetLinkedItems`：`blocked`, `blockedByCount`, `blockingCount`, `linkedItems` | work item 统一模型 |
| 写 linked | `workItemAddLinkedItems` / `workItemRemoveLinkedItems`（16.3+，**Experiment**） | `linkType` 默认 `RELATED`；单次 work item id 数量有上限（文档：最多 10） |
| 写 hierarchy | `workItemHierarchyAddChildrenItems`（18.2+，**Experiment**）等 | 与 links 分离 |
| Hierarchy 读 | `WorkItemWidgetHierarchy`：`parent`/`children`/`ancestors`/`hasParent`/`hasChildren` | |

**建议**：v1 Adapter 若只做 GitLab Issues 依赖读写，优先 **稳定 REST Issue links**；GraphQL 适合后续 work item 统一与「blocked 摘要」查询优化，但需接受 Experiment 变更风险。

### 3.4 非 API 但可观测的写入口

- UI Linked items
- Quick actions（`/blocks` 等）——最终仍落同一关系模型
- **不推荐**适配器用改 description 模拟 blocking（仅作无 Premium 时的降级，与现 GitHub 正文 fallback 同思路）

---

## 4. 与 GitHub「sub-issue + blocked_by」模型对照

### 4.1 概念拆分对照

| 维度 | GitHub | GitLab |
|------|--------|--------|
| 层级（map/child） | **Sub-issues**：issue 树；子 issue 通常 **单一 parent**；REST `…/sub_issues` | **Child items**：Epic/Issue/Task 类型化层级；Issue 下主子类型是 **Task**，不是任意 Issue |
| 阻塞依赖 | **Issue dependencies**：`blocked_by` / 对称视图 `blocking`；REST `…/dependencies/blocked_by` 与 `…/blocking` | **Linked issues**：`blocks` / `is_blocked_by`（+ 非依赖的 `relates_to`） |
| 一般关联 | 无单独一等「relates to」依赖 API | **`relates_to`** 一等 |
| 跨仓/跨项目 | Sub-issue 要求 sub-issue 与 parent **同 owner**（API 说明）；依赖另有规则 | Links **原生跨 project**（`target_project_id`） |
| 关闭策略 | 依赖用于协作可见性；本仓库 wayfinder 以 open blocker 为 Frontier 门闩 | 关闭被阻塞 issue 时 **警告**，非强制 |
| 许可层 | 依赖/sub-issue 随 GitHub Issues 能力演进（现已有 REST） | **blocking 明确 Premium+**；relates Free |
| 列表门闩字段 | `issue_dependencies_summary.blocked_by`（open blockers）等 | 需 `…/links` 或 GraphQL `blockedBy*`；Issues list 无完整对等摘要 |

### 4.2 API 形状对照（依赖）

**GitHub**（写方向以 blocked_by 为主）：

```http
GET    /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by
POST   /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by
       body: { "issue_id": <database id> }
DELETE /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocked_by/{issue_id}
GET    /repos/{owner}/{repo}/issues/{issue_number}/dependencies/blocking
```

**GitLab**：

```http
GET    /projects/{id}/issues/{issue_iid}/links
POST   /projects/{id}/issues/{issue_iid}/links
       ?target_project_id=…&target_issue_iid=…&link_type=is_blocked_by|blocks|relates_to
DELETE /projects/{id}/issues/{issue_iid}/links/{issue_link_id}
```

映射注意：

- GitHub `issue_id` = **数据库 id**（不是 `#number`）。
- GitLab 创建用 **project + iid**；删除用 **`issue_link_id`**（关系 id），不是对端 issue id。
- GitHub 无 `relates_to`；GitLab 多一种边类型。
- `blocks` vs `is_blocked_by`：同一物理边的两种创建方向；适配器应 **规范化存储为「A blocked_by B」** 一条有向依赖，避免双写。

### 4.3 层级 API 对照（map/child）

**GitHub sub-issues**：`List / Add / Remove / Reprioritize` 均针对 issue 号树。

**GitLab**：无同构的「任意 issue 挂任意 issue」稳定 REST 套件；应组合：

- Epic 成员关系（`epic_id` / 旧 Epics API / work item hierarchy）
- Issue 下 Tasks（work item + Child items UI）
- 或应用层约定（label + `relates_to` / 正文 `Part of`）——与当前 GitHub 不可用 sub-issue 时的 fallback 同级

---

## 5. 对统一 Tracker Adapter 能力面的约束

下列约束可直接喂给后续接口设计（仍不在此 ticket 实现）。

### 5.1 能力矩阵应协商，而非假设全有

建议 Adapter 暴露类似能力探测（名称示意）：

| Capability | GitHub v1 预期 | GitLab 预期 |
|------------|----------------|-------------|
| `deps.read` | ✅ blocked_by / blocking | ✅ via links / GraphQL |
| `deps.write` | ✅ blocked_by | ✅ links；**可能**因 Free tier 仅 `relates_to` |
| `deps.relates` | ❌（可恒 false） | ✅ `relates_to` |
| `hierarchy.sub_issues` | ✅ | ⚠️ **不等价**；可能 `false` 或降级为 `hierarchy.tasks` / `hierarchy.epic_issues` |
| `cross_project_deps` | 有限 | ✅ 一等 |
| `frontier.open_blocker_summary` | ✅ summary 字段 | ⚠️ 需聚合或 GraphQL |

**写依赖失败**时：与现 GitHub 策略一致——降级到正文约定（如 `Blocked by: #…`），并在能力位标记 `deps.write=false` 或 `deps.write.degraded=body`。

### 5.2 Domain 映射规则（建议钉死）

```
Taskboard Dependency  →  GitLab link_type ∈ {blocks, is_blocked_by}
                         （规范化方向：successor is_blocked_by predecessor
                          或 predecessor blocks successor，二选一写死）

Taskboard「相关但不阻塞」→  GitLab relates_to（可选；GitHub 无则忽略）

Taskboard map 子票     →  不可默认 = GitLab Task
                         需单独 ADR：Epic 子 Issue vs 同级 Issue + relates/label vs Task
```

**严禁**：把 parent/child 自动翻译成 Dependency，或把 Dependency 画成 sub-issue 树。

### 5.3 标识与作用域

- GitLab issue 主键推荐：`project_id` + `iid`，或全局数字 `id`（API 字段 `id`）。
- 跨 project 边：Dependency 记录必须带 **两端 project**。
- Frontier 查询：对每个 open issue 拉 links（或 GraphQL blockedBy），过滤 `state != closed` 的 blockers；**不要**依赖关闭硬拦截。

### 5.4 批量与性能

- REST links **按 issue** 拉取 → N+1；大批量 Frontier 需：
  - 并发限制 + 缓存，或
  - GraphQL 批量 `blockedByCount` / `blockedByIssues`
- `workItemAddLinkedItems` 单次最多约 10 个 id → 大批量写要分片。

### 5.5 权限

- 创建/删除 link：文档要求对 **两个** project/issue 均有足够角色（Guest+ 在 17.0 后可链；写仍需更新权限）。
- Adapter token 权限不足时：读可能缺边（授权过滤），写返回错误 → 应映射为明确错误码，而非静默成功。

### 5.6 与本仓库 wayfinder 惯例的落差

本仓库 GitHub 惯例（`docs/agents/issue-tracker.md`）：

- map = 父 issue + **sub-issues** 子票
- blocking = **native dependencies**（`blocked_by`）
- Frontier = 无 open blocker 且未 assignee

迁到 GitLab 时至少要先决策（**非本调研实现范围**）：

1. map 在 GitLab 用 **Epic** 还是 **普通 Issue**？
2. 子票是 **Issue** 还是 **Task**？（Task 在部分 Agent/CLI 工作流里是否一等公民）
3. Premium 不可用时 blocked 边的 **唯一降级格式** 是否与 GitHub 正文格式统一？

---

## 6. 推荐的 Adapter 接口预留（仅形状，不实现）

```text
TrackerAdapter
  capabilities() -> {
    deps: { read, write, relates, cross_project },
    hierarchy: { kind: sub_issues | epic_issues | tasks | none, write },
    frontier: { open_blocker_count_cheap: bool }
  }

  list_blocked_by(issue_ref) -> [issue_ref]      # open + closed 由调用方过滤或参数控制
  list_blocking(issue_ref) -> [issue_ref]
  add_blocked_by(issue_ref, blocker_ref) -> void # GitLab: link_type=is_blocked_by
  remove_dependency(issue_ref, link_or_blocker) -> void
  # optional:
  list_related(issue_ref) -> [issue_ref]         # GitLab relates_to only

  list_children(issue_ref) -> [issue_ref]        # 语义依赖 hierarchy.kind
  add_child / remove_child / set_parent …        # 可按 capabilities 禁用
```

GitLab 实现备注：

- `add_blocked_by(A, B)` → `POST .../issues/{A.iid}/links` with `target_*=B`, `link_type=is_blocked_by`
- 删除优先存 `issue_link_id`；若只有 blocker ref，则 list links 后匹配删除
- `relates` 不参与 Frontier

---

## 7. 事实边界与未决

**已核实（官方文档）**：

- 三类 `link_type` 与 REST CRUD
- 双向关系、跨 project、关闭警告
- blocking 产品能力 Premium/Ultimate；relates Free（13.4+）
- Hierarchy（Epic/Task）与 Linked 分离
- GraphQL linked/hierarchy 多处 **Experiment**
- GitHub 侧 dependencies 与 sub-issues 分 API

**未在本调研中做的**：

- 对公开 `gitlab.com` Free/Premium 租户的实机 403/201 对照（文档已足够做接口预留；实现 ticket 应补集成测试）
- Self-Managed 无许可 EE 与 CE 的边界矩阵
- Work item 全面替换 Issues REST 的时间表
- 具体 TypeScript 接口定稿（属后续设计/ADR）

---

## 8. 对 #1 Map 的可引用 gist（关票用）

> GitLab：横向 Linked issues（`relates_to` / `blocks` / `is_blocked_by`，REST `…/issues/:iid/links` 可 CRUD；blocking 为 Premium+）与纵向 Child items（Epic/Issue/Task）正交。GitHub 的 sub-issue 树与 blocked_by 依赖在 GitLab **不能 1:1 同构**；Adapter 必须能力协商，Dependency 只映射 blocks 边，map/child 需另选层级策略。

---

## References

1. https://docs.gitlab.com/user/project/issues/related_issues/
2. https://docs.gitlab.com/api/issue_links/
3. https://docs.gitlab.com/user/work_items/linked_items/
4. https://docs.gitlab.com/user/work_items/child_items/
5. https://docs.gitlab.com/user/tasks/
6. https://docs.gitlab.com/api/issues/
7. https://docs.gitlab.com/user/project/quick_actions/
8. https://docs.gitlab.com/ee/api/graphql/reference/（schema 字段名）
9. https://docs.github.com/en/rest/issues/issue-dependencies
10. https://docs.github.com/en/rest/issues/sub-issues
