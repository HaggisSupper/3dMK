# 3DMk Hardware-Aware Execution Specification

## 1. Objective

3DMk SHALL expose geometry execution providers to Veritas, which owns generic hardware discovery, resource arbitration, provider admission, provider topology, and placement policy.

Hardware awareness is runtime scheduling, not geometry business logic.

## 2. Discovery profile

Veritas SHALL maintain the machine capability profile used by 3DMk, including relevant CPU/SIMD, RAM, GPU/VRAM, CUDA, Vulkan/WebGPU, driver/runtime, and provider-version evidence sufficient to invalidate stale decisions.

3DMk SHALL not invent a second generic hardware inventory.

## 3. Provider abstraction

A geometry capability may expose multiple providers, for example:

```text
geometry.estimate_normals
  - CPU reference/SIMD
  - CUDA
  - Vulkan/WebGPU
```

All providers MUST conform to the same capability semantics. Provider selection and topology admission SHALL be centrally governed by Veritas.

## 4. Placement evidence

Placement MAY consider verified correctness/parity, dataset size, transfer/setup overhead, current load, VRAM/RAM reserve, latency/quality goals, deterministic requirements, expected duration, and adjacent-DAG data locality.

A GPU SHALL not be selected merely because it exists. Correctness and resource safety override performance preference.

## 5. CUDA-first geometry policy

For supported NVIDIA systems, CUDA SHALL remain the primary provider for eligible heavy geometry operations after correctness, parity, fault, memory, and performance gates pass.

Vulkan/WebGPU MAY be a verified capability-specific fallback. CPU reference paths SHALL remain where required for validation or small-workload efficiency.

## 6. Resource authority

Veritas SHALL govern leases/reservations for accelerator compute, VRAM, CPU worker budget, staging memory, and model residency.

No 3DMk capability SHALL independently seize accelerator resources or hold authoritative project-store locks while waiting for an accelerator lease.

## 7. Data locality

The scheduler SHOULD avoid unnecessary device/host transfers when adjacent validated capability providers can safely consume device-resident intermediates. Durable authority and persistence remain Rust-owned.

## 8. Intelligence-tier resource admission

The escalation hierarchy defined in `INTELLIGENCE_ESCALATION_SPEC.md` SHALL use the same Veritas resource authority.

Any in-process intelligence provider is conditional on a canonical Veritas host/provider topology contract that explicitly admits in-process execution. Until that upstream contract exists and is pinned, 3DMk SHALL NOT create a local exception.

When enabled under an approved topology:

- lightweight embedded inference MAY run in-process;
- local LLM/VLM/multimodal providers SHALL be lazy-loaded only when escalation requires them;
- local model selection SHALL consider modality, competence, context size, VRAM/RAM reserve, active geometry work, and approved provider/model availability;
- loading a larger model SHALL NOT blindly evict critical geometry state or violate safety reserves;
- cloud escalation avoids local residency but remains subject to network/privacy/policy admission.

## 9. Calibration and measured selection

Veritas MAY persist measured provider characteristics keyed to material hardware/runtime fingerprints. Stale fingerprints SHALL invalidate affected calibration.

3DMk SHALL contribute representative geometry crossover/parity evidence rather than generic framework policy.

## 10. Veto authority

The Veritas resource authority SHALL veto unsupported/unverified providers, insufficient memory reserve, unsafe concurrency, incompatible driver/runtime combinations, disallowed topology, or higher-tier model loads that destabilize active work.

Vetoes SHALL return structured evidence so the planner/escalation router can select a safe alternative or abstain.

## 11. Fault behavior

The hardware layer SHALL explicitly handle applicable initialization failure, device loss/reset, OOM, provider crash, stale calibration, driver/runtime incompatibility, and fallback initialization failure.

Fallback occurs only to an admitted provider for the same capability semantics. Silent quality degradation is prohibited.

## 12. Challenge rule

New scheduling sophistication SHALL be justified by measurements. The architecture does not mandate predictive schedulers, learned routing, multiple inference frameworks, or in-process inference merely because they are technically possible.