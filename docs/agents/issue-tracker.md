# Issue tracker: GitHub

Issues and specs for this repo live as GitHub issues. Use the `gh` CLI for all operations.

## Conventions

- **Create an issue**: `gh issue create --title "..." --body "..."`. Use a heredoc for multi-line bodies.
- **Read an issue**: `gh issue view <number> --comments`, filtering comments by `jq` and also fetching labels.
- **List issues**: `gh issue list --state open --json number,title,body,labels,comments --jq '[.[] | {number, title, body, labels: [.labels[].name], comments: [.comments[].body]}]'` with appropriate `--label` and `--state` filters.
- **Comment on an issue**: `gh issue comment <number> --body "..."`
- **Apply / remove labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: implementation tickets follow **收尾关票** below; Wayfinder Resolve and triage use `gh issue close <number> --comment "..."`.

Infer the repo from `git remote -v` — `gh` does this automatically when run inside a clone.

## Scope

Issues track work this repo has already chosen to track. A request the user makes directly in conversation is not one: implement it, and deliver it the way the user asks (commit, branch, or PR). Never open an issue on your own initiative to give a change a number. **收尾关票** below applies only when an implementation ticket already exists.

## 收尾关票

After a successful `/implement` of an implementation ticket, run these steps in the same turn. Successful means tests are green and `HEAD` is the implementation commit. Also run them when the user says 收尾 / 关票.

1. Commit the task changes on the current branch (usually `main`) with a Conventional Commit message that names the ticket in the subject (`fix(launch): 修复启动表选 Agent 被覆盖且 model 无法点选 (#126)`) and carries a `Closes #<n>` line in the body. Done when `git status` is clean and `HEAD` is the implementation commit.
2. `git push`. Done when the remote branch is at that commit.
3. Confirm the ticket closed: `gh issue view <n>` reports `CLOSED`. GitHub closes it when the commit carrying `Closes #<n>` lands on the default branch; if it is still open (keyword missing), close it explicitly with `gh issue close <n> --comment "<what landed>"`.

A PR is opt-in: open one only when the user asks for it. Then replace step 3 with `gh pr create` (title equal to the commit title, body with `## 摘要`, a `Closes #<n>` line, `## 范围`, and `明确不做` for deferred tickets) followed by `gh pr merge <pr> --merge --delete-branch`; `Closes #<n>` closes the issue on merge. If merge is blocked, stop and report.

Wayfinder children still **Resolve**: comment, `gh issue close`, then a pointer on the map.

## Pull requests as a triage surface

**PRs as a request surface: no.** _(Set to `yes` if this repo treats external PRs as feature requests; `/triage` reads this flag.)_

When set to `yes`, PRs run through the same labels and states as issues, using the `gh pr` equivalents:

- **Read a PR**: `gh pr view <number> --comments` and `gh pr diff <number>` for the diff.
- **List external PRs for triage**: `gh pr list --state open --json number,title,body,labels,author,authorAssociation,comments` then keep only `authorAssociation` of `CONTRIBUTOR`, `FIRST_TIME_CONTRIBUTOR`, or `NONE` (drop `OWNER`/`MEMBER`/`COLLABORATOR`).
- **Comment / label / close**: `gh pr comment`, `gh pr edit --add-label`/`--remove-label`, `gh pr close`.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either — resolve with `gh pr view 42` and fall back to `gh issue view 42`.

## When a skill says "publish to the issue tracker"

Create a GitHub issue.

## When a skill says "fetch the relevant ticket"

Run `gh issue view <number> --comments`.

## Wayfinding operations

Used by `/wayfinder`. The **map** is a single issue with **child** issues as tickets.

- **Map**: a single issue labelled `wayfinder:map`, holding the Notes / Decisions-so-far / Fog body. `gh issue create --label wayfinder:map`.
- **Child ticket**: an issue linked to the map as a GitHub sub-issue (`gh api` on the sub-issues endpoint). Where sub-issues aren't enabled, add the child to a task list in the map body and put `Part of #<map>` at the top of the child body. Labels: `wayfinder:<type>` (`research`/`prototype`/`grilling`/`task`). Once claimed, the ticket is assigned to the driving dev.
- **Blocking**: GitHub's **native issue dependencies** — the canonical, UI-visible representation. Add an edge with `gh api --method POST repos/<owner>/<repo>/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-db-id>`, where `<blocker-db-id>` is the blocker's numeric **database id** (`gh api repos/<owner>/<repo>/issues/<n> --jq .id`, _not_ the `#number` or `node_id`). GitHub reports `issue_dependencies_summary.blocked_by` (open blockers only — the live gate). Where dependencies aren't available, fall back to a `Blocked by: #<n>, #<n>` line at the top of the child body. A ticket is unblocked when every blocker is closed.
- **Frontier query**: list the map's open children (`gh issue list --state open`, scoped to the map's sub-issues / task list), drop any with an open blocker (`issue_dependencies_summary.blocked_by > 0`, or an open issue in the `Blocked by` line) or an assignee; first in map order wins.
- **Claim**: `gh issue edit <n> --add-assignee @me` — the session's first write.
- **Resolve**: `gh issue comment <n> --body "<answer>"`, then `gh issue close <n>`, then append a context pointer (gist + link) to the map's Decisions-so-far.
