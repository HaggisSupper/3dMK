#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$repository_root/scripts/validate-veritas-child-contract.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/contracts" "$fixture/.codex/skills/3dmk-dev"
cp "$repository_root/contracts/veritas-child-conformance.v1.json" "$fixture/contracts/"
cp "$repository_root/AGENTS.md" "$fixture/"
cp "$repository_root/.codex/skills/3dmk-dev/SKILL.md" "$fixture/.codex/skills/3dmk-dev/"

sed -i 's/Veritas/Not-Veritas/g' "$fixture/AGENTS.md"
if "$validator" "$fixture" >/dev/null 2>&1; then
  printf 'A repository without its Veritas parent declaration was accepted.\n' >&2
  exit 1
fi

cp "$repository_root/AGENTS.md" "$fixture/AGENTS.md"
if ! "$validator" "$fixture" >/dev/null; then
  printf 'A conforming Veritas child repository was rejected.\n' >&2
  exit 1
fi

printf 'Veritas child conformance tests: PASS\n'
