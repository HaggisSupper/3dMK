# Copilot Repository Instructions

Read AGENTS.md first, then docs/CURRENT_STATE.md and the governing documents it names.

The task supplied in the generated issue is the only implementation scope. The issue contains the exact content diff from .github/DEV_INSTRUCTIONS.md. Implement only the new or changed task.

Required behavior:

- Work on the assigned Copilot branch and open a draft pull request.
- Do not edit .github/DEV_INSTRUCTIONS.md.
- Do not modify main, secrets, credentials, or unrelated files.
- Preserve the admit → stage → execute → validate → publish → finalize boundary.
- Preserve Rust authority, CUDA-first design, Windows-first support, and no-Docker rules.
- Run focused tests, affected regression tests, formatting, and relevant validation before requesting review.
- Report exact commands, results, limitations, and rollback notes in the pull request.
- If the task conflicts with AGENTS.md or a governing ADR, stop and report the conflict instead of weakening the contract.
