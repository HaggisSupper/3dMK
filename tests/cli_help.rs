use std::process::Command;

#[test]
fn cli_shows_help() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-cad-backend"))
        .args(["--help"])
        .output()
        .expect("failed to run compiled CLI");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pdf-to3d"));
    assert!(stdout.contains("poisson-reconstruct"));
}
