# 3DMK Tool Execution Router Design

## Goal

Provide one developer-facing command that selects a verified local runner for a named 3DMK tool task and refuses to guess when no compatible runner exists.

## Boundary

This is developer infrastructure only. It is not shipped with the Tauri application, does not process project geometry, does not select product compute backends, and does not alter the authoritative Rust/Tauri/CUDA runtime.

## Decision

The router is a small standalone Rust 2021 CLI under `tools/3dmk-exec-router`. It reports structured JSON, probes only local executables, and starts a command only after the selected runner is found.

## Profiles

| Profile | Preferred runner | Fallback | Required capability |
|---|---|---|---|
| `toolkit-check` | Git Bash `bash` | none | `scripts/validate-3dmk-toolkit.sh` exists and `bash` resolves |
| `windows-toolkit-check` | PowerShell 7 `pwsh` | Windows PowerShell `powershell.exe` | Windows host and `scripts/validate-3dmk-toolkit.ps1` exists |
| `rust-check` | `cargo` | none | `cargo` resolves |
| `cuda-probe` | `nvidia-smi` | none | `nvidia-smi` resolves |
| `mistralrs-agent` | PowerShell 7 | Windows PowerShell | Windows host, PowerShell, and `scripts/run-mistralrs-vwm-task.ps1` |

## Contract

`3dmk-exec-router probe <profile>` emits one JSON record to stdout and returns:

- `0`: profile is runnable;
- `2`: unknown profile or invalid arguments;
- `3`: known profile is blocked.

`3dmk-exec-router run <profile> [-- <args>]` probes first, then executes the selected process with inherited stdout/stderr. It never invokes a shell string; commands and arguments are passed as separate process arguments.

## Selection rules

1. Select only the profile’s declared runner.
2. Check executable discovery before every run.
3. Require Windows for PowerShell-only profiles.
4. Prefer `pwsh` over `powershell.exe`.
5. Treat Git Bash as `bash` discovered on `PATH`; Windows Git Bash qualifies.
6. Return a machine-readable block reason, never silently substitute another tool.
7. Do not create, mutate, or install tools.

## Tests

Unit tests cover profile lookup, runner selection, blocking reasons, argument parsing, and JSON escaping. The Git Bash validator mirrors the existing PowerShell toolkit validator’s content checks.

## Constraints

- Rust 2021, standard library only; no network, cloud, Docker, Podman, WSL, Electron, or Python backend.
- Windows-first behavior with Git Bash accepted for portable shell validation.
- CUDA remains a separately verified hardware capability, not a generic fallback.
