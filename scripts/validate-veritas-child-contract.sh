#!/usr/bin/env bash
set -euo pipefail

repository_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
contract_path="$repository_root/contracts/veritas-child-conformance.v1.json"
agents_path="$repository_root/AGENTS.md"
skill_path="$repository_root/.codex/skills/3dmk-dev/SKILL.md"

for path in "$contract_path" "$agents_path" "$skill_path"; do
  if [[ ! -f "$path" ]]; then
    printf 'Missing Veritas child conformance artifact: %s\n' "$path" >&2
    exit 1
  fi
done

required_contract_fragments=(
  '"contract_version": 1'
  '"parent_capability_framework": "Veritas"'
  '"child_repository": "3dMK"'
  '"versioned_machine_readable_contracts": true'
  '"validated_concrete_types_after_ingress": true'
  '"no_silent_fallback": true'
  '"no_undocumented_shared_mutable_state": true'
  '"deterministic_validation": true'
  '"negative_and_regression_tests": true'
  '"evidence_before_completion": true'
)

for fragment in "${required_contract_fragments[@]}"; do
  if ! grep -Fq "$fragment" "$contract_path"; then
    printf 'Veritas child contract is missing: %s\n' "$fragment" >&2
    exit 1
  fi
done

if ! grep -Fq 'child repository of the Veritas capability framework' "$agents_path"; then
  printf 'AGENTS.md must declare 3DMK as a Veritas child repository.\n' >&2
  exit 1
fi

if ! grep -Fq 'Veritas child conformance' "$skill_path"; then
  printf '3DMK development skill must enforce Veritas child conformance.\n' >&2
  exit 1
fi

printf 'Veritas child conformance validation: PASS\n'
