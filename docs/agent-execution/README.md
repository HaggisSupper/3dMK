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
