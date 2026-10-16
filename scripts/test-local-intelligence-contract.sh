#!/usr/bin/env bash
set -euo pipefail

repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validator="$repository_root/scripts/validate-local-intelligence-contract.sh"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/contracts" "$fixture/src"
cp "$repository_root/contracts/local-intelligence-routing.v1.json" "$fixture/contracts/"

cat > "$fixture/src/local_intelligence.rs" <<'RUST'
pub const LOCAL_INTELLIGENCE_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_REQUEST_BYTES: u64 = 16 * 1024 * 1024;
enum IntelligenceOperation { MetadataInspection, GeometryValidation, EvidenceSummarization, VisionAdjudication, FloorplanInterpretation, GeneralAssistance }
enum IntelligenceTier { Deterministic, MistralRs, LlamaCpp, OpenAiCompatibleCloud, Blocked }
fn route() { let automatic_fallback_allowed = true; }
RUST

if "$validator" "$fixture" >/dev/null 2>&1; then
  printf 'Automatic llama.cpp fallback was accepted.\n' >&2
  exit 1
fi

sed -i 's/automatic_fallback_allowed = true/automatic_fallback_allowed = false/' "$fixture/src/local_intelligence.rs"
if ! "$validator" "$fixture" >/dev/null; then
  printf 'Explicit no-fallback policy was rejected.\n' >&2
  exit 1
fi

printf 'Local intelligence contract tests: PASS\n'
