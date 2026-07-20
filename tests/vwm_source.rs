use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn runtime_vwm_crates_resolve_from_one_canonical_tree() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("repository root should exist");
    let canonical = root
        .join("VWM-Repo-Implicit")
        .canonicalize()
        .expect("canonical VWM source should exist");
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version=1"])
        .current_dir(&root)
        .output()
        .expect("cargo metadata should run");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata should be JSON");
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata packages should be an array");
    let mut names = BTreeSet::new();

    for package in packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if !name.starts_with("vwm-") {
            continue;
        }
        names.insert(name.to_owned());
        let manifest = Path::new(
            package["manifest_path"]
                .as_str()
                .expect("VWM package should report a manifest path"),
        )
        .canonicalize()
        .expect("VWM manifest path should exist");
        assert!(
            manifest.starts_with(&canonical),
            "{name} resolved outside canonical VWM tree: {}",
            manifest.display()
        );
    }

    for required in ["vwm-core", "vwm-io", "vwm-geometry"] {
        assert!(names.contains(required), "missing runtime crate {required}");
    }
}
