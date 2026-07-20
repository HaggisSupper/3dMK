use std::process::Command;

#[test]
fn cli_shows_help() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--help"])
        .output()
        .expect("failed to run cargo");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pdf-to3d"));
    assert!(stdout.contains("poisson-reconstruct"));
}
