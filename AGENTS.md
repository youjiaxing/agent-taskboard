## Agent skills

### Issue tracker

Issues live as GitHub Issues (`gh`). See `docs/agents/issue-tracker.md`.

A change the user asks for directly in conversation is not a ticket. Implement it and deliver it the way the user asks; never open an issue on your own initiative just to give a change a number.

### Wrap-up

Successful `/implement` of an implementation ticket 收尾 automatically in the same turn. Also 收尾 / 关票. Deliver through a pull request: commit on a branch, `gh pr create`, then merge, so a body line `Closes #<n>` closes the ticket. Do not `gh issue close` first. Steps: `docs/agents/issue-tracker.md` (**收尾关票**).

### Branch naming

This repository supports multiple Agent implementations. Use `agent/<agent-id>/<task-slug>` for implementation branches, such as `agent/codex/launch-field-help`; do not assume that Codex is the only Agent. If the checkout is detached, create this branch before editing and then follow the delivery flow in `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
