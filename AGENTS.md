## Agent skills

### Issue tracker

Issues live as GitHub Issues (`gh`). See `docs/agents/issue-tracker.md`.

A change the user asks for directly in conversation is not a ticket. Implement it and deliver it the way the user asks; never open an issue on your own initiative just to give a change a number.

### Wrap-up

Successful `/implement` of an implementation ticket 收尾 automatically in the same turn. Also 收尾 / 关票. Deliver by committing and pushing on the current branch; do not open a PR unless the user asks. Steps: `docs/agents/issue-tracker.md` (**收尾关票**).

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`). See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: root `CONTEXT.md` + `docs/adr/`. See `docs/agents/domain.md`.
