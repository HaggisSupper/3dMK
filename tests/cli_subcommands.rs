use std::process::Command;

#[test]
fn pdf_to_3d_missing_image_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-cad-backend"))
        .args(["pdf-to3d"])
        .output()
        .expect("failed to run compiled CLI");
    assert!(!output.status.success());
}

#[test]
fn poisson_reconstruct_missing_input_errors() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentic-cad-backend"))
        .args(["poisson-reconstruct"])
        .output()
        .expect("failed to run compiled CLI");
    assert!(!output.status.success());
}
