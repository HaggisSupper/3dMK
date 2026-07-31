# OpenCode VWM Execution Harness Design

**Date:** 2026-07-31  
**Repository:** `HaggisSupper/3dMK`  
**Authoritative implementation plan:** `docs/superpowers/plans/2026-07-31-vwm-authoritative-revision-workflow.md`

## Objective

Configure the 3DMk repository so OpenCode can execute the authoritative VWM revision workflow with the OpenCode Zen Big Pickle model, resume across sessions, preserve scope, and refuse premature completion claims.

## Selected approach

Use a repository-native orchestration harness:

- root `AGENTS.md` for mandatory project rules;
- root `opencode.json` for project instructions and bounded permissions;
- a primary `vwm-executor` agent;
- read-only `vwm-reviewer` and `vwm-verifier` subagents;
- a reusable `/implement-vwm` command;
- a persistent progress ledger;
- a Windows PowerShell launcher that resolves the exact Big Pickle model ID from `opencode models opencode --refresh`.

The model is selected by the launcher rather than hard-coded into agent files. This prevents stale model catalog identifiers from silently selecting a different model.

## Execution lifecycle

```text
preflight
→ resolve Big Pickle model ID
→ verify clean implementation branch
→ read AGENTS + authoritative plan + progress ledger
→ execute one task using TDD
→ independent review
→ independent verification
→ repair findings
→ update evidence ledger
→ commit
→ continue or stop at a clean session boundary
```

## Authority boundaries

- The implementation plan defines scope and dependency order.
- `AGENTS.md` defines repository-wide operating constraints.
- The progress ledger records evidence; it does not override the plan.
- The executor may edit and test repository files.
- The reviewer and verifier may not edit.
- Direct pushes to `main` or `master`, force pushes, destructive resets, Docker, Podman, and WSL are denied.
- Big Pickle may propose changes but cannot redefine the Definition of Done.

## Completion semantics

The harness distinguishes:

- **task complete** — every task step and acceptance command passes;
- **batch complete** — the plan's first execution batch gate passes;
- **session boundary** — work is committed and resumable, but the plan is incomplete;
- **blocked** — an external credential, missing licensed model, or reproducible architecture blocker prevents progress;
- **project complete** — all 17 tasks and the plan's Definition of Done are evidenced.

The executor must never use “done,” “complete,” or a percentage for the full project unless the Task 17 evidence exists.

## Failure handling

- A failing test is investigated before implementation changes.
- Three failed fix attempts on the same root cause trigger an architecture review.
- Missing credentials or licensed model assets are logged as explicit blockers.
- A context limit produces a clean session boundary, not a completion claim.
- Uncommitted work is preserved; destructive cleanup is prohibited.

## Privacy note

Big Pickle is a free stealth model on OpenCode Zen. During the free period, submitted data may be used to improve the model. The launcher prints this warning. Secrets and `.env` files are denied by repository permissions.
