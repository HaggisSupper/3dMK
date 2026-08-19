#!/usr/bin/env bash
set -euo pipefail

test_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$test_root/scripts/validate-3dmk-exec-router-contract.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/tools/3dmk-exec-router/src"
cp "$test_root/tools/3dmk-exec-router/contract.json" "$fixture/tools/3dmk-exec-router/contract.json"

cat > "$fixture/tools/3dmk-exec-router/src/main.rs" <<'RUST'
const CONTRACT_VERSION: u8 = 1;
const PROFILES: &[&str] = &[
    "toolkit-check",
    "windows-toolkit-check",
    "rust-check",
    "cuda-probe",
    "mistralrs-agent",
];
fn main() {
    let executable = "powershell.exe";
    let mut command = std::process::Command::new(executable);
    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
}
RUST

if "$validator" "$fixture" >/dev/null 2>&1; then
  printf 'Unsafe PowerShell execution policy was accepted.\n' >&2
  exit 1
fi

sed -i 's/, "-ExecutionPolicy", "Bypass"//' "$fixture/tools/3dmk-exec-router/src/main.rs"
if ! "$validator" "$fixture" >/dev/null; then
  printf 'Safe PowerShell invocation was rejected.\n' >&2
  exit 1
fi

printf '3DMK execution-router contract tests: PASS\n'
