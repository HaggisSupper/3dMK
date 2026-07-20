use std::process::Command;

#[test]
fn pdf_to_3d_missing_image_errors() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "pdf-to3d"])
        .output()
        .expect("failed to run cargo");
    assert!(!output.status.success());
}

#[test]
fn poisson_reconstruct_missing_input_errors() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "poisson-reconstruct"])
        .output()
        .expect("failed to run cargo");
    assert!(!output.status.success());
}
