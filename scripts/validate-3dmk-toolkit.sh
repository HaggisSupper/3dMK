#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
required_files=(
  "AGENTS.md"
  "docs/CURRENT_STATE.md"
  ".codex/skills/3dmk-dev/SKILL.md"
  ".codex/skills/3dmk-dev/references/authority-map.md"
  ".codex/skills/3dmk-dev/references/verification-matrix.md"
)

for relative_path in "${required_files[@]}"; do
  if [[ ! -f "$repository_root/$relative_path" ]]; then
    printf 'Missing required 3DMK toolkit file: %s\n' "$relative_path" >&2
    exit 1
  fi
done

skill_path="$repository_root/.codex/skills/3dmk-dev/SKILL.md"
required_headings=(
  "## Trigger"
  "## Authority boundary"
  "## Deterministic evidence"
  "## Workflow"
  "## Output artifact"
  "## Stop condition"
)

for heading in "${required_headings[@]}"; do
  if ! grep -Fqx "$heading" "$skill_path"; then
    printf 'SKILL.md is missing required heading: %s\n' "$heading" >&2
    exit 1
  fi
done

if ! grep -Fq "admit → stage → execute → validate → publish → finalize" "$skill_path"; then
  printf 'SKILL.md is missing the required transaction boundary.\n' >&2
  exit 1
fi

if grep -Ein '\b(TODO|FIXME)\b' "$skill_path"; then
  printf 'SKILL.md contains an unsupported placeholder.\n' >&2
  exit 1
fi

printf '3DMK toolkit validation: PASS\n'
