# OpenCode GitHub Delegation Implementation Plan

> For agentic workers: use the repository's approved implementation and review gates. Steps use checkbox syntax for tracking.

Goal: Add a bounded, owner-gated OpenCode GitHub Actions delegation lane to 3dMK without replacing the authoritative local Mistral.rs executor.

Architecture: A comment-triggered GitHub Actions workflow runs OpenCode on an ephemeral GitHub-hosted runner. It writes only to a branch and pull request; repository governance, review, CI, and merge remain authoritative. Cloud inference is isolated to this optional delegation lane.

Tech Stack: GitHub Actions, OpenCode GitHub action, OpenRouter, Markdown governance documentation.

Spec: docs/agent-execution/OPENCODE_GITHUB_AGENT.md

## Global Constraints

- Windows-first product execution remains authoritative.
- Local CUDA-backed Mistral.rs remains the primary executor.
- No Docker, Podman, WSL, Electron, or Python production backend.
- The delegated workflow is owner-triggered, bounded, review-gated, and cannot publish directly to main.
- Secrets are read only from GitHub Actions secrets.
- Every implementation task follows admit → stage → execute → validate → publish → finalize.

---

### Task 1: Add the delegated workflow

Files:
- Create: .github/workflows/opencode.yml
- Test: GitHub Actions workflow syntax and owner-trigger contract review

Interfaces:
- Consumes: OPENROUTER_API_KEY, OPENCODE_MODEL, GITHUB_TOKEN
- Produces: owner-triggered branch, commit, and pull-request delegation

- [ ] Add the workflow with issue_comment and pull_request_review_comment triggers.
- [ ] Gate execution to repository-owner comments containing /opencode or /oc.
- [ ] Set a 60-minute timeout, scoped write permissions, and per-issue or pull-request concurrency.
- [ ] Validate YAML structure and inspect rendered workflow permissions before opening the pull request.

### Task 2: Document operator use and boundaries

Files:
- Create: docs/agent-execution/OPENCODE_GITHUB_AGENT.md
- Modify: AGENTS.md
- Test: Documentation validation workflow and manual contract review

Interfaces:
- Consumes: Existing local Mistral.rs authority and transaction rules in AGENTS.md
- Produces: Explicit optional delegated-executor contract

- [ ] Document required repository secret and variable setup.
- [ ] Document invocation, supervision, rollback, and the local-endpoint limitation.
- [ ] Clarify in AGENTS.md that OpenCode is optional and delegated only; local Mistral.rs remains authoritative.
- [ ] Run documentation validation and verify that no active-executor or CUDA-first requirement is weakened.

### Task 3: Verify activation prerequisites

Files:
- No source changes.

Checks:
- [ ] Add OPENROUTER_API_KEY as a repository secret.
- [ ] Add OPENCODE_MODEL as a repository variable.
- [ ] Comment /opencode inspect the repository and report the task plan without changing files on a test issue.
- [ ] Confirm the workflow starts only for the owner, completes within the timeout, and leaves no direct main mutation.
