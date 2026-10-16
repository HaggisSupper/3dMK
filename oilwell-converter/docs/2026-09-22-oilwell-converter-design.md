# Oilwell Converter standalone application design

## Status and scope

**Status:** APPROVED FOR PLANNING — 2026-09-22

This design defines `oilwell-converter`, an independent Windows 11 Tauri application adjacent to, but not linked with, 3DMK. It is a portable, offline conversion tool for directional-survey, section, BHA, and formation-top CSV tables.

The application SHALL have its own Cargo workspace, lockfiles, frontend dependencies, Tauri configuration, tests, sample data, documentation, and release artifacts. It SHALL NOT import 3DMK crates, source files, configuration, assets, runtime services, or package data.

The supported target is Windows 11 x64 with the standard Microsoft WebView2 Runtime installed. A bundled Fixed Version WebView2 runtime is out of scope for this release.

## Goals

- Provide a native, offline desktop workflow for converting well-table CSV inputs into GLB, OBJ, PLY, or STL output.
- Provide a compiled Windows CLI for the same conversion workflow and scripted/batch usage.
- Use one Rust-authoritative conversion library for the Tauri app and CLI; neither executable may reimplement conversion logic.
- Preserve CAD source orientation: X/Y are plan coordinates, Z is elevation, and depth is negative Z.
- Apply the glTF root rotation needed for Y-up viewers while retaining native CAD Z-up coordinates in OBJ, PLY, and STL.
- Make source table validation, errors, generated geometry, and publication deterministic and locally reproducible.

## Non-goals

- Integration, dependency, data exchange, or shared configuration with 3DMK.
- Network access, accounts, telemetry, cloud conversion, or automatic updates.
- A general-purpose CAD editor, 3D viewer, project database, or well-data management system.
- Bundling a WebView2 fixed runtime.

## Repository layout

```text
oilwell-converter/
├─ Cargo.toml                         # independent workspace
├─ Cargo.lock
├─ crates/
│  └─ oilwell-core/                   # validation, trajectory, mesh, exporters
├─ src-tauri/                         # Tauri desktop executable and commands
├─ ui/                                # React, TypeScript, Vite, semantic tokens
├─ samples/                           # representative CSV source tables
├─ tests/                             # CLI and release smoke tests
├─ docs/                              # design, CLI reference, portable instructions
└─ dist/windows-portable/             # generated distribution; not hand-authored
   ├─ Oilwell Converter.exe
   ├─ oilwell-converter-cli.exe
   ├─ README.txt
   ├─ LICENSES/
   └─ samples/
```

`dist/` is a generated artifact directory. It SHALL NOT be used as a source-of-truth input to builds.

## Runtime architecture

```text
React/Vite UI ── typed Tauri invoke ──> Tauri command adapter
                                          │
CLI arguments ── typed CLI adapter ──────┼──> oilwell-core
                                          │      validate → interpolate → mesh → stage → validate → publish
                                          └──> typed success or error result
```

`oilwell-core` SHALL own CSV parsing, column validation, trajectory interpolation, dimensional validation, CAD coordinate generation, GLB/OBJ/PLY/STL serialization, and atomic output publication. Tauri and Clap adapters SHALL translate their ingress to the shared typed request and map shared typed outcomes to their presentation surface.

The UI SHALL invoke a narrow `convert` command. It SHALL NOT receive broad filesystem, shell, process, or network permissions. Native file dialogs may select paths; the command receives only the selected paths and typed settings.

The GUI SHALL call the shared Rust library through a Tauri command rather than spawning the CLI. This prevents shell quoting ambiguity, temporary-file coupling, and duplicate error streams while retaining the CLI as an equivalent automation interface.

## Conversion contract

### Request

```text
ConversionRequest
  survey_path: Path
  sections_path: Path
  bha_path: Option<Path>
  formation_path: Option<Path>
  output_path: Path
  output_format: Glb | Obj | Ply | Stl
  diameter_scale: positive finite number
  smooth_step_md: positive finite metres
  interpolation_method: MinimumCurvature | LinearDirectionBlend
  colour_palette: typed component-kind → RGB mapping
```

Survey rows require `well_id`, `md`, `inc_deg`, and `azi_deg`; `pad_id` and surface coordinates are optional. Sections require `well_id`, `kind`, `md_from`, `md_to`, `od_in`, `id_in`, and `name`. BHA and formation-top input schemas retain the established supported fields and are optional as a whole.

Every numeric field SHALL be finite. A well survey SHALL have strictly increasing MD after parsing. A tubular component SHALL have a positive OD, non-negative ID, and `ID < OD`. Unknown component colours SHALL use the documented neutral default and produce a warning; structural input errors SHALL fail conversion.

### Result and error

```text
ConversionResult
  output_path: Path
  wells: unsigned count
  formations: unsigned count
  meshes: unsigned count
  warnings: list of user-readable warnings

ConversionError
  code: InvalidInput | MissingColumn | InvalidNumber | NonIncreasingMd |
        InvalidDimensions | UnsupportedFormat | ReadFailed | WriteFailed |
        ValidationFailed | PublishFailed
  message: user-actionable text
  field: optional input field identifier
  source_path: optional path
```

Error codes are stable machine-readable contract values. The Tauri adapter SHALL map field-associated errors to the relevant UI control; all error paths SHALL preserve selected input paths and settings.

## Geometry and export behavior

`MinimumCurvature` is the default interpolation method. It SHALL use spherical interpolation of directional vectors and the minimum-curvature ratio factor. `LinearDirectionBlend` remains an explicit comparison option.

The source coordinate convention SHALL be `cad_z_up`: X/Y plan, Z elevation, and downward depth as negative Z. Geometry arrays SHALL reject non-finite position or normal data before serialization.

GLB SHALL preserve the pad/well/component hierarchy, material palette, transparent open-hole materials, component metadata, and source-coordinate extras. Its root SHALL rotate CAD Z-up geometry into glTF Y-up display coordinates. OBJ, ASCII PLY, and binary STL SHALL export the same generated triangle geometry in native CAD Z-up coordinates; these formats do not preserve GLB hierarchy or material semantics.

## Publication transaction

Every output follows this local transaction:

```text
admit request → stage beside target → serialize → validate bytes → atomic rename → report result
```

The staging path SHALL be unique and located on the target filesystem. Validation SHALL include format-specific structural checks and finite-bounds checks before publication. A conversion failure SHALL remove its stage file where possible and SHALL NOT overwrite or partially replace an existing requested output.

## CLI contract

```text
oilwell-converter-cli.exe convert \
  --survey survey.csv --sections sections.csv \
  [--bha bha.csv] [--formations formation_tops.csv] \
  --output well-pad.glb --format glb \
  [--diameter-scale 36] [--smooth-step-md 75] \
  [--interpolation minimum-curvature] [--json]
```

The CLI SHALL write human-readable diagnostics to stderr. With `--json`, it SHALL write exactly one JSON success or error document to stdout and preserve meaningful non-zero process exit codes for failures.

## Desktop UI

The UI SHALL be React/TypeScript/Vite and follow the application-style-system baseline:

- Use semantic tokens for light and dark themes; components SHALL NOT use theme-specific colour literals directly.
- Provide drop zones paired with named file-choice buttons for Survey, Sections, optional BHA, and optional Formation tops.
- Provide persistent field labels, unit labels, validation messages, and keyboard-accessible controls.
- Provide number fields for diameter exaggeration and station step; a select for interpolation; a radio group for output format; and component palette colour inputs.
- Provide one primary `Export model` action in the export region, with disabled, loading, success, and error states.
- Provide a result panel that persists output path, counts, warnings, and an accessible `Reveal output folder` action after successful export.
- Use a visible focus treatment, minimum 44 by 44 CSS-pixel interactive targets, reduced-motion behavior, and status communication that does not rely on colour alone.

The UI SHALL use typed Tauri command payloads and structured result/error mapping. It SHALL not claim success until the Rust command has returned a validated success result.

## Verification

The implementation SHALL provide these deterministic checks:

1. Core unit tests for CSV parsing, missing columns, invalid finite values, duplicate/non-increasing MD, interpolation behavior, CAD Z-up convention, all export writers, finite GLB bounds, and atomic failure behavior.
2. CLI integration tests for valid sample conversion, invalid input failure, `--json` contracts, exit codes, and absence of partial output.
3. React/Vitest/Testing Library tests for the primary export flow, accessible labels and focusable controls, loading state, retained settings after an error, structured backend-error mapping, and success result rendering.
4. Tauri production build validation and a portable-folder smoke test that executes the included CLI against the included samples and validates the generated output.

## Distribution and documentation

The generated Windows portable folder SHALL include both executables, representative samples, a plain-text quick-start guide, CLI usage examples, the WebView2/Windows 11 requirement, supported format semantics, and third-party license notices.

The quick-start guide SHALL state that no data is uploaded or retained by the application beyond the user-selected input/output locations.

## Risks and bounded limitations

- The portable folder depends on the Windows 11 WebView2 Runtime; a machine without it is unsupported for this release.
- Tauri application packaging and raw executable portability need fresh host validation; raw EXE output is not a substitute for a tested folder distribution.
- OBJ, PLY, and STL have lower semantic fidelity than GLB by format design.
- The initial app does not include a 3D preview; verification is through generated output and result summaries.
