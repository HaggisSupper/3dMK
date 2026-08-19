#!/usr/bin/env bash
set -euo pipefail

repository_root="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
contract_path="$repository_root/contracts/local-intelligence-routing.v1.json"
source_path="$repository_root/src/local_intelligence.rs"

for path in "$contract_path" "$source_path"; do
  if [[ ! -f "$path" ]]; then
    printf 'Missing local-intelligence contract artifact: %s\n' "$path" >&2
    exit 1
  fi
done

for fragment in \
  '"contract_version": 1' \
  '"deterministic_first": true' \
  '"mistralrs_primary": true' \
  '"llamacpp_automatic_fallback": false' \
  '"cloud_egress_default_enabled": false' \
  '"local_cpu_inference_allowed": false' \
  '"ai_can_publish_authoritative_state": false'; do
  if ! grep -Fq "$fragment" "$contract_path"; then
    printf 'Local-intelligence contract is missing: %s\n' "$fragment" >&2
    exit 1
  fi
done

for operation in MetadataInspection GeometryValidation EvidenceSummarization VisionAdjudication FloorplanInterpretation GeneralAssistance; do
  if ! grep -Fq "$operation" "$source_path"; then
    printf 'Typed operation is missing: %s\n' "$operation" >&2
    exit 1
  fi
done

for tier in Deterministic MistralRs LlamaCpp OpenAiCompatibleCloud Blocked; do
  if ! grep -Fq "$tier" "$source_path"; then
    printf 'Typed intelligence tier is missing: %s\n' "$tier" >&2
    exit 1
  fi
done

if ! grep -Fq 'LOCAL_INTELLIGENCE_CONTRACT_VERSION: u16 = 1' "$source_path"; then
  printf 'Local-intelligence source does not declare contract version 1.\n' >&2
  exit 1
fi

if ! grep -Fq 'automatic_fallback_allowed: false' "$source_path" && ! grep -Fq 'automatic_fallback_allowed = false' "$source_path"; then
  printf 'Local-intelligence source does not enforce explicit fallback.\n' >&2
  exit 1
fi

if grep -Fq 'automatic_fallback_allowed: true' "$source_path" || grep -Fq 'automatic_fallback_allowed = true' "$source_path"; then
  printf 'Local-intelligence source permits an automatic fallback.\n' >&2
  exit 1
fi

printf 'Local intelligence contract validation: PASS\n'
