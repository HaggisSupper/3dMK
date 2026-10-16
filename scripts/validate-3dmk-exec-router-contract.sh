#!/usr/bin/env bash
set -euo pipefail

repository_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
contract_path="$repository_root/tools/3dmk-exec-router/contract.json"
source_path="$repository_root/tools/3dmk-exec-router/src/main.rs"

for path in "$contract_path" "$source_path"; do
  if [[ ! -f "$path" ]]; then
    printf 'Missing execution-router contract artifact: %s\n' "$path" >&2
    exit 1
  fi
done

required_contract_fragments=(
  '"contract_version": 1'
  '"shell_command_strings": false'
  '"execution_policy_bypass": false'
  '"install_or_mutate_tools": false'
  '"blocked_exit_code": 3'
  '"invalid_argument_exit_code": 2'
)

for fragment in "${required_contract_fragments[@]}"; do
  if ! grep -Fq "$fragment" "$contract_path"; then
    printf 'Execution-router contract is missing: %s\n' "$fragment" >&2
    exit 1
  fi
done

for profile in toolkit-check windows-toolkit-check rust-check cuda-probe mistralrs-agent; do
  if ! grep -Fq "\"$profile\"" "$contract_path" || ! grep -Fq "\"$profile\"" "$source_path"; then
    printf 'Execution-router profile is not jointly declared: %s\n' "$profile" >&2
    exit 1
  fi
done

if ! grep -Fq 'const CONTRACT_VERSION: u8 = 1;' "$source_path"; then
  printf 'Execution-router source does not declare contract version 1.\n' >&2
  exit 1
fi

if grep -Eq 'ExecutionPolicy[[:space:]]*",[[:space:]]*"Bypass|ExecutionPolicy[[:space:]]+Bypass' "$source_path"; then
  printf 'Execution-router must not bypass PowerShell execution policy.\n' >&2
  exit 1
fi

if grep -Eq '(^|[^[:alnum:]_-])(cargo install|winget |choco |scoop |Invoke-WebRequest|curl |wget )' "$source_path"; then
  printf 'Execution-router must not install or download tools.\n' >&2
  exit 1
fi

if grep -Eq '(^|[^[:alnum:]_-])(sh|bash|cmd|powershell)[[:space:]]+-[cC][[:space:]]' "$source_path"; then
  printf 'Execution-router must not construct shell command strings.\n' >&2
  exit 1
fi

if ! grep -Fq 'Command::new(executable)' "$source_path"; then
  printf 'Execution-router must start the selected executable directly.\n' >&2
  exit 1
fi

printf '3DMK execution-router contract validation: PASS\n'
