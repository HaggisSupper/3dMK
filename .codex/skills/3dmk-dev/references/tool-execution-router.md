# Tool Execution Router

The developer-only router lives at `tools/3dmk-exec-router`.

## Use

From the repository root:

```text
cargo run --manifest-path tools/3dmk-exec-router/Cargo.toml -- probe toolkit-check
cargo run --manifest-path tools/3dmk-exec-router/Cargo.toml -- run toolkit-check
```

Always probe before running. Its JSON result is authoritative for runner availability.

## Profiles

| Profile | Runner choice | Purpose |
|---|---|---|
| `toolkit-check` | Git Bash `bash` | Portable toolkit validation; Git for Windows qualifies. |
| `windows-toolkit-check` | `pwsh`, then `powershell.exe` | Windows-native validation. |
| `rust-check` | `cargo` | Router crate tests. |
| `cuda-probe` | `nvidia-smi` | GPU/driver evidence. |
| `mistralrs-agent` | `pwsh`, then `powershell.exe` | Local Mistral.rs agent task entrypoint. |

## Semantics

- Exit `0`: runnable probe or successful run.
- Exit `2`: unknown profile or invalid router arguments.
- Exit `3`: profile blocked; inspect the JSON `reason`.
- The router passes process arguments directly. It never creates a shell command string, installs software, or changes the product runtime.
