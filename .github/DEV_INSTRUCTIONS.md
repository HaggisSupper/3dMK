# 3dMK Development Instructions

This file is the reviewed task queue for the next bounded implementation increment.

## Next task

Add phase-level resource admission and telemetry for advanced processing.

Acceptance:

- Route every heavy processing request through one bounded admission path.
- Permit only one heavy job by default on the reference Windows machine.
- Record route, job ID, point count, face count, estimated bytes, peak private bytes, response bytes, elapsed time, and cancellation outcome.
- Reject work before allocation when the estimate exceeds the available budget.
- Add deterministic tests for accepted, rejected, concurrent, and cancelled requests.
- Do not redesign the browser serializer, VWM algorithms, CUDA worker, or Three.js lifecycle in this increment.
- Open a draft pull request with the focused diff and test evidence.

## Operating rules

- Read AGENTS.md and the governing documents before editing.
- Preserve the admit → stage → execute → validate → publish → finalize boundary.
- Rust remains authoritative; JavaScript remains presentation and bounded evidence capture.
- CUDA-first requirements, Windows-first support, and no-Docker rules remain unchanged.
- Never edit this file from the delegated implementation task.
- Never merge or publish directly to main.
