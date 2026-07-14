---
title: "HANDOFF CONTRACT"
project: project
classification: high
created: 2026-06-06T03:40:25Z
updated: 2026-06-06T03:40:25Z
product_id: project
project_id: project
file_kind: HandoffContract
author: 
---

# Handoff Contract — project

## Session Handoff Requirements

Every session must update the following files before closing:

| File | Required Content |
|------|-----------------|
| `docs/state/STATUS.md` | Current status, what's done, what's in flight, what's next |
| `docs/state/HANDOFF.md` | Where we stopped, what's next, what to watch |
| `docs/state/CHANGELOG.md` | What changed this session |
| `docs/state/DECISIONS.md` | Any decisions made with context and rationale |
| `docs/state/OPEN_QUESTIONS.md` | Unresolved questions with blocking status |
| `docs/state/RISKS.md` | New or changed risks |

## Handoff Quality Criteria

A handoff is complete when the next session can start without asking "what happened?":

- [ ] Working tree is clean (all changes committed or stashed)
- [ ] All tests pass
- [ ] HANDOFF.md explains where work stopped and why
- [ ] HANDOFF.md lists the next logical step
- [ ] Any "gotchas" or surprises are documented in What to Watch
- [ ] New files or significant changes are mentioned in CHANGELOG.md
- [ ] If monorepo: every solution touched this session is listed with a one-line summary of what changed in it
- [ ] If cross-solution change: callers checked, side effects documented (R-27.2, R-27.4)

## Context Transfer

- Never assume the next session has context from this session
- Be specific: "I stopped mid-way through implementing the store trait for PluginResult" > "work in progress"
- Include file paths and function names, not just descriptions
- If a workaround was applied, document it and the intended permanent fix

## Emergency Handoff

If a session must end abruptly:

1. Commit or stash all changes
2. Write a minimal HANDOFF.md entry with what's in flight
3. Note any broken state (failing tests, partial implementation)
