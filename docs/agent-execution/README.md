# OpenCode VWM Execution Harness

This directory records resumable execution state for the authoritative VWM implementation plan.

## Start on the Windows laptop

From the repository root:

```powershell
.\scripts\start-opencode-vwm.ps1 -Mode Start
```

Resume the most recent OpenCode session:

```powershell
.\scripts\start-opencode-vwm.ps1 -Mode Continue -AllowDirty
```

Run non-interactively:

```powershell
.\scripts\start-opencode-vwm.ps1 -Mode Run
```

## Run through GitHub Actions

The repository workflow is:

```text
.github/workflows/opencode-big-pickle.yml
```

It runs the committed `vwm-executor` agent with:

```text
model: opencode/big-pickle
share: false
```

### Required repository secret

In **Settings → Secrets and variables → Actions**, add:

```text
OPENCODE_API_KEY
```

The value must be an OpenCode Zen API key with access to Big Pickle. The workflow fails closed when the secret is absent; it does not select another provider or model.

### Trigger from an issue or pull request

Only comments from the repository owner, members, or collaborators are accepted. Add a comment containing `/opencode` or `/oc`, for example:

```text
/opencode Execute the next dependency-ready task in the authoritative VWM plan. Follow AGENTS.md, use TDD, invoke vwm-reviewer and vwm-verifier, update VWM_PROGRESS.md with command evidence, and open or update a pull request.
```

### Trigger manually

Open **Actions → OpenCode Big Pickle → Run workflow** and provide a prompt. The default prompt continues from the first dependency-ready incomplete task.

The GitHub workflow uses the repository `GITHUB_TOKEN` with write access to contents, issues, and pull requests. It may create implementation branches and pull requests, but the repository OpenCode policy still blocks direct work on `main`, force pushes, merges, destructive resets, and secret reads.

GitHub-hosted execution runs on Ubuntu because that is the supported OpenCode GitHub Action path. Windows-specific acceptance gates remain mandatory and must run through the Windows laptop harness or a separately approved Windows runner before `PROJECT_COMPLETE` can be reported.

## What the launcher enforces

- OpenCode, Git, Rust, Cargo, and PowerShell are available.
- The authoritative plan and agent contract exist.
- The live OpenCode model catalog contains Big Pickle.
- Work occurs on `agent/vwm-authoritative-revision-implementation`, not `main`.
- The OpenCode session uses the `vwm-executor` agent and Big Pickle explicitly.
- Repository permissions deny direct main/master pushes, force pushes, destructive resets, Docker, Podman, WSL, and secret reads.

## Execution control

Use `/implement-vwm` inside OpenCode to start or resume the workflow.

The progress ledger is `VWM_PROGRESS.md`. It must be updated only with actual command output and commit evidence.

## Privacy

OpenCode Zen documents Big Pickle as a free stealth model available for a limited period. During the free period, submitted data may be used to improve the model. Do not place credentials, personal data, proprietary third-party material, or secrets in prompts or repository files.
