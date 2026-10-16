# 3DMK repository map

This records the current top-level layout; it does not approve consolidation of legacy directories.

| Directory | Purpose |
| --- | --- |
| `.codex` | Repository-local agent skills and instructions. |
| `.github` | GitHub automation. |
| `config` | Product configuration and bundled defaults. |
| `contracts` | 3DMK's machine-readable conformance contracts. |
| `docs` | Product architecture, plans, status, and execution evidence. |
| `oilwell-converter` | Child oilwell conversion application and its crates/UI. |
| `Open Design Prototypes` | Historical UI design prototypes. |
| `output` | Git-ignored local runtime exports and application data, created when the browser-development server runs. |
| `public` | Browser-rendered workstation assets. |
| `scripts` | Validation and local development task runners. |
| `spatial-engineering-platform` | Spatial engineering bootstrap module. |
| `src` | Rust backend application source. |
| `src-tauri` | Tauri desktop shell and resources. |
| `tests` | Integration tests and fixtures. |
| `tools` | Standalone development utilities, including the execution router. |
| `VWM-Repo-Implicit` | VWM implicit-geometry subproject. |

Root manifests and source files remain at repository root. `scripts` and `tools` have distinct current roles; no repository-wide structural changes are implied by this map.
