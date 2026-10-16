# Domain configuration candidate evidence

**State:** `TASK_CANDIDATE` for the application-preference/dialog slice only. This is not `PROJECT_COMPLETE` and does not satisfy object-record, dimensional-reconstruction, CUDA-foundation, or release gates.

## Acceptance boundary

1. A bundled general-geometry profile preserves the current workstation as the default.
2. Rust validates a versioned upstream Veritas domain profile, stores the application-wide selection and catalog in one recoverable write, and checks content digests plus semantic validity on restart.
3. Import remains a preview until Apply; Cancel leaves persisted configuration unchanged.
4. The dialog discloses classes, properties, configured panels, validation errors, and invalid-preference recovery. Configured panel intent cannot authorize backend capabilities.
5. Mutation routes require a per-process token and matching local Host/Origin.
6. Existing project geometry is unaffected by preference changes.

## Evidence, 2026-09-22

- Upstream generic contract: Veritas PR #16, commit `01bbb77`, remains open for independent acceptance. The 3DMK Cargo dependency pins this commit; it is not represented as merged mainline authority.
- Red tests: `cargo test --lib preview_does_not_install_and_apply_persists_atomically` failed for missing preview/apply methods; `cargo test --lib domain_preference_routes_require_explicit_action_and_persist` failed with forged Origin returning 200 instead of 403; `cargo test --lib rejects_semantically_invalid_profile_even_with_matching_digest` failed before upstream semantic validation was added.
- Focused repairs: all three red tests passed after their production changes.
- Fresh independent verifier session `01a0caca-3665-7ff3-b890-8f6998f79253`: `cargo fmt --all -- --check` exit 0; `cargo test --lib` exit 0, **93 passed / 0 failed**; `cargo check --manifest-path src-tauri/Cargo.toml` exit 0; `node --check public/domain-config.js` exit 0; `git diff --check` exit 0 (line-ending warnings only). Cargo used `CARGO_NET_GIT_FETCH_WITH_CLI=true` for the private pinned dependency.
- Independent reviewer sessions: first review `01a0cab4-6499-7f20-b384-87c11e046bcc` found six issues; re-review `01a0cac7-7a05-7af1-b5d8-7fc7b5a5f4fb` accepted the fail-closed capability boundary but found missing semantic validation; final bounded review `01a0caca-8004-7010-bb63-50231654b394` passed with no new blocking/important finding.
- Browser test through the loopback Axum server: invalid legacy preference displayed a blocking recovery dialog; Apply recovered to default; JSON profile preview did not alter the catalog after Cancel; Apply installed and selected it; reload restored it; the default was then reselected. Console had no errors after the recovery fix.
- Windows Tauri release build: `cargo tauri build --no-bundle` produced `three-dmk-desktop.exe` in the configured shared Cargo release target. This is a binary build, not a clean-machine installer/offline acceptance test.

## Residual scope and risks

- The upstream Veritas PR is not merged. Composition/capability admission is deliberately fail-closed: nonempty capability IDs and composition IDs are rejected, and panel labels mean configured presentation, not backend availability. No claim is made that arbitrary domain operations are usable.
- Generic class-bearing project records, property editors, immutable embedded geometry admission, dimensional reconstruction, candidate revisions, source mappings, and package round-trip remain unimplemented.
- Deployment-policy lock enforcement is deferred; the API currently reports `locked: false`.
- Clean-machine offline installation, supported NVIDIA/CUDA evidence, fault-matrix, and wider product release gates are not covered by this slice.
