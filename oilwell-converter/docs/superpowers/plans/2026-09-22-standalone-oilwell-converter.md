# Standalone Oilwell Converter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a portable Windows 11 Tauri desktop application and paired CLI that converts oilwell CSV tables into CAD-oriented GLB, OBJ, PLY, and STL files offline.

**Architecture:** An independent Cargo workspace contains an `oilwell-core` library that owns validation, interpolation, mesh generation, serialization, and atomic publication. A Clap CLI and Tauri command adapter translate their ingress into the same typed `ConversionRequest`; React/Vite renders the user interface and invokes only the narrow Tauri command.

**Tech Stack:** Rust 2021, `serde`, `thiserror`, `csv`, `clap`, Tauri 2, React, TypeScript, Vite, Vitest, Testing Library, and the Windows 11 WebView2 Runtime.

**Spec:** `docs/2026-09-22-oilwell-converter-design.md`

## Global Constraints

- The project SHALL remain an independent repository and SHALL NOT import 3DMK code, crates, configuration, assets, or runtime services.
- The supported host is Windows 11 x64 with the standard WebView2 Runtime; a Fixed Version WebView2 runtime is excluded.
- Conversion is offline; no network, account, telemetry, automatic-update, shell, or broad filesystem capability is permitted.
- Rust SHALL own CSV parsing, validation, interpolation, geometry, format serialization, and atomic output publication.
- UI and CLI SHALL share `oilwell-core`; neither adapter may duplicate conversion behavior.
- Input geometry uses CAD Z-up: X/Y plan, Z elevation, and depth negative Z. GLB roots rotate to glTF Y-up; OBJ/PLY/STL retain CAD Z-up.
- Supported output formats are GLB, OBJ, ASCII PLY, and binary STL.
- All components use typed serializable contracts, structured errors, semantic design tokens, accessible controls, and deterministic tests.
- Every output follows `admit → stage → serialize → validate → publish → finalize`; failed output cannot replace an existing destination or leave a published partial file.

## Review Focus

- A CSV with a UTF-8 BOM or blank/comment rows parses identically to an ordinary valid table; Task 2 owns this regression test.
- A destination that already exists remains byte-for-byte unchanged when serialization or validation fails; Task 3 owns this failure test.
- A survey whose initial station is MD zero does not create a duplicated point or a non-finite displacement; Task 2 owns this trajectory test.
- A CLI `--json` error emits exactly one JSON document to stdout while diagnostics remain on stderr; Task 4 owns this integration test.
- A Tauri command field error keeps selected input paths and maps an understandable error next to the affected control; Task 6 owns this UI test.

---

## File structure

```text
Cargo.toml
crates/oilwell-core/Cargo.toml
crates/oilwell-core/src/{lib,contract,error,csv_input,trajectory,mesh,export,publish}.rs
crates/oilwell-core/tests/{csv_input,trajectory,export,publish}.rs
crates/oilwell-cli/Cargo.toml
crates/oilwell-cli/src/main.rs
crates/oilwell-cli/tests/convert_command.rs
src-tauri/{Cargo.toml,build.rs,tauri.conf.json,capabilities/default.json,src/lib.rs,src/main.rs}
ui/{package.json,tsconfig.json,vite.config.ts,index.html,src/main.tsx,src/App.tsx,src/api.ts,src/contracts.ts,src/styles/tokens.css,src/styles/app.css,src/components/*.tsx,src/test/setup.ts}
ui/src/**/*.test.tsx
samples/{survey,sections,bha,formation_tops}.csv
scripts/build-portable.ps1
tests/portable-smoke.ps1
README.md
LICENSES/README.txt
```

## Task 1: Create the independent workspace and typed conversion contracts

**Files:**
- Create: `Cargo.toml`
- Create: `crates/oilwell-core/Cargo.toml`
- Create: `crates/oilwell-core/src/lib.rs`
- Create: `crates/oilwell-core/src/contract.rs`
- Create: `crates/oilwell-core/src/error.rs`
- Create: `crates/oilwell-core/tests/contract.rs`
- Create: `README.md`

**Interfaces:**
- Consumes: no production interfaces.
- Produces: `ConversionRequest`, `ConversionResult`, `OutputFormat`, `InterpolationMethod`, `ColourPalette`, and `ConversionError` for every later task.

- [ ] **Step 1: Write the failing contract serialization test**

```rust
use oilwell_core::{ConversionError, ErrorCode, InterpolationMethod, OutputFormat};

#[test]
fn serializes_stable_public_enums() {
    assert_eq!(serde_json::to_string(&OutputFormat::Glb).unwrap(), "\"glb\"");
    assert_eq!(serde_json::to_string(&InterpolationMethod::MinimumCurvature).unwrap(), "\"minimum-curvature\"");
    let error = ConversionError::new(ErrorCode::MissingColumn, "Survey needs md", Some("md".into()), None);
    assert_eq!(serde_json::to_value(error).unwrap()["code"], "missing_column");
}
```

- [ ] **Step 2: Run the contract test to verify it fails because the crate does not exist**

Run: `cargo test -p oilwell-core --test contract`

Expected: FAIL with an unresolved package or missing test target.

- [ ] **Step 3: Create the workspace manifest and core manifest**

```toml
# Cargo.toml
[workspace]
resolver = "2"
members = ["crates/oilwell-core", "crates/oilwell-cli", "src-tauri"]

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"
```

```toml
# crates/oilwell-core/Cargo.toml
[package]
name = "oilwell-core"
version.workspace = true
edition.workspace = true

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
```

- [ ] **Step 4: Implement stable contracts and errors**

```rust
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat { Glb, Obj, Ply, Stl }

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum InterpolationMethod { MinimumCurvature, LinearDirectionBlend }

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConversionResult { pub output_path: String, pub wells: usize, pub formations: usize, pub meshes: usize, pub warnings: Vec<String> }

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode { InvalidInput, MissingColumn, InvalidNumber, NonIncreasingMd, InvalidDimensions, UnsupportedFormat, ReadFailed, WriteFailed, ValidationFailed, PublishFailed }
```

Define `ConversionError::new(code, message, field, source_path)` and implement `std::error::Error` through `thiserror` without exposing filesystem internals in the message.

- [ ] **Step 5: Run formatting and the contract test**

Run: `cargo fmt --check && cargo test -p oilwell-core --test contract`

Expected: PASS with stable kebab-case format/interpolation values and snake-case error code.

- [ ] **Step 6: Add the project boundary documentation and commit**

Document the Windows 11/WebView2 requirement, offline behavior, supported input tables, and explicit 3DMK non-linkage in `README.md`.

Run: `git add Cargo.toml Cargo.lock crates/oilwell-core README.md && git commit -m "feat: scaffold independent oilwell conversion core"`

## Task 2: Parse and validate CSV inputs and calculate trajectories

**Files:**
- Modify: `crates/oilwell-core/Cargo.toml`
- Create: `crates/oilwell-core/src/csv_input.rs`
- Create: `crates/oilwell-core/src/trajectory.rs`
- Modify: `crates/oilwell-core/src/lib.rs`
- Create: `crates/oilwell-core/tests/csv_input.rs`
- Create: `crates/oilwell-core/tests/trajectory.rs`

**Interfaces:**
- Consumes: Task 1 contract/error types.
- Produces: `ParsedTables`, `SurveyStation`, `TrajectoryPoint`, `parse_tables`, and `resample_trajectory` for Tasks 3–5.

- [ ] **Step 1: Write failing CSV ingress tests**

```rust
#[test]
fn accepts_bom_comments_and_blank_rows() {
    let survey = "\u{feff}well_id,md,inc_deg,azi_deg\n# ignored\n\nA1,0,0,0\nA1,100,10,90\n";
    let rows = oilwell_core::parse_survey_csv(survey.as_bytes()).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].well_id, "A1");
}

#[test]
fn rejects_missing_md_column_with_field_context() {
    let error = oilwell_core::parse_survey_csv(b"well_id,inc_deg,azi_deg\nA1,0,0\n").unwrap_err();
    assert_eq!(error.code, ErrorCode::MissingColumn);
    assert_eq!(error.field.as_deref(), Some("md"));
}
```

- [ ] **Step 2: Run the CSV tests to verify they fail because parser functions are absent**

Run: `cargo test -p oilwell-core --test csv_input`

Expected: FAIL with unresolved `parse_survey_csv`.

- [ ] **Step 3: Implement CSV parsing with explicit field conversion**

Use the `csv` crate with `trim(csv::Trim::All)`. Strip a UTF-8 BOM only from the first header field, discard wholly blank rows and first-field `#` comments, lower-case headers, and require the exact named columns. Parse finite `f64` values through one helper:

```rust
fn finite(value: &str, field: &str) -> Result<f64, ConversionError> {
    let parsed = value.parse::<f64>().map_err(|_| ConversionError::field(ErrorCode::InvalidNumber, format!("{field} must be numeric"), field))?;
    if parsed.is_finite() { Ok(parsed) } else { Err(ConversionError::field(ErrorCode::InvalidNumber, format!("{field} must be finite"), field)) }
}
```

- [ ] **Step 4: Run CSV tests to verify valid and invalid ingress behavior**

Run: `cargo test -p oilwell-core --test csv_input`

Expected: PASS.

- [ ] **Step 5: Write failing trajectory tests**

```rust
#[test]
fn rejects_duplicate_measured_depth() {
    let rows = vec![station(0.0, 0.0, 0.0), station(100.0, 10.0, 90.0), station(100.0, 20.0, 90.0)];
    assert_eq!(resample_trajectory(&rows, [0.0; 3], 25.0, InterpolationMethod::MinimumCurvature).unwrap_err().code, ErrorCode::NonIncreasingMd);
}

#[test]
fn md_zero_station_is_not_duplicated_or_non_finite() {
    let rows = vec![station(0.0, 0.0, 0.0), station(100.0, 10.0, 90.0)];
    let points = resample_trajectory(&rows, [0.0; 3], 25.0, InterpolationMethod::MinimumCurvature).unwrap();
    assert_eq!(points.iter().filter(|point| point.md == 0.0).count(), 1);
    assert!(points.iter().flat_map(|point| point.position).all(f64::is_finite));
}
```

- [ ] **Step 6: Run trajectory tests to verify they fail because resampling is absent**

Run: `cargo test -p oilwell-core --test trajectory`

Expected: FAIL with unresolved `resample_trajectory`.

- [ ] **Step 7: Implement directional vectors, slerp, minimum curvature, and linear blending**

Expose `resample_trajectory(rows, surface, smooth_step_md, method)`. Sort by MD, reject `current.md <= previous.md`, use exactly one MD-zero initial point, use normalized spherical interpolation for minimum curvature, use the ratio factor `2 * tan(dogleg / 2) / dogleg` outside the near-zero dogleg threshold, and use the documented linear direction blend only for `LinearDirectionBlend`.

- [ ] **Step 8: Run the core parser and trajectory suites**

Run: `cargo test -p oilwell-core --test csv_input --test trajectory`

Expected: PASS with finite positions and strictly increasing resampled MD.

- [ ] **Step 9: Commit**

Run: `git add crates/oilwell-core && git commit -m "feat: validate oilwell tables and trajectories"`

## Task 3: Generate meshes, write all output formats, and publish atomically

**Files:**
- Create: `crates/oilwell-core/src/mesh.rs`
- Create: `crates/oilwell-core/src/export.rs`
- Create: `crates/oilwell-core/src/publish.rs`
- Modify: `crates/oilwell-core/src/lib.rs`
- Create: `crates/oilwell-core/tests/export.rs`
- Create: `crates/oilwell-core/tests/publish.rs`

**Interfaces:**
- Consumes: Task 1 `ConversionRequest` and Task 2 parsed tables/trajectory.
- Produces: `convert(request) -> Result<ConversionResult, ConversionError>` for CLI and Tauri adapters.

- [ ] **Step 1: Write failing multi-format export tests**

```rust
#[test]
fn writes_valid_signatures_for_all_output_formats() {
    for (format, prefix) in [(OutputFormat::Glb, b"glTF".as_slice()), (OutputFormat::Ply, b"ply\n".as_slice())] {
        let bytes = export_fixture(format).unwrap();
        assert_eq!(&bytes[..prefix.len()], prefix);
    }
    assert!(String::from_utf8(export_fixture(OutputFormat::Obj).unwrap()).unwrap().contains("\nv "));
    assert_eq!(u32::from_le_bytes(export_fixture(OutputFormat::Stl).unwrap()[80..84].try_into().unwrap()), 1);
}

#[test]
fn glb_declares_finite_position_bounds_and_cad_orientation() {
    let json = glb_json(&export_fixture(OutputFormat::Glb).unwrap()).unwrap();
    assert_eq!(json["nodes"][0]["extras"]["source_coordinate_system"], "cad_z_up");
    assert!(json["accessors"].as_array().unwrap().iter().all(accessor_bounds_are_finite));
}
```

- [ ] **Step 2: Run export tests to verify they fail because exporters are absent**

Run: `cargo test -p oilwell-core --test export`

Expected: FAIL with unresolved `export_fixture` or `convert`.

- [ ] **Step 3: Implement indexed tube and formation meshes**

Represent a mesh as `name`, `positions: Vec<[f32; 3]>`, `normals: Vec<[f32; 3]>`, and `indices: Vec<u32>`. Generate tubular section/BHA geometry with 16 sides, validate all coordinates and normals are finite, create named formation grids with finite normals, and retain pad/well/component hierarchy data needed only by GLB.

- [ ] **Step 4: Implement GLB, OBJ, ASCII PLY, and binary STL writers**

GLB must write legal 4-byte aligned JSON and binary chunks, finite min/max position accessors, materials, named hierarchy, `cad_z_up` extras, and root quaternion `[-√1/2, 0, 0, √1/2]`. OBJ must emit vertices, normals, and indexed faces with running offsets. PLY must emit ASCII vertices/normals and triangular face indices. STL must emit an 80-byte header, little-endian facet count, finite facet normals, and 50-byte binary facets.

- [ ] **Step 5: Run export tests to verify all writer contracts**

Run: `cargo test -p oilwell-core --test export`

Expected: PASS.

- [ ] **Step 6: Write the failing atomic-publication test**

```rust
#[test]
fn leaves_existing_destination_unchanged_when_validation_fails() {
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("output.glb");
    std::fs::write(&output, b"previous bytes").unwrap();
    let error = publish_bytes(&output, b"bad", OutputFormat::Glb).unwrap_err();
    assert_eq!(error.code, ErrorCode::ValidationFailed);
    assert_eq!(std::fs::read(&output).unwrap(), b"previous bytes");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
```

- [ ] **Step 7: Run publication test to verify it fails because staging is absent**

Run: `cargo test -p oilwell-core --test publish`

Expected: FAIL with unresolved `publish_bytes`.

- [ ] **Step 8: Implement stage, validate, and publish**

Create a unique stage file in the output parent directory using `tempfile::NamedTempFile::new_in`. Validate signatures, declared GLB length, finite GLB accessor bounds, PLY header, OBJ text, or STL byte length before persist/rename. On any error, return `ValidationFailed`, `WriteFailed`, or `PublishFailed` and let `NamedTempFile` remove the staged file. Never pre-delete the requested output.

- [ ] **Step 9: Run all core tests and commit**

Run: `cargo fmt --check && cargo test -p oilwell-core`

Expected: PASS.

Run: `git add crates/oilwell-core Cargo.lock && git commit -m "feat: generate and publish oilwell model formats"`

## Task 4: Add the shared-core CLI executable

**Files:**
- Create: `crates/oilwell-cli/Cargo.toml`
- Create: `crates/oilwell-cli/src/main.rs`
- Create: `crates/oilwell-cli/tests/convert_command.rs`
- Modify: `README.md`

**Interfaces:**
- Consumes: Task 3 `convert(ConversionRequest)` and public contracts.
- Produces: `oilwell-converter-cli.exe convert` with human and JSON output.

- [ ] **Step 1: Write the failing CLI integration tests**

```rust
#[test]
fn emits_one_json_result_for_success() {
    let output = tempdir().unwrap().path().join("well.glb");
    let command = Command::cargo_bin("oilwell-converter-cli").unwrap()
        .args(["convert", "--survey", sample("survey.csv"), "--sections", sample("sections.csv"), "--output", output.to_str().unwrap(), "--format", "glb", "--json"])
        .assert().success();
    let value: serde_json::Value = serde_json::from_slice(&command.get_output().stdout).unwrap();
    assert_eq!(value["outputPath"], output.to_string_lossy().as_ref());
}

#[test]
fn emits_one_json_error_and_nonzero_exit_for_invalid_input() {
    Command::cargo_bin("oilwell-converter-cli").unwrap()
        .args(["convert", "--survey", sample("invalid-survey.csv"), "--sections", sample("sections.csv"), "--output", "bad.glb", "--format", "glb", "--json"])
        .assert().failure().stdout(predicate::str::contains("\"code\":\"missing_column\""));
}
```

- [ ] **Step 2: Run CLI tests to verify they fail because the binary is absent**

Run: `cargo test -p oilwell-converter-cli --test convert_command`

Expected: FAIL with missing package or binary.

- [ ] **Step 3: Implement Clap commands and JSON output mode**

Define `ConvertArgs` with `PathBuf` fields and `ValueEnum` values mapped to core enums. The default format is `glb`, default diameter scale `36.0`, default smooth step `75.0`, and default interpolation `minimum-curvature`. Send exactly one `ConversionResult` or serialized `ConversionError` to stdout under `--json`; send concise diagnostics to stderr and exit nonzero on error.

- [ ] **Step 4: Run CLI integration tests and a manual help check**

Run: `cargo test -p oilwell-converter-cli --test convert_command && cargo run -p oilwell-converter-cli -- convert --help`

Expected: PASS and help lists all path, geometry, format, and JSON options.

- [ ] **Step 5: Document CLI usage and commit**

Add the spec’s CLI command example and supported output semantics to `README.md`.

Run: `git add crates/oilwell-cli README.md Cargo.lock && git commit -m "feat: add oilwell conversion cli"`

## Task 5: Expose the narrow Tauri command and native dialogs

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`
- Create: `src-tauri/src/lib.rs`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/tests/command_contract.rs`

**Interfaces:**
- Consumes: Task 3 `convert(ConversionRequest)`.
- Produces: Tauri commands `convert_model`, `choose_input_file`, `choose_output_file`, and `reveal_output_folder`; no shell, process, or network command is registered.

- [ ] **Step 1: Write the failing Tauri command-contract tests**

```rust
#[test]
fn maps_core_field_error_without_losing_code_or_field() {
    let mapped = map_error(ConversionError::field(ErrorCode::MissingColumn, "Survey needs md", "md"));
    assert_eq!(mapped.code, ErrorCode::MissingColumn);
    assert_eq!(mapped.field.as_deref(), Some("md"));
}

#[test]
fn capability_manifest_excludes_shell_and_network_permissions() {
    let manifest = std::fs::read_to_string("capabilities/default.json").unwrap();
    assert!(!manifest.contains("shell:"));
    assert!(!manifest.contains("http:"));
}
```

- [ ] **Step 2: Run command tests to verify they fail because adapter files are absent**

Run: `cargo test -p oilwell-converter --test command_contract`

Expected: FAIL with missing package or test target.

- [ ] **Step 3: Configure Tauri and implement typed commands**

Use Tauri 2, `tauri-plugin-dialog`, and `tauri-plugin-opener` only. Define a `ConvertModelRequest` that uses the core enums and maps to `ConversionRequest`. Make `convert_model` async through `tauri::async_runtime::spawn_blocking`; convert core errors to a serializable `CommandError` with `code`, `message`, `field`, and `sourcePath`. Dialog commands must filter CSV input and the four valid output extensions.

- [ ] **Step 4: Run command tests and `cargo check`**

Run: `cargo test -p oilwell-converter --test command_contract && cargo check -p oilwell-converter`

Expected: PASS.

- [ ] **Step 5: Commit**

Run: `git add src-tauri Cargo.lock && git commit -m "feat: add native oilwell converter shell"`

## Task 6: Build the accessible React conversion workflow

**Files:**
- Create: `ui/package.json`
- Create: `ui/tsconfig.json`
- Create: `ui/vite.config.ts`
- Create: `ui/index.html`
- Create: `ui/src/main.tsx`
- Create: `ui/src/contracts.ts`
- Create: `ui/src/api.ts`
- Create: `ui/src/App.tsx`
- Create: `ui/src/styles/tokens.css`
- Create: `ui/src/styles/app.css`
- Create: `ui/src/components/FileField.tsx`
- Create: `ui/src/components/ExportPanel.tsx`
- Create: `ui/src/components/ResultPanel.tsx`
- Create: `ui/src/test/setup.ts`
- Create: `ui/src/App.test.tsx`

**Interfaces:**
- Consumes: Task 5 command names and `ConvertModelRequest`/result/error JSON shapes.
- Produces: a bundled, offline desktop UI with accessible input and result states.

- [ ] **Step 1: Write failing UI tests for primary and error workflows**

```tsx
it('submits selected inputs and renders the returned output summary', async () => {
  render(<App invoke={invokeSuccess} />);
  await userEvent.click(screen.getByRole('button', { name: 'Choose survey CSV' }));
  await userEvent.click(screen.getByRole('button', { name: 'Choose sections CSV' }));
  await userEvent.click(screen.getByRole('button', { name: 'Choose output location' }));
  await userEvent.click(screen.getByRole('button', { name: 'Export model' }));
  expect(await screen.findByText(/Exported 4 wells/)).toBeVisible();
});

it('keeps paths and maps a field error after a failed export', async () => {
  render(<App invoke={invokeMissingMd} />);
  await chooseRequiredPaths();
  await userEvent.click(screen.getByRole('button', { name: 'Export model' }));
  expect(await screen.findByText('Survey needs md')).toBeVisible();
  expect(screen.getByText(/survey.csv/)).toBeVisible();
});
```

- [ ] **Step 2: Run UI tests to verify they fail because the app is absent**

Run: `npm --prefix ui test -- --run`

Expected: FAIL with missing project configuration or test files.

- [ ] **Step 3: Create semantic UI tokens and components**

Create `tokens.css` with semantic `--color-*`, `--space-*`, `--radius-*`, `--motion-*`, and typography tokens for both light and dark themes. Implement labelled file fields with a drag/drop target and native choose button, numeric fields with explicit units, native select controls, an output-format radio group, palette inputs, and a single primary export action. Controls must have 44 px hit targets, focus-visible outlines, disabled/loading styles, and `prefers-reduced-motion` handling.

- [ ] **Step 4: Implement the typed Tauri API boundary and result states**

```ts
export type CommandError = { code: ErrorCode; message: string; field?: string; sourcePath?: string };
export async function convertModel(request: ConvertModelRequest): Promise<ConversionResult> {
  return invoke<ConversionResult>('convert_model', { request });
}
```

Use a discriminated export state: `idle`, `ready`, `exporting`, `success`, and `error`. Disable duplicate export while `exporting`; preserve settings on `error`; render warnings and accessible status text on `success`; map field errors beside the relevant control.

- [ ] **Step 5: Run type, lint, and UI tests**

Run: `npm --prefix ui run typecheck && npm --prefix ui run lint && npm --prefix ui test -- --run`

Expected: PASS.

- [ ] **Step 6: Commit**

Run: `git add ui && git commit -m "feat: add accessible desktop conversion workflow"`

## Task 7: Package the portable folder and verify the release path

**Files:**
- Create: `samples/survey.csv`
- Create: `samples/sections.csv`
- Create: `samples/bha.csv`
- Create: `samples/formation_tops.csv`
- Create: `scripts/build-portable.ps1`
- Create: `tests/portable-smoke.ps1`
- Create: `LICENSES/README.txt`
- Modify: `README.md`

**Interfaces:**
- Consumes: Task 4 CLI, Task 5 desktop executable, and Task 6 bundled frontend.
- Produces: `dist/windows-portable/` with both executables, samples, guidance, and licenses.

- [ ] **Step 1: Write the failing portable-folder smoke test**

```powershell
$distribution = Join-Path $PSScriptRoot '..\dist\windows-portable'
$cli = Join-Path $distribution 'oilwell-converter-cli.exe'
if (-not (Test-Path -LiteralPath $cli)) { throw 'Portable CLI executable is missing.' }
$output = Join-Path $TestDrive 'sample.glb'
& $cli convert --survey (Join-Path $distribution 'samples\survey.csv') --sections (Join-Path $distribution 'samples\sections.csv') --bha (Join-Path $distribution 'samples\bha.csv') --formations (Join-Path $distribution 'samples\formation_tops.csv') --output $output --format glb --json
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $output)) { throw 'Portable conversion failed.' }
if ([BitConverter]::ToUInt32([IO.File]::ReadAllBytes($output), 0) -ne 0x46546C67) { throw 'Output is not a GLB.' }
```

- [ ] **Step 2: Run the smoke test to verify it fails because distribution artifacts are absent**

Run: `pwsh -File tests/portable-smoke.ps1`

Expected: FAIL with `Portable CLI executable is missing.`

- [ ] **Step 3: Create sample files and portable build script**

`build-portable.ps1` must run frontend production build, `cargo tauri build --no-bundle`, `cargo build --release -p oilwell-converter-cli`, create `dist/windows-portable`, copy the desktop EXE as `Oilwell Converter.exe`, copy the CLI as `oilwell-converter-cli.exe`, copy samples, create `README.txt`, and copy `LICENSES`. It must fail on any missing source artifact and never download runtime dependencies.

- [ ] **Step 4: Build the production app and portable folder**

Run: `pwsh -File scripts/build-portable.ps1`

Expected: PASS and `dist/windows-portable` contains both executables, samples, `README.txt`, and `LICENSES`.

- [ ] **Step 5: Run the portable smoke test and complete suite**

Run: `pwsh -File tests/portable-smoke.ps1; cargo test --workspace; npm --prefix ui run typecheck; npm --prefix ui run lint; npm --prefix ui test -- --run`

Expected: PASS.

- [ ] **Step 6: Commit release instructions and packaging automation**

Run: `git add samples scripts tests LICENSES README.md && git commit -m "feat: package portable oilwell converter"`

## Plan self-review

### Spec coverage

- Independent repository and explicit non-linkage: Tasks 1 and 7.
- Shared Rust core for Tauri and CLI: Tasks 1, 3, 4, and 5.
- Typed ingress, stable outcomes, and explicit errors: Tasks 1, 2, 4, 5, and 6.
- CAD Z-up and four format exports: Task 3.
- Atomic publication/no partial replacement: Task 3.
- Offline, narrow Tauri permissions, and WebView2 boundary: Tasks 5 and 7.
- Style-system UI accessibility and meaningful states: Task 6.
- Core, CLI, UI, build, and portable smoke verification: Tasks 2–7.

### Placeholder scan

The plan contains concrete file paths, public interfaces, test examples, commands, expected outcomes, and implementation constraints for each task. It contains no unassigned implementation placeholders.

### Type consistency

`ConversionRequest`, `ConversionResult`, `ConversionError`, `OutputFormat`, `InterpolationMethod`, and `convert` are introduced by Tasks 1–3 and consumed under those exact names by Tasks 4–6. Tauri command `convert_model` maps to the same request/result model used by the UI API.

### Review-focus ownership

- BOM/comment/blank CSV ingestion: Task 2 CSV tests.
- Existing destination preservation on failed validation: Task 3 publication test.
- MD-zero station behavior: Task 2 trajectory test.
- One JSON error document and non-zero CLI exit: Task 4 CLI test.
- Retained UI input and mapped field error: Task 6 UI test.
