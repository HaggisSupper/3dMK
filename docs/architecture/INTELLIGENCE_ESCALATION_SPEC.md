# 3DMk Intelligence Escalation Specification

**Status:** Proposed architecture boundary; implementation details remain unvalidated until measured  
**Owner:** Veritas core. 3DMk supplies geometry-domain context, evidence renderers, and typed goals.

## 1. Purpose

Provide a bounded competence-escalation path. The cheapest competent mechanism is used first; escalation occurs only when evidence shows the current tier cannot resolve the request within its declared contract.

This specification commits to the escalation boundary, not to a particular model family, parameter count, adapter, training method, or provider.

## 2. Escalation ladder

```text
Deterministic semantic/context normalization
        ↓ unresolved
Embedded narrow ML, if validated for the task
        ↓ unresolved / low confidence / modality gap
Local LLM / VLM / multimodal model, if admitted
        ↓ unresolved / capability deficit / policy permits
Approved OpenAI-compatible cloud LLM/VLM endpoint
        ↓
Typed proposal or unresolved result
        ↓
Veritas deterministic validation and application execution
```

Every tier SHALL return exactly one disposition:

- `RESOLVED` with a schema-valid typed proposal;
- `ABSTAINED` with evidence;
- `ESCALATE` with a bounded escalation package;
- `FAILED` with typed failure evidence.

No intelligence tier gains capability-legality, authorization, transaction, or publication authority.

## 3. Deterministic escalation packager

Prompt/context construction SHALL be owned by a deterministic Veritas escalation-packaging capability. This avoids requiring a small classifier to be generative.

The packager MAY consume embedded-model outputs such as intent candidates, confidence, category, entities, and unresolved ambiguity. A validated embedded model MAY additionally produce bounded structured fields, but its free-form text SHALL NOT be authoritative.

The package SHALL contain only the minimum sufficient context:

- original operator request preserved verbatim in provenance;
- deterministic application facts and active object/state identity;
- relevant 3DMk/Veritas capability summaries;
- hard constraints, authority limits, and risk class;
- previous-tier interpretation, confidence, and competence result;
- unresolved questions and candidate intents/goals;
- evidence already gathered;
- exact question the next tier must resolve;
- permitted output schema;
- token/media/resource budget;
- escalation-chain provenance.

The packager SHALL NOT weaken, omit, or rewrite hard constraints or authorization policy.

## 4. Multimodal evidence

Bulk geometry SHALL NOT be inserted into general-model context by default.

When visual or spatial evidence is required, a dedicated 3DMk evidence capability SHALL produce bounded artifacts such as approved renders, views, measurements, feature summaries, or other explicitly contracted representations. Source revision/hash and transformation provenance SHALL accompany every derived evidence artifact.

## 5. Local higher-capability tier

Local higher-capability reasoning is preferred before cloud use when an approved model/runtime can be admitted within Veritas resource policy.

Admission SHALL consider task modality, declared competence, model integrity, context requirements, RAM/VRAM reserve, active geometry workload, latency policy, and provider availability.

Higher-capability models SHALL be lazy-loaded. Normal deterministic workflows SHALL not reserve their resources.

## 6. Cloud tier

Cloud escalation MAY occur only when:

- lower tiers return evidence-backed abstention/escalation;
- the additional capability is materially relevant;
- current operator/project policy permits cloud use;
- data-minimization/confidentiality checks pass;
- the endpoint is explicitly approved and OpenAI-compatible at the client boundary.

Cloud use is optional and fail-closed. Credentials SHALL NOT be placed in prompts, project payloads, model bundles, or provenance logs.

## 7. Return-to-authority boundary

Every intelligence response SHALL re-enter through the same deterministic path:

```text
response
  → schema validation
  → typed goal/capability resolution
  → contract + authority + risk validation
  → resource admission
  → candidate execution
  → objective outcome evaluation
  → explicit promote/reject/publication transaction
```

Model confidence SHALL NOT override Veritas legality, hard constraints, or measured outcome evidence.

## 8. Escalation triggers

Escalation SHALL require typed evidence such as:

- out-of-domain/competence rejection;
- confidence below a versioned threshold;
- unresolved compound intent;
- required modality unavailable at the current tier;
- no legal deterministic plan to the requested goal;
- evaluator conflict requiring bounded interpretation;
- repeated bounded failure with useful unresolved evidence.

A larger model being available is not an escalation reason.

## 9. Provenance and effectiveness

Every escalation step SHALL record provider/model identity where applicable, reason, package hash, competence/confidence state, output schema/result hash, latency/resource cost, disposition, and downstream measured outcome when executed.

This evidence SHALL permit later comparison of whether an escalation tier materially improves task completion enough to justify its cost and risk.

## 10. Failure and adversarial cases

Required tests before a tier is production-enabled include:

- false high-confidence lower-tier result;
- malformed and schema-invalid higher-tier response;
- prompt/context omission of a hard constraint;
- prompt-injection text inside user/project content;
- malicious or irrelevant model output;
- local model OOM/crash/unavailability;
- cloud policy denied/offline/timeout/rate-limit;
- credential leakage attempts;
- oversized multimodal evidence;
- stale source revision during escalation;
- higher-tier answer that is confident but measurably worse.

Every case SHALL fail closed without corrupting authoritative state.

## 11. Challenge gate

Before any concrete implementation choice becomes normative, ask:

> How much of this is speculative bullshit because we enjoy the riff?

Exact model architecture, embedded-model presence, prompt optimizer, adapter method, local model family, learning method, and cloud provider remain non-normative until they beat a simpler alternative within declared accuracy, latency, resource, privacy, and failure bounds.