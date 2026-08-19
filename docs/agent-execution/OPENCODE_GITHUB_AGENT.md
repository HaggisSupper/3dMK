# OpenCode GitHub Delegated Agent

## Purpose

This is an optional, review-gated delegation lane for bounded repository tasks. It is not the authoritative 3dMK executor and it does not replace the local CUDA-backed Mistral.rs harness.

The workflow runs OpenCode on a GitHub-hosted runner after an owner-authored /opencode or /oc comment. It can create a branch, commit changes, and open a pull request. It must never publish directly to main.

## Required repository settings

Configure these under Settings → Secrets and variables → Actions:

1. Repository secret: OPENROUTER_API_KEY.
2. Repository variable: OPENCODE_MODEL, using a current provider/model identifier.

The runner cannot reach a Mistral.rs service bound only to a private Windows machine or localhost. Use this lane only with the configured cloud model, or move the workflow to an approved self-hosted runner before attempting local inference.

## Invocation

Comment on an issue or pull request:

    /opencode implement only the bounded task in this issue, run the required checks, and open a draft pull request. Do not modify authoritative geometry, CUDA, persistence, or release contracts without explicit scope in the issue.

Only comments authored by the repository owner trigger the workflow.

## Supervision contract

Every delegated task must:

- name exact files and acceptance tests;
- remain within one independently reviewable scope;
- preserve AGENTS.md, ADRs, immutable contracts, and the admit → stage → execute → validate → publish → finalize boundary;
- avoid secrets, credentials, production data, and destructive commands;
- produce a pull request for review;
- include commands run, results, limitations, and rollback notes.

The supervisor must inspect the full diff, workflow logs, test evidence, dependency changes, and security implications before merge. A successful agent run is not evidence that the product requirement is satisfied.

## Appropriate first task for the 3D lockup

Use a bounded issue for instrumentation and admission control:

- one heavy-job permit;
- phase-level memory and CPU counters;
- deterministic resource-limit errors;
- no algorithm rewrite;
- no browser serialization redesign;
- no worker-process activation.

The browser-copy, quadratic VWM, worker isolation, and Three.js disposal changes must remain separate tasks.

## Security and operating limits

- The trigger is owner-gated and comment-based.
- The job is limited to 60 minutes and cannot run concurrently for the same issue or pull request.
- The action uses the repository GITHUB_TOKEN and the configured OpenRouter secret.
- Do not paste API keys, local paths, model credentials, or private data into issue comments.
- Treat anomalyco/opencode/github@latest as a supply-chain dependency; pin and review an immutable action revision before treating this lane as release-grade.

## Rollback

Delete or revert this workflow and documentation. Existing branches and pull requests remain reviewable; no authoritative state is changed until a reviewed pull request is merged.
