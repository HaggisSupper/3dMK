# 3DMk Capability Domain Non-Negotiable Acceptance Gates

**Status:** Mandatory merge/release gates for the Veritas-child capability migration.

## Gate A — Veritas parent relationship

PASS requires:

- migrated 3DMk surfaces depend on explicit compatible Veritas contract/runtime versions and fingerprints;
- Veritas has no dependency on 3DMk;
- every generic framework contract required by 3DMk exists canonically upstream;
- generic contracts are not redefined locally;
- CI detects dependency cycles, incompatible pins, and shadow generic contracts;
- the adapter remains thin and geometry-domain logic remains outside it.

FAIL if 3DMk locally invents a generic framework primitive because Veritas does not yet expose one.

## Gate B — Epistemic challenge

PASS requires every normative claim to be `ESTABLISHED` or `VALIDATED`.

For high-confidence claims, evidence MUST include residual uncertainty, strongest plausible countercase, tested boundary/failure conditions, comparison with a simpler alternative where one exists, reproducibility evidence, and explicit error/tolerance bounds where applicable.

FAIL if `HYPOTHESIS` or `RIFF` material is presented as required production architecture.

## Gate C — Strongly typed capability contracts

PASS requires stable capability identity/version; typed inputs/outputs; explicit preconditions, postconditions, effects and hard constraints; typed/unit-bound parameters; resource/provider requirements; cancellation/failure/provenance behavior; and rejection of invalid composition before execution.

## Gate D — Deterministic planning legality

PASS requires only registered compatible capabilities can be emitted; hard constraints cannot be bypassed; search is explicitly bounded; impossible goals fail closed with evidence; and plan validation is independent of plan generation.

## Gate E — Transaction and revision authority

PASS requires mutating capabilities follow `admit → stage → execute → validate → publish → finalize`; no pre-publication output is authoritative; revision lineage remains complete; topology changes retain mapping/transfer evidence; and cancellation/failure leaves prior authoritative state intact.

## Gate F — Outcome evaluation truth

PASS requires `LatestState`, `BestState`, `PresentedState`, and `AuthoritativeState` remain distinct concepts; hard constraints remain separate from objective scores; evaluator evidence/tolerance is persisted; and a deliberately worse later candidate cannot replace a superior incumbent.

Mandatory regression: candidate N is measurably superior to N+1 and remains `BestState` after N+1 completes.

## Gate G — Bounded convergence

Where iterative convergence exists, PASS requires explicit iteration/time/compute/memory limits; cancellation safety; incumbent preservation; bounded plateau/oscillation handling appropriate to the selected strategy; restart/checkpoint safety where persistent; and no infinite loop under adversarial tests.

## Gate H — Hardware-aware execution

PASS requires generic provider/resource arbitration is governed by Veritas core; 3DMk providers declare capability-specific requirements; resource reservations prevent unsafe overcommit; CUDA-primary paths have correctness evidence; fallback paths are verified per capability; and intelligence/geometry workloads cannot independently overcommit shared resources.

If standalone in-process providers are required, PASS additionally requires an upstream Veritas host/provider contract that explicitly admits them; a 3DMk-local exception is a failure.

## Gate I — Intelligence escalation

Where escalation is enabled, PASS requires:

- tier order and admission policy are explicit;
- each enabled tier has a declared competence/abstention contract;
- lower tiers escalate only with evidence-backed reason;
- a deterministic Veritas escalation packager constructs the schema-bound minimal context;
- embedded-model output may inform the package but cannot rewrite hard constraints, policy, or authority;
- local LLM/VLM/multimodal models are lazy-loaded and resource-admitted;
- cloud use is optional, policy-controlled, data-minimized, and restricted to approved OpenAI-compatible endpoints;
- credentials never enter prompts, model artifacts, project payloads, or provenance logs;
- raw bulk geometry is not sent to a general language model by default;
- every response returns through typed schema validation and deterministic Veritas admission;
- model confidence cannot override measured outcome evidence or hard constraints;
- disabling or losing every probabilistic tier leaves deterministic core workflows intact.

Adversarial PASS evidence SHALL include false high-confidence lower-tier output, malformed higher-tier output, prompt injection in project/user content, stale revision, local OOM/crash, cloud denial/offline/timeout, oversized multimodal evidence, and a confident higher-tier proposal that produces a worse measurable result.

## Gate J — Standalone Tauri deployment

PASS requires clean Windows installation and offline deterministic-core launch; pinned required Veritas core components are packaged with the 3DMk child domain; no separately installed Veritas application is required; deterministic geometry workflows operate without cloud access; and Tauri presentation does not become geometry authority.

## Gate K — Provenance and reproducibility

Every promoted result SHALL identify source revision/hash, goal/constraints, admitted plan/DAG, capability/provider versions, parameters, hardware profile, evaluator outputs/tolerances, incumbent comparison, and publication transaction. Any escalation used SHALL additionally identify tier/provider/model where applicable, reason, package/result hashes, and disposition.

## Gate L — Fault matrix

Applicable tests SHALL cover invalid/corrupt input, incompatible capability contract, no-path planning, capability error containment, cancellation/timeout, OOM, accelerator initialization/device loss/fallback failure, disk-full, evaluator failure, non-improvement/regression, restart at transaction boundaries, stale hardware/framework compatibility, local-model unavailable/OOM, malformed model response, cloud unavailable/unauthorized, prompt injection, stale escalation source, and escalation-schema failure.

No fault may silently corrupt authoritative state.

## Gate M — Regression preservation

PASS requires current supported imports, exports, VWM paths, measurements, revision/package invariants, and rendering-authority boundaries remain functional or have a proven replacement with rollback evidence.

## Gate N — Documentation truth and decision disposition

PASS requires:

- `AGENTS.md` and governing documents reflect actual authority;
- `docs/CURRENT_STATE.md` remains implemented truth rather than target claims;
- ADRs match accepted architecture;
- speculative research mechanisms do not appear as normative requirements;
- every explicitly `APPROVED` requirement is represented in governing documentation with normative `SHALL`/`MUST`/`REQUIRED` semantics, explicit scope, and acceptance evidence;
- every explicitly `REJECTED` mechanism is removed or represented as a normative prohibition where recurrence must be prevented;
- `SUPERSEDED` material is removed from active authority or points unambiguously to its replacement;
- `DEFERRED`, `HYPOTHESIS`, and `RIFF` material remains expressly non-normative;
- no contradictory active normative requirements remain after a decision changes;
- exact evidence is tied to commits/artifacts before completion claims.

FAIL if conversation approval/rejection has not been reflected in repository authority, or if approval is used to claim implementation that has not been evidenced.

## Final promotion rule

No implementation branch may be reported complete until all applicable gates pass freshly. High confidence alone is never acceptance evidence.