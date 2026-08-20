# 调研：Antigravity CLI 的启动、TUI 与配置面

> 对应 wayfinder research ticket: [#43](https://github.com/youjiaxing/agent-taskboard/issues/43)  
> 目标：按已钉 Agent Adapter 合同（探测可执行文件 / 字段声明 / 拼 TUI argv / 说明能力），判断 Antigravity CLI 能否作为 v1 第四家内置 Agent 接入；支撑 [决策：v1 如何内置 Antigravity CLI](https://github.com/youjiaxing/agent-taskboard/issues/44) 与 [原型：开 Run 配置与游离 Run 入口](https://github.com/youjiaxing/agent-taskboard/issues/38)。  
> 对照格式：[#7 调研文稿](https://github.com/youjiaxing/agent-taskboard/blob/research/agent-cli-surface/docs/research/agent-cli-surface.md)。

## 证据基线（本机实测 + 官方文档）

| Agent | 本机路径 | 版本 | 主要官方文档 |
| --- | --- | --- | --- |
| Antigravity CLI | `~/.local/bin/agy`（Mach-O arm64 原生二进制） | `1.1.16` | [CLI Overview](https://antigravity.google/docs/cli/overview/)、[Install & Auth](https://antigravity.google/docs/cli/install)、[Headless mode](https://antigravity.google/docs/cli/headless)、[Modes](https://antigravity.google/docs/cli/modes)、[Permissions](https://antigravity.google/docs/cli/permissions)、[Sandbox](https://antigravity.google/docs/cli/sandbox)、[Conversations](https://antigravity.google/docs/cli/conversations)、[CLI Reference](https://antigravity.google/docs/cli/reference)、[Migrating from Gemini CLI](https://antigravity.google/docs/cli/gcli-migration) |
| Gemini CLI（遗留） | `~/.nvm/versions/node/v22.18.0/bin/gemini`（npm） | `0.21.3` | [Transition blog](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/) |

官方产品页与仓库：

- [Introducing Google Antigravity CLI（官方博客，2026-05-19）](https://antigravity.google/blog/introducing-google-antigravity-cli)
- [An important update: Transitioning Gemini CLI to Antigravity CLI（Google Developers Blog，2026-05-19）](https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli/)
- [github.com/google-antigravity/antigravity-cli](https://github.com/google-antigravity/antigravity-cli)（README / CHANGELOG / 社区 issue）

**证据优先级**：本机 `agy --help` / 子命令实际输出 > 官方文档 > 官方博客/仓库 > 社区。本票未启动交互 TUI（`agy` 无参数会进 TUI，禁止实测挂住）；TUI 行为以官方文档 + 本机 `--help` + 官方 changelog 为据。

---

## 总览对照（对齐 #7 的维度表）

| 维度 | Antigravity CLI |
| --- | --- |
| 可执行文件 | `agy` |
| 启动交互 TUI | `agy`（无参数）/ `agy -i "<prompt>"`（`--prompt-interactive` 带初始 prompt） |
| 非交互 / 脚本 | `-p` / `--print` / `--prompt`（headless；`--output-format text\|json\|stream-json`） |
| model 启动参数 | `--model <slug>`（`agy models` 列出；slug 形如 `gemini-3.7-flash-high`） |
| effort / 等价物 | `--effort <low\|medium\|high>`；effort 也**内嵌在模型 slug**（`-high/-medium/-low` 变体）；TUI `/effort` |
| 权限 / 审批 | `--mode <default\|accept-edits\|plan>`（执行模式，**没有** `--permission-mode`）；`--dangerously-skip-permissions`；settings `toolPermission` + `permissions.{allow,deny,ask}` 规则 |
| sandbox | `--sandbox`（Run 级）/ settings `enableTerminalSandbox`（持久）；Linux nsjail / macOS sandbox-exec / Windows AppContainer |
| worktree | **无官方 flag**；`/fork` 只克隆对话线程，不克隆 git checkout（官方文档明说隔离文件要靠 git branch/stash） |
| 配置文件 | `~/.gemini/antigravity-cli/settings.json`（JSON）+ `~/.gemini/config/`（mcp_config.json、hooks.json、projects/）+ `~/.gemini/antigravity-cli/keybindings.json` |
| 安装探测 | `which agy` + `agy --version`（本机 `1.1.16`） |
| 登录探测 | **无** `status`/`login` 子命令；登录发生在 TUI 启动时（keyring 静默 / 浏览器 / SSH URL 循环）；`/logout`；也可 `GEMINI_API_KEY` + `modelProvider:"gemini"` |
| doctor 含义 | **无** doctor 子命令 |

---

## 0. Antigravity CLI 是什么（半页内）

Antigravity CLI（可执行文件 `agy`）是 Google Antigravity 的终端面：官方定位为「轻量 TUI」([Overview](https://antigravity.google/docs/cli/overview/))，与桌面应用 **Antigravity 2.0**（GUI/IDE）共用同一 agent harness、共享 settings 与权限配置，会话可双向导入导出（CLI 会话可在 2.0 的 `@conversation` 下拉里调出）。它不是 Gemini CLI 的改名：2026-05-19 官方博客宣布将 Gemini CLI 统一进 Antigravity，Antigravity CLI 用 Go 重写（本机 `file` 确认为原生 Mach-O 二进制），保留了 Agent Skills、Hooks、Subagents、Extensions（现为 plugins）等核心概念，并提供 `agy plugin import gemini` 一次性迁移；2026-06-18 起 Gemini CLI 停止服务 consumer（Google AI Pro/Ultra/免费档），企业（Gemini Code Assist Standard/Enterprise）不受影响。

---

## 1. 如何启动交互式 TUI

```bash
cd <project>
agy                        # 无参数 → 交互 TUI（官方文档：Launch the TUI inside a project）
agy -i "<prompt>"          # --prompt-interactive：带初始 prompt 进入交互会话并继续
agy -c                     # --continue：续最近会话（交互 TUI）
agy --conversation <id>    # 按 ID resume 会话
agy --model <slug> --effort high   # 启动时钉 model/effort
agy --mode=plan            # 启动即 plan 模式
agy --sandbox              # 本会话开终端沙箱
```

官方确认 TUI 存在：Overview 首句「The Antigravity CLI is the lightweight Terminal User Interface (TUI) surface of Antigravity」；Getting Started 第 2 步「Launch the TUI inside a project … execute the launcher command: `agy`」；GitHub README「start Antigravity CLI by running: `agy`」。TUI 基于 Go 的 bubbletea（官方 changelog 多处提及 bubbletea v2 更新）。

**带初始 prompt 的正确 flag 是 `-i` / `--prompt-interactive`**（本机 `agy --help`：`Run an initial prompt interactively and continue the session`；与 Gemini CLI 的 `-i` 同源）。位置参数**不是** TUI 初始 prompt 入口——`--prompt` 是 `--print` 的别名（headless）。

**非 TUI 入口（对照，避免 Adapter 误用）**：

- `agy -p/--print/--prompt "<prompt>"`：headless 单轮，stdout 输出后退出；`--input-format stream-json` 可从 stdin 连续驱动多轮（同一会话）
- ACP：**无原生支持**（见 §7）

Taskboard v1 的 Embedded Terminal 应对齐**原生 TUI 路径**（`agy` / `agy -i "<prompt>"`），而不是 headless。

## 2. Run 级参数（`agy --help` 实测 + 官方 headless/modes 文档）

| 字段 | CLI | 说明 |
| --- | --- | --- |
| model | `--model <MODEL>` | 模型 slug（`agy models` 列出；changelog 1.1.12 起有稳定 slug）；未知 slug headless 硬失败（exit 非 0），交互会话回退+警告 |
| effort | `--effort <low\|medium\|high>` | 推理 effort；模型 slug 自带 effort 变体（如 `gemini-3.7-flash-high/medium/low`）；TUI `/effort` |
| 执行模式 | `--mode <accept-edits\|plan>` | 等价物是**执行模式**而非 permission-mode：`default`（写文件前 diff 审查）、`accept-edits`（自动批准文件编辑）、`plan`（先出计划）；Shift+Tab 会话内循环；持久键 `agentMode` |
| 权限 | `--dangerously-skip-permissions` | 自动批准所有工具（含 shell 命令）；settings `toolPermission` 另有 `request-review`（默认）/`proceed-in-sandbox`/`always-proceed`/`strict` 四档 |
| 细粒度权限 | settings `permissions.{allow,deny,ask}` | `action(target)` 规则，如 `command(git)`、`write_file(src/)`；优先级 Deny > Ask > Allow；TUI `/permissions` 管理 |
| sandbox | `--sandbox` | 本会话终端沙箱；持久键 `enableTerminalSandbox`（默认 false）；交互审批可单次「run in sandbox / run without sandbox」 |
| agent | `--agent <NAME>` | 会话 agent；`agy agent(s)` 列出（本机当前为空列表） |
| 会话 | `-c/--continue`、`--conversation <ID>`、`/resume`、`/fork` | 续最近 / 按 ID resume / 交互选择器 / fork 对话线程（**非 git**） |
| 项目 | `--project <ID>`、`--new-project` | 项目是会话组织单元（默认 `default-cli-project`），不是目录隔离 |
| 工作区 | `--add-dir <DIR>`（可重复）、`--project` | 附加工作区目录；会话按 cwd 限定 |
| headless 族 | `-p`、`--output-format`、`--input-format`、`--json-schema`、`--print-timeout`（默认 5m） | 非 TUI；`--input-format stream-json` 供宿主驱动长会话 |
| 其他 | `--log-file`、`--disable-slash-commands` | 日志覆盖 / 关掉 slash 展开（print 模式） |

**持久默认**：`~/.gemini/antigravity-cli/settings.json`（JSON，不是 TOML）。Reference 页列出的键与默认值：

```json
{
  "colorScheme": "terminal",          // light/dark/solarized/tokyo night/terminal…
  "altScreenMode": "default",         // default|always|never（altscreen vs 内联）
  "toolPermission": "request-review", // request-review|proceed-in-sandbox|always-proceed|strict
  "artifactReviewPolicy": "asks-for-review",
  "notifications": false,
  "showTips": true,
  "showFeedbackSurvey": true,
  "editor": "auto",
  "editorMode": "default",
  "vimInsertFirst": false,
  "allowNonWorkspaceAccess": false,
  "enableTerminalSandbox": false,
  "useG1Credits": false,
  "enableTelemetry": true,
  "verbosity": "high",
  "runningLightSpeed": "medium",
  "agentMode": "default",             // modes 文档：default|accept-edits|plan
  "modelProvider": "gemini",          // install 文档：仅 "gemini" 被接受
  "permissions": { "allow": [], "deny": [], "ask": [] }
}
```

环境变量：`GEMINI_API_KEY`（配合 `modelProvider:"gemini"` 直连 Gemini API）、`GOOGLE_GEMINI_BASE_URL`、`AGY_CLI_HIDE_ACCOUNT_INFO`、`AGY_CLI_CMD_OUTPUT_PERCENTAGE`、`AGY_CLI_DISABLE_LATEX`（官方 changelog）。

## 3. 安装 / 登录探测

| 探测目标 | 推荐方式 | 本机实测 |
| --- | --- | --- |
| 已安装 | `command -v agy` + `agy --version` | `1.1.16`（`~/.local/bin/agy`，Mach-O arm64） |
| 可用模型 | `agy models` | `gemini-3.7-flash-high` … `claude-opus-4-6-thinking`、`gpt-oss-120b-medium` 等；`--output-format json` 前置可拿 JSON 包（`{"status":"SUCCESS","response":"<tab 分隔 id\t名称>"}`） |
| 可用 agent | `agy agent` / `agy agents` | 本机空输出、exit 0 |
| 登录 | **无** `status` 子命令 | 登录在 TUI 启动时完成：OS keyring 静默 → 无凭据则自动开浏览器 → SSH 环境打印授权 URL + 授权码循环；`/logout` 清 keyring |
| 登录（无浏览器） | `GEMINI_API_KEY` + `modelProvider:"gemini"` | 跳过登录屏直进主界面；headless 未登录时 exit 报 `authentication required` 而不是挂住 |
| 安装位置 | macOS/Linux `~/.local/bin/agy`；Windows `C:\Users\<username>\AppData\Local\agy\bin` | 官方 install 文档 |

**Adapter 建议**：installed = `which` + `--version` 可解析；logged-in **不要探测**（按已钉合同不管登录态；且 CLI 无一等 status 接口）。注意本机 `~/.gemini/antigravity-cli/settings.json` 现为 `toolPermission: "always-proceed"`——表单默认值不要替用户假设。

## 4. 原生 git worktree / 隔离执行目录：**没有**

- `agy --help` 全部 flag 中**无任何 worktree 相关 flag**（对照 Grok `--worktree`、Claude `-w`）。
- 官方 [Conversations](https://antigravity.google/docs/cli/conversations) 文档明确：`/fork` 克隆的是**对话线程，不是本地 git checkout**——「Forking clones the *conversation thread*, not your local git checkout. To fully isolate files during parallel forks, use git branches or stash local changes」。即官方明示隔离文件要靠 git 本身。
- changelog 1.1.12 的 worktree 相关条目是**仓库根检测**（嵌套 repo / submodule / worktree 正确解析到根），不是创建 worktree 的能力。
- sandbox（`--sandbox`）是命令执行隔离（nsjail/sandbox-exec/AppContainer），不是工作目录隔离；沙箱内对 `.git` 只读（changelog 1.1.12/1.0.9）。

结论：**Antigravity CLI 无原生 worktree/隔离目录 flag**；v1 若需隔离，沿用 #18 已调研的外部方案（Taskboard 自己建 worktree 再 `agy` 进去）。

## 5. 登录/账号（记录事实，Adapter 不管）

- 官方 [Install & Auth](https://antigravity.google/docs/cli/install)：本机 keyring 静默登录；无会话则自动开浏览器；SSH 远程环境走「授权 URL + 手动粘贴授权码」循环。
- `/logout`：清 keyring 凭据。
- `GEMINI_API_KEY` + `modelProvider:"gemini"`（settings.json）：直连 Gemini API，不建账号会话，适合 CI；`/logout` 对其无效。
- 企业：GCP 项目绑定（enterprise 文档），Application Default Credentials 支持。
- 本机数据：`~/.gemini/antigravity-cli/`（settings.json、keybindings.json、conversations/*.db、cache/、log/、skills/、plugins/）；共享配置 `~/.gemini/config/`（mcp_config.json、hooks.json、projects/）；工作区规则 `GEMINI.md`/`AGENTS.md`；工作区技能 `.agents/skills/`、工作区 MCP `.agents/mcp_config.json`（gcli-migration 文档）。
- 登录探测若未来要做：无官方 status 命令；只能启发式（`agy models` 成功 / keyring 有 token / `GEMINI_API_KEY` 非空）。**不要把登录写成启动门槛。**

## 6. 完成信号 / token 用量可观察物（只列事实）

- **headless JSON envelope**（官方 [Headless](https://antigravity.google/docs/cli/headless) 文档）：`--output-format json` 结束输出单对象，字段 `conversation_id`、`status`、`response`、`error`、`duration_seconds`、`num_turns`、`usage{input_tokens,output_tokens,thinking_tokens,cache_read_tokens,total_tokens}`；`status` 枚举 `SUCCESS|ERROR|CANCELED|INTERRUPTED|INVALID|WAITING|RUNNING`。
- **stream-json 事件流**：`init`（含 cwd/tools/permission_mode/model/agent）→ `step_update`（`state: ACTIVE|DONE`、`step_type`、`text_delta`、`tool_info`、`subagent_info`、**逐 step `usage`**）→ `result`（终态，同 json envelope）。可当完成信号 + 用量观察物；`--input-format stream-json` 可保持同一会话多轮。
- **TUI 内**：`/usage`（别名 `/quota`）面板显示各模型配额/用量；`/credits` 显示 G1 credits；statusline 可显示 quota 与执行模式；`notifications` 设置可在任务完成时发系统通知；statusline 支持自定义脚本管道（官方 features 文档：可拿到 cwd、active model、token usage、state 的 JSON）。
- 官方**没有**名为 SessionEnd 的协议事件；「终态 + 用量」在 headless 路径有结构化出口，TUI 路径只有面板/statusline。

## 7. 是否原生讲 ACP：**无**

- `agy` 无 ACP flag（本机 `--help` 全量 flag 无 ACP 项；官方文档无 ACP 页）。
- 官方仓库有开放 feature request：[google-antigravity/antigravity-cli#31 "Feature request: add ACP (Agent Client Protocol) stdio JSON-RPC mode"](https://github.com/google-antigravity/antigravity-cli/issues/31)（open，2026-05-20）。
- 旧 `gemini` 0.21.3 有 `--experimental-acp`（本机实测），agy 未继承。
- 社区有第三方包装器（如 npm `agy-acp`、`antigravity-acp` 等，包装 `agy` + 轮询本地 SQLite 会话库转 ACP 事件；官方 ToS 对第三方接入有风险声明）——只列存在，不评协议。
- 本机另有**隐藏子命令** `agy agentapi`（`new-conversation` / `send-message` / `get-conversation-metadata`，`~/.gemini/antigravity-cli/bin/agentapi` 包装脚本）——内部 IDE 桥接面，非公开 ACP。

## 8. 和 Gemini CLI 的关系（Taskboard 应不应把 `gemini` 当另一家）

- 官方博客（2026-05-19）：Gemini CLI 的能力并入 Antigravity CLI（同一 agent harness 的另一面）；2026-06-18 起 Gemini CLI 停止服务 consumer（Google AI Pro/Ultra/免费档与 Gemini Code Assist for individuals）；企业（Gemini Code Assist Standard/Enterprise）与付费 API key 继续可用。
- 本机 `gemini` 0.21.3（npm）仍在，`--experimental-acp` 等旧 flag 可跑；`agy plugin import gemini` 提供迁移。
- **结论：`gemini` 与 `agy` 是同一家（Google）同一产品线的前后两代，不算另一家 Agent**；Taskboard 探测到 `gemini` 时应提示迁移到 `agy`，不要把它当独立 Agent 接入。

## 9. TUI 内专属 / 不宜仅靠启动参数声明

| 能力 | TUI 入口 | 启动参数能否等价 |
| --- | --- | --- |
| 中途换 model / effort | `/model`（effort timeline 选择器）、`/effort` | `--model`/`--effort` 只定初始值 |
| 执行模式循环 | `Shift+Tab`（default → accept-edits → plan） | `--mode` 只定初始；会话中切换无 flag |
| 权限规则管理 | `/permissions` | settings 持久化；Run 级只有 `--dangerously-skip-permissions` |
| 会话选择 / fork | `/resume`（picker）、`/fork` | `-c`/`--conversation` 部分覆盖；picker 体验在 TUI |
| 后台任务 / 子代理 | `/tasks`、`/agents`、`alt+j` 传送、`ctrl+k` 快批 | 会话内交互 |
| diff 审查 / artifact | `/diff`、`/artifact`、`f`、`ctrl+r` | `--mode=accept-edits` 可跳过审查，但审查体验在 TUI |
| settings / keybindings | `/settings`（`/config`）、`/keybindings` | 持久配置，非 Run argv |
| 其他 | `/usage`、`/credits`、`/statusline`、`/hooks`、`/mcp`、`/skills`、`/btw`、`/rewind`、`/context`、`/title`、`/plan` | 会话内 |

## 10. Adapter 字段草稿（Antigravity CLI；只列事实候选，不拍进 v1 表单）

```text
binary: agy
launch_interactive: ["agy", "-i", initialPrompt, ...flags]   # 或裸 "agy"
detect_installed: which agy + agy --version                  # 本机 1.1.16
detect_logged_in: 无官方 status；按合同不管登录态
declared_run_fields:
  - model          (--model; slug 来源: agy models)
  - effort         (--effort low|medium|high; 也可随 model slug 表达)
  - mode           (--mode accept-edits|plan; 注意不是 --permission-mode)
  - dangerouslySkipPermissions (bool → --dangerously-skip-permissions)
  - sandbox        (bool → --sandbox)
  - agent          (--agent; agy agents 列出)
  - continueLast   (bool → -c)
  - resume         (--conversation <id>)
  - addDirs        (--add-dir 可重复)
  - project        (--project <id> / --new-project)
  - initialPrompt  (-i; Run 绑 Issue 的注入面)
config_defaults_file: ~/.gemini/antigravity-cli/settings.json (JSON)
notes:
  - 无 --permission-mode / 无 --effort 之外的 effort 枚举漂移风险（help 写死 low|medium|high）
  - 无原生 worktree flag；/fork 只 fork 对话
  - 会话数据在 ~/.gemini/antigravity-cli/conversations/*.db (SQLite)，退出时 CLI 打印 resume 命令
  - 桌面 IDE（Antigravity 2.0）同 harness 同 settings，但那是另一个产品面，不参与本 Adapter
```

## 11. 结论（回答 ticket 问题）

1. **官方交互 TUI：有。** `agy` 无参数即进 TUI（官方 Getting Started「Launch the TUI」；Overview 自称 TUI surface）；带初始 prompt 用 `-i/--prompt-interactive`（`--help` 实测）；`-p/--print` 是 headless 另一入口；ACP 无原生；桌面应用是另一产品面。
2. **可执行文件名：`agy`**。macOS/Linux 默认 `~/.local/bin/agy`，Windows `C:\Users\<username>\AppData\Local\agy\bin`；`--version` → `1.1.16`，`--help` exit 0。
3. **Run 级参数**：model `--model`；effort `--effort low|medium|high`（且内嵌模型 slug）；权限/审批走 `--mode`（执行模式）+ `--dangerously-skip-permissions` + settings `toolPermission`/`permissions`；sandbox `--sandbox`；会话 `-c`/`--conversation`；另有 `--agent`、`--project/--new-project`、`--add-dir`。
4. **原生 worktree：没有**。官方 `/fork` 只克隆对话线程，文档明示隔离文件用 git branch/stash。
5. **登录**：keyring 静默/浏览器/SSH URL 循环，`/logout` 清除；无 status 子命令；`GEMINI_API_KEY` 可免登录。Adapter 不管登录态。
6. **Gemini CLI**：被 Antigravity CLI 取代（consumer 2026-06-18 停服；企业继续）；`gemini` npm 旧版仍在本机，**不算另一家 Agent**。
7. **完成信号/用量**：headless JSON/stream-json 有终态 `status` 枚举 + 结构化 `usage`（含 cache_read_tokens）；TUI 有 `/usage` `/quota` `/credits` 与 statusline；无 SessionEnd 命名事件。
8. **ACP：无原生**（官方 issue #31 开放中；旧 gemini 有 `--experimental-acp`；社区第三方包装器存在）。

**算不算 Agent（按已钉定义）：算。** Antigravity CLI 有官方交互 TUI，能在 Embedded Terminal 里以官方 TUI 运行（`agy` / `agy -i "<prompt>"`），不是只有 Web UI 的形态；登录态/API key 与看板无关。作为 v1 第四家内置 Agent 接入的探测、字段声明、argv 拼装均成立，且不与 #7 三家假统一（flag 命名明显不同：`--mode` 而非 `--permission-mode`，effort 枚举为 low|medium|high，无 worktree flag）。

> 本票只落事实；不写 ADR，不替 [决策：v1 如何内置 Antigravity CLI](https://github.com/youjiaxing/agent-taskboard/issues/44) 拍板（含优先级）。
