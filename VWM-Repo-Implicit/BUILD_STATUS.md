# Build and Verification Status

## Completed in the packaging environment

- Parsed every workspace `Cargo.toml` file.
- Verified workspace membership, unique package names, and local path dependencies.
- Checked Rust source delimiter balance and declared module files.
- Checked required crate files and prohibited placeholder patterns.
- Verified the final ZIP with CRC testing and source-to-extraction SHA-256 comparison.

## Not executed in the packaging environment

The packaging container does not include `cargo`, `rustc`, `rustfmt`, or Clippy, and it cannot resolve Cargo dependencies from crates.io. Consequently, this package does not claim a successful Rust compilation or test run in this environment.

Run the following from the repository root on the development machine:

```powershell
.\scripts\check.ps1
```

That script executes formatting validation, workspace compilation, tests, and Clippy with warnings denied.
