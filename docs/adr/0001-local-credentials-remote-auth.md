# 本机凭据与远端鉴权

Agent Taskboard 是个人本地工具，不引入产品账号。远端 Issue Tracker（GitHub / 未来 GitLab）的访问采用**混合凭据**：应用进程内直调 REST/GraphQL，token 按固定顺序解析；local markdown 无远端鉴权。未就绪时做**项目级降级**，并并列提供 CLI / 编辑 Host 秘密文件 / 环境变量三条修复路径。

存储介质与顺序的第四来源已由 [设置与数据存放位置](./0012-settings-and-data-location.md) 改写：不用 OS 钥匙串，PAT 写在 Host 数据里的 JSON 秘密文件；该文件里**显式写了**才算专用覆盖，否则继续走通用（先 `gh`/`glab`，再通用环境变量）。

## 凭据解析顺序（按 host 分桶）

1. **应用专用环境变量**（本次会话的最高覆盖，不改别人的环境）  
   - GitHub：`AGENT_TASKBOARD_GITHUB_TOKEN`  
   - GitLab：`AGENT_TASKBOARD_GITLAB_TOKEN`
2. **Host 秘密文件里该 host 显式写入的 PAT**（长期覆盖，JSON，仅当前用户可读）
3. **本机 CLI**（日常默认）  
   - GitHub：`gh auth token`（可带 `--hostname`）  
   - GitLab：`glab` 对等能力
4. **通用环境变量**（没装 CLI、只在环境里有 token 时）  
   - GitHub：`GH_TOKEN`，否则 `GITHUB_TOKEN`  
   - GitLab：`GITLAB_TOKEN`，否则 `GL_TOKEN`

v1 **不**在 Project 配置或仓库里绑定/存储 token。应用专用 env 与秘密文件都是 Taskboard 自己的覆盖，不动 `GH_TOKEN` 等别人的环境。企业/self-hosted 以 CLI 或该 host 在秘密文件里的 PAT 为主。Project 仍保存 host + 定位信息（如 `owner/repo`）。

## 失败体验

- 单个 Project 远端未就绪时降级提示，不锁死整个应用。
- 修复入口三者并列（CLI / 编辑秘密文件 / 环境变量），顶部用探测结果做情境提示（例如未检测到 `gh`、401/403）。
- 权限不足时给出可读错误，并指向建议 scope/权限表（具体列表由各 Tracker Adapter 规格填充）。

## 明确不做

- 产品级 OAuth / 账号系统 / 云同步凭据
- 强制 GitHub App 安装作为唯一登录方式
- 将 Agent CLI（Grok / Codex / Claude Code）登录与 Tracker 鉴权绑成同一套流程

## Considered options（摘要）

| 选项 | 未采纳原因 |
|------|------------|
| 仅复用 CLI | 许多人未安装 `gh`/`glab`，GitHub 项目会直接不可用 |
| 仅应用自管 PAT | 与已有 CLI 工作流脱节，双份 token |
| 应用自建 Device OAuth | 产品化运维成本，违背无账号立场 |
| 全部 shell 出 `gh` 做 API | 解析与并发脆弱，能力受 CLI 暴露面限制 |
| 全局阻断直至登录 | 伤害多 Project / local markdown 并行使用 |
| OS 钥匙串存 PAT | 搬家要对齐钥匙串；浏览器没有；用户更熟本地文件 |
| 通用环境变量排在 `gh` 前面 | 个人电脑上的 `GH_TOKEN` 常是别的工具留下的，会误伤日常 `gh` 登录 |

## Consequences

- Tracker Adapter 必须实现统一的「解析 token → 直调 API → 结构化鉴权错误」边界；CLI 是凭据源而非唯一总线。
- 设置/未就绪 UI 必须同时讲清三条路径，不能只引导 CLI；PAT 入口是编辑 Host 秘密文件，不是钥匙串。
- 后续若支持 per-project 凭据或 env 多 host 后缀，属于显式扩展，需新决策。
