# 调研：local markdown Issue Tracker 常见约定

**Ticket:** [#6](https://github.com/youjiaxing/agent-taskboard/issues/6)  
**日期:** 2026-08-10  
**目的:** 为 Taskboard 的 local markdown Tracker Adapter 提供稳定目标模型的事实基础（v1 不要求完整实现）。

## 问题

在「local markdown 作为 Issue Tracker」实践中（含 Matt Pocock skills 的 local tracker 约定及其他高信任来源），目录结构、正文章节、状态、Blocked by、与 git 的关系分别是什么？哪些约定足够稳定，适合作为 Adapter 目标模型？

## 方法与来源

按「主源优先」阅读并交叉核对：

| 优先级 | 来源 | 角色 |
|--------|------|------|
| 主源 | [mattpocock/skills](https://github.com/mattpocock/skills) 的 `issue-tracker-local.md`（与本机 `setup-matt-pocock-skills` 种子一致） | **权威目录与 Wayfinding 合同** |
| 主源 | 同仓 `to-tickets` / `wayfinder` / `to-spec` / `setup-matt-pocock-skills` / `CONTEXT.md` | 实施票模板、map 章节、领域词 |
| 主源 | 本机已安装的上述 skill 副本（`~/.agents/skills/`） | 与上游 raw 内容对照 |
| 高信任周边 | `yjx-local-tracker-setup`、`yjx-local-kanban`（用户生态对 Matt 约定的可执行解析） | 字段解析、完成语义收敛、机器配置 |
| 周边讨论 | [mattpocock/skills#203](https://github.com/mattpocock/skills/issues/203)（CLI 包装 local markdown） | Matt 明确「不必内建 CLI」 |

未把第三方「通用 skill 目录结构」文章当作 Issue Tracker 约定（它们描述的是 skill 包本身，不是 issue 落盘格式）。

---

## 1. 目录结构

### 1.1 Matt 官方 local 合同（稳定）

Issues / specs 落在仓库内的 **`.scratch/`**：

```text
.scratch/
  <feature-slug>/                 # 一个 feature / effort 一个目录
    spec.md                       # to-spec 产出（实施向）
    map.md                        # wayfinder 的 map（规划向）
    issues/
      01-<slug>.md                # 一票一文件，从 01 起编号
      02-<slug>.md
      …
```

硬性约定（官方反复强调）：

- **一 feature 一目录**：`.scratch/<feature-slug>/`
- **一票一文件**：`.scratch/<feature-slug>/issues/<NN>-<slug>.md`，**禁止**合成单一 tickets 大文件
- **编号从 `01` 起**；`to-tickets` 要求按依赖序编号（blocker 在前）
- Wayfinder 用同一套路径语义：map = `.scratch/<effort>/map.md`，子票 = `issues/NN-<slug>.md`（`<effort>` 与 feature-slug 同形态）

### 1.2 发现 feature 的实践（周边稳定）

可执行解析器（`yjx-local-kanban`）的发现规则：

- 读机器配置 `docs/agents/local-tracker.json` 的 `trackerRoot`（默认 `.scratch`）
- 下列目录中存在至少一个匹配 `^\d+-.+\.md$` 的文件，即视为一个 feature：  
  `<trackerRoot>/<feature>/issues/`

### 1.3 与远程 Tracker 的关系

`setup-matt-pocock-skills`：local markdown 是与 GitHub / GitLab **并列的一等选项**，适合 solo 或无 remote；**不是**叠在 GitHub Issues 上的第二层。

---

## 2. 正文章节与字段形态

### 2.1 字段形态（两套写法并存，解析应都认）

| 形态 | 示例 | 出处 |
|------|------|------|
| 行内粗体字段 | `**Status:** ready-for-agent` | `to-tickets` local 模板（Matt 原生） |
| 普通行字段 | `Status: claimed` | `issue-tracker-local` Wayfinding；周边解析器 |
| `##` 章节 | `## Blocked by` / `## Question` / `## Answer` / `## Comments` | 远程 issue 模板 + local 约定 |

**没有**把 YAML frontmatter 定为 Matt local 标准。元数据写在 **标题附近的正文行**，或 `##` 章节里。

周边解析的实用规则（高信任扩展，适合 Adapter 采用）：

- 从文件头读字段，**遇到第一个 `##` 章节即停止 header 扫描**
- `Blocked by` 可来自 **header 行内字段** 或 **`## Blocked by` 章节**（章节优先于行内时以解析器约定为准；kanban 实现是：有章节则用章节）
- `## Comments` 内的同名字段 **不参与** 元数据解析（避免评论污染）

### 2.2 实施票（`to-tickets` local 模板）

Matt 原生模板：

```markdown
# <NN> — <Ticket title>

**What to build:** <端到端用户可见行为，不是分层实现清单>

**Blocked by:** <编号/标题，或 "None — can start immediately">

**Status:** ready-for-agent

- [ ] Acceptance criterion 1
- [ ] Acceptance criterion 2
```

要点：

- 默认 triage：`ready-for-agent`（票在构造上即可被 agent 领取）
- 验收标准用 checklist
- 避免写具体文件路径/大段代码（易过期）；原型决策片段可例外内联

远程 Tracker 的发行模板则用章节：`## Parent` / `## What to build` / `## Acceptance criteria` / `## Blocked by`。

### 2.3 Wayfinder 子票

合同层：

- 正文核心是 **问题**（`## Question`）
- **`Type:`** 行：`research` / `prototype` / `grilling` / `task`
- **`Status:`** 行：认领 / 完成（见 §3）
- 解决时：在 **`## Answer`** 下写答案，再把 Status 设为 `resolved`
- 评论历史：文件末尾 **`## Comments`** 追加

### 2.4 Map（`map.md`）

内容模型与 Tracker 无关（`wayfinder` skill 定义），local 只是换成文件：

```markdown
## Destination
## Notes
## Decisions so far
## Not yet specified
## Out of scope
```

- Map 是 **索引**，不是决策仓库：决策细节只在对应子票里；map 只记 gist + 链接
- **不在 map 上枚举开放子票**；开放票靠扫 `issues/` 查询
- 周边扩展：map 头可有 `Label: wayfinder:map` 供 workflow 识别（非 Matt 种子强制）

### 2.5 Spec（`spec.md`）

`to-spec` 章节（发布到 local 时即写成 `.scratch/<feature>/spec.md` 一类路径）：

`Problem Statement` / `Solution` / `User Stories` / `Implementation Decisions` / `Testing Decisions` / `Out of Scope` / `Further Notes`。

### 2.6 通用：Comments

全 local 合同：会话与评论历史 **追加** 在文件底部 `## Comments` 下（无独立评论 API）。

---

## 3. 状态（Status）

存在 **两条工作流** 共用 `Status:` 字段，语义不同。Adapter 必须能区分，不能假设单一状态机。

### 3.1 实施票：Triage 五角色（Matt 规范）

`docs/agents/triage-labels.md` 映射的五个 canonical 角色：

| 角色 | 含义 |
|------|------|
| `needs-triage` | 维护者待评估 |
| `needs-info` | 等报告者补充 |
| `ready-for-agent` | 规格足够，可 AFK agent |
| `ready-for-human` | 需要人做 |
| `wontfix` | 不做 |

记录方式：issue 文件顶部附近的 **`Status:`** 行（local）；远程则是 label。

### 3.2 Wayfinder 子票：生命周期（Matt local 合同）

官方 `issue-tracker-local.md`：

| Status | 含义 |
|--------|------|
| （缺省 / 未写 claimed·resolved） | 开放、未认领 |
| `claimed` | 已认领（写盘后再开工） |
| `resolved` | 已解决（离开 frontier，解除下游阻塞） |

周边解析在此基础上的 **稳定扩展**（与 Matt 开放语义兼容）：

| 值 | 处理 |
|----|------|
| 空 / `open` / `ready-for-agent` | 开放未领 |
| `claimed` | 进行中（不进 frontier，**不**解除下游） |
| `resolved` / `wontfix` | 终态（离开 frontier，**解除**下游） |

识别 Wayfinder 票的启发式：**存在 `Type:` 且取值为四类之一**；无 `Type` 则按实施票解析。

### 3.3 完成真源：Matt 种子 vs 周边收敛

| 场景 | Matt 官方 local 种子 | 周边高信任收敛（`+resolved-v1`） |
|------|----------------------|----------------------------------|
| Wayfinder 完成 | `Status: resolved` + `## Answer` | 同左 |
| 实施票 triage | `Status: <triage role>` | 同左 |
| 实施票「做完」 | **种子未写死 `Closed` 字段**；模板停在 `ready-for-agent` | **主完成 = `Status: resolved`**；`Closed: true` 仅 legacy 只读兼容 |
| 解阻条件 | 「blocker 文件均为 `resolved`」（Wayfinding 节原文） | 终态 = `resolved` **或** `wontfix`（及 legacy `Closed: true`） |

对 Taskboard Adapter 的含义：

- **目标模型应把「完成」建模为终态**，local 上优先对齐 **`Status: resolved`（及 `wontfix`）**
- **不要**把 `Closed:` 当作 Matt 官方一等字段；若读取存量板，可兼容 `Closed: true`
- 实施票上 `Status: claimed` 是 **执行锁**，与 triage 角色正交（周边实践）；Matt 种子对实施票 claim 不如 Wayfinder 明确

### 3.4 Frontier（跨 Tracker 稳定语义）

Frontier = **未完成** ∧ **无未完成 blocker** ∧ **未认领**。

Local 上：

- 扫 `.scratch/<effort>/issues/`
- 排序：**编号升序，先到先得**
- Claim：写 `Status: claimed` 并保存（**开工前第一写**）
- Resolve：写答案 → `Status: resolved` → 在 map 的 Decisions so far 追加 gist + 链接

---

## 4. Blocked by

### 4.1 写法

| 形式 | 示例 | 出处 |
|------|------|------|
| 行内列表 | `Blocked by: 01, 02` 或 `**Blocked by:** 01 — Foundation` | Matt Wayfinding / to-tickets |
| 章节列表 | `## Blocked by` + `- 01` / `` `01-foo.md` `` | 远程模板 + 周边兼容 |
| 无依赖 | `None — can start immediately` / `None` | to-tickets |

### 4.2 引用解析（周边已工程化、可作 Adapter 目标）

同一 feature 的 `issues/` 内解析，支持：

1. 文件名：`01-foundation.md` 或 markdown 链接目标的 basename  
2. 纯编号：`01` / `` `01` ``  
3. 编号 + 标题：`01 — Foundation`  
4. 逗号/分号分隔多 blocker  
5. `None…` → 空依赖

图范围：**feature 内**（非跨 `.scratch` 多 feature 的全局图——Matt 合同按 effort/feature 目录组织）。

### 4.3 解阻规则

- 某票的每个 blocker 都处于 **完成/终态** → 该票 unblocked  
- Matt Wayfinding 原文：blocker 均为 `resolved`  
- 周边：`resolved` / `wontfix` / legacy `Closed: true`  
- **claimed 不算完成**，不解除下游

### 4.4 与 GitHub 对照（适配器抽象用）

| 能力 | Local markdown | GitHub（同 skill 族） |
|------|----------------|------------------------|
| 依赖真源 | 正文 `Blocked by` | 原生 issue dependencies；否则回退正文 `Blocked by: #n` |
| 完成 | 文件内 Status（+ 可选 legacy Closed） | issue closed |
| 认领 | `Status: claimed` | assignee |
| 父子 | 同目录 + map 文件；无强制 `Part of` | sub-issue 或 task list + `Part of #map` |

Local 没有「原生 UI 边」——**正文约定即真源**。

---

## 5. 与 git 的关系

### 5.1 已钉在主源上的事实

1. **Issue 是仓库内的普通文件**（默认树 `.scratch/...`），读写 = 改文件，**不依赖** `gh`/`glab`。
2. Local 与 GitHub Issues **二选一配置**，不是「git 远程 + local 双写」。
3. Wayfinder **research** 子流程：发现落在 **一次性分支** `research/<name>`，票上只留 **context pointer**（gist + 链接）；research skill 本身要求把调研 md 写进 repo 既有笔记位置。
4. `implement`：在 **当前分支** 提交实现代码（与 tracker 文件是否同提交未强制）。
5. Wayfinder 允许多会话 **并发改 tracker 文件**（靠 claim 减少撞车）。

### 5.2 主源未强制、实践上开放的点

| 问题 | 主源态度 | 实践含义 |
|------|----------|----------|
| `.scratch/` 是否 gitignore | 官方 skills 仓 `.gitignore` **未**忽略 `.scratch`；也未命令必须提交 | 可进版本库当可审计板，也可本地-only；**Adapter 不应假定已提交或未提交** |
| Issue 状态变更是否单独 commit | 无强制 | 常见是与功能分支同仓编辑；并发靠文件锁语义（claimed）而非 git lock |
| Issue id 与 git | id = **feature 内文件名** `NN-slug.md`，不是 git object id | Adapter 稳定 id 应用路径/文件名，不要用 blob hash |
| 与 worktree | 未在 local tracker 合同里规定 | Taskboard 的 Run/worktree 策略应独立于 tracker 落盘格式 |

### 5.3 Matt 对「CLI 包装」的立场

[#203](https://github.com/mattpocock/skills/issues/203) 提议为 local markdown 加 CLI（确定性 list/filter/改元数据）。Matt 关闭并回复大意：**不必写进 skills 本体**；需要时可在 setup 阶段自行挂接。  
→ Taskboard Adapter **可以**成为那种确定性读写层，且不违背 Matt 设计。

---

## 6. 两条工作流对照（Adapter 必分）

| 维度 | Implementation（to-spec / to-tickets / implement） | Wayfinder（决策 map） |
|------|-----------------------------------------------------|------------------------|
| 目录锚点 | `.scratch/<feature>/` + 可选 `spec.md` | `.scratch/<effort>/map.md` |
| 票文件 | `issues/NN-slug.md` | 同左 |
| 区分字段 | **无** `Type`（或非四类） | **有** `Type: research\|prototype\|grilling\|task` |
| Status 主语义 | triage 五角色 +（周边）`claimed`/`resolved` | 空/open → claimed → resolved |
| 正文核心 | What to build + AC | Question → Answer |
| 完成 | 周边：`Status: resolved`；legacy Closed | `Status: resolved` + Answer + map 索引行 |
| 推荐 skill | `/implement` | `/wayfinder` |

同 feature 目录 **可以混放** 两类票（周边 `mixed` workflow）；未完成的 research 只阻塞其下游，不整板锁死。

---

## 7. 适合作为 Taskboard Local Markdown Tracker Adapter 的目标模型

### 7.1 足够稳定、建议写死为能力合同

1. **布局**  
   - `trackerRoot`（默认 `.scratch`）  
   - `<feature>/issues/<NN>-<slug>.md` 一票一文件  
   - 可选同级 `map.md`、`spec.md`

2. **身份**  
   - Issue id = feature 内稳定文件名（如 `03-ready-impl.md`）  
   - 展示编号 = 文件名前缀 `NN`  
   - 标题 = 首个 `#` 标题（可剥掉前缀编号）

3. **依赖**  
   - 真源：`Blocked by`（行内或 `## Blocked by`）  
   - 解析：编号 / 文件名 / 编号+标题；feature 内图  
   - 解阻：blocker 均终态

4. **状态与 frontier**  
   - 字段名：`Status`  
   - 终态：至少 `resolved`（建议同时认 `wontfix`）  
   - 认领：`claimed`  
   - Frontier：非终态 ∧ 无开放 blocker ∧ 非 claimed；编号序

5. **类型分流**  
   - `Type` ∈ wayfinder 四类 → 决策票  
   - 否则 → 实施票；实施票 triage ⊆ 五角色（可配置映射）

6. **内容读写**  
   - 读全文；写状态/依赖/评论（追加 `## Comments`）；Wayfinder 写 `## Answer`  
   - 无独立评论后端

7. **配置面（项目级）**  
   - 人类合同：`docs/agents/issue-tracker.md` + `triage-labels.md`  
   - 可选机器配置：`docs/agents/local-tracker.json`（`trackerRoot`、`statusRoles`、protocol）  
   - 无配置时仍可按 Matt 默认路径与同名 role 降级工作

### 7.2 刻意不要写死进 v1 目标模型

| 项 | 原因 |
|----|------|
| 必须以 `Closed:` 为完成真源 | Matt 种子未标准化；周边已迁到 `Status: resolved` |
| 必须 YAML frontmatter | 非 Matt 约定 |
| 必须 git commit / 禁止 commit `.scratch` | 策略层，非 tracker 模型 |
| 跨 feature 全局编号 | 合同是 per-feature 编号空间 |
| 与 GitHub Issues 双写 | setup 明确为替代关系 |
| 必须内建 CLI | Matt 明确可不进 skills；Adapter 自身可提供等价 API |
| 强制 `Part of` / map 子列表 | local 靠目录并列 + map 文件，无 GitHub sub-issue |

### 7.3 建议的 Adapter 抽象映射（对接 Taskboard 领域词）

| Taskboard 概念 | Local markdown 映射 |
|----------------|---------------------|
| Project | 含 `trackerRoot` 的本地工作区根 |
| Issue Tracker | 文件树 + 解析约定（+ 可选 `local-tracker.json`） |
| Issue | 一个 `NN-slug.md`（或 `map.md`/`spec.md` 若产品要展示） |
| Dependency | `Blocked by` 边 |
| Frontier | §3.4 查询结果 |
| 完成 | 终态 Status（主）/ legacy Closed（次） |
| Claim | `Status: claimed` |
| Comment | 文件内 `## Comments` 追加 |

### 7.4 风险与歧义（实现前需知）

1. **实施完成语义在 Matt 种子里弱于 Wayfinder**——以 `Status: resolved` 为 Adapter 写出合同最干净，读路径保留 legacy。  
2. **粗体字段 vs 普通字段、章节 vs 行内** 混用——解析必须双兼容。  
3. **编号冲突 / 重命名 slug**——id 跟文件名走；重命名即换 id，依赖引用会断除非同步改 Blocked by。  
4. **并发**——无服务端锁，仅靠 `claimed` 约定；Taskboard 多 Run 时需预期冲突。  
5. **map / implementation 混目录**——应用 `Type` 分流，避免把 research 当 `/implement` 候选。

---

## 8. 结论（直接回答票面问题）

| 维度 | 稳定约定（摘要） |
|------|------------------|
| **目录** | `.scratch/<feature>/{spec.md,map.md,issues/NN-slug.md}`；一票一文件；编号自 01 |
| **正文** | Header 字段（`Status`/`Type`/`Blocked by`/`What to build`）+ 章节（`Question`/`Answer`/`Comments`/map 五段）；无标准 YAML frontmatter |
| **状态** | 实施：triage 五角色；Wayfinder：空→`claimed`→`resolved`；完成真源宜统一为 `Status: resolved`（`wontfix` 终态；`Closed` 仅 legacy） |
| **Blocked by** | 正文边；编号/文件名引用；blocker 全终态则解阻；feature 内图 |
| **与 git** | Issue 即仓内文件；与 GitHub Issues 替代而非叠加；research 可用 `research/*` 分支存产物；`.scratch` 是否入库未强制 |
| **Adapter 目标** | §7.1 列表足够稳定，可作为 local markdown Tracker Adapter 的目标模型；§7.2 为明确非目标 |

**一句话：** Matt local markdown tracker 的稳定核心是「**按 feature 分目录的一票一 Markdown 文件 + `Status`/`Blocked by`（及 Wayfinder 的 `Type`/`Answer`/`map.md`）**」；Taskboard Adapter 应把文件树解析成与 GitHub 适配器同构的 Issue/Dependency/Frontier，而把 git 提交策略和 CLI 外壳留在产品层。

## 来源索引

- https://github.com/mattpocock/skills/blob/main/skills/engineering/setup-matt-pocock-skills/issue-tracker-local.md  
- https://github.com/mattpocock/skills/blob/main/skills/engineering/to-tickets/SKILL.md  
- https://github.com/mattpocock/skills/blob/main/skills/engineering/wayfinder/SKILL.md  
- https://github.com/mattpocock/skills/blob/main/skills/engineering/to-spec/SKILL.md  
- https://github.com/mattpocock/skills/blob/main/skills/engineering/setup-matt-pocock-skills/SKILL.md  
- https://github.com/mattpocock/skills/blob/main/CONTEXT.md  
- https://github.com/mattpocock/skills/issues/203  
- 本机：`yjx-local-tracker-setup` / `yjx-local-kanban`（`scripts/issue-board.mjs` 解析合同）
