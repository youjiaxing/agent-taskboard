## Agent skills

### Issue tracker

Issues live as GitHub Issues (`gh`). See `docs/agents/issue-tracker.md`.

### Wrap-up

Successful `/implement` of an implementation ticket 收尾 automatically in the same turn. Also 收尾 / 关票. Merge a PR whose body contains `Closes #<n>`. Do not `gh issue close` first. Steps: `docs/agents/issue-tracker.md` (**收尾关票**).

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
