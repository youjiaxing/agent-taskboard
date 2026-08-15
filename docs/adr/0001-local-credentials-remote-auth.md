# 本机凭据与远端鉴权

Agent Taskboard 是个人本地工具，不引入产品账号。远端 Issue Tracker（GitHub / 未来 GitLab）的访问采用**混合凭据**：应用进程内直调 REST/GraphQL，token 按固定顺序解析；local markdown 无远端鉴权。未就绪时做**项目级降级**，并并列提供 CLI / 钥匙串 PAT / 环境变量三条修复路径。

## 凭据解析顺序（按 host 分桶）

1. **应用专用环境变量**（避免污染其它工具对通用 token 的依赖）  
   - GitHub：`AGENT_TASKBOARD_GITHUB_TOKEN`  
   - GitLab：`AGENT_TASKBOARD_GITLAB_TOKEN`
2. **通用环境变量（生态惯例）**  
   - GitHub：`GH_TOKEN`，否则 `GITHUB_TOKEN`  
   - GitLab：`GITLAB_TOKEN`，否则 `GL_TOKEN`
3. **本机 CLI**（日常主路径）  
   - GitHub：`gh auth token`（可带 `--hostname`）  
   - GitLab：`glab` 对等能力
4. **OS 钥匙串中的应用级 PAT**（按 host 分条；面向未装 CLI 的用户）

v1 **不**在 Project 配置中绑定或存储 token。env 在 v1 视为该 Tracker 类型**默认公网 host** 的覆盖；企业/self-hosted 以 CLI 或该 host 钥匙串为主。Project 仍保存 host + 定位信息（如 `owner/repo`）。

## 失败体验

- 单个 Project 远端未就绪时降级提示，不锁死整个应用。
- 修复入口三者并列，顶部用探测结果做情境提示（例如未检测到 `gh`、401/403）。
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

## Consequences

- Tracker Adapter 必须实现统一的「解析 token → 直调 API → 结构化鉴权错误」边界；CLI 是凭据源而非唯一总线。
- 设置/未就绪 UI 必须同时讲清三条路径，不能只引导 CLI。
- 后续若支持 per-project 凭据或 env 多 host 后缀，属于显式扩展，需新决策。
