use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_files(&entry.path(), files)?;
        } else if kind.is_file() {
            files.push(entry.path());
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Frontend assets must be regular files",
            ));
        }
    }
    Ok(())
}

fn main() {
    let public = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest directory"))
        .join("../public");
    println!("cargo:rerun-if-changed={}", public.display());
    let mut files = Vec::new();
    collect_files(&public, &mut files).expect("read frontend assets");
    files.sort();
    assert!(!files.is_empty(), "Frontend assets are missing");
    let mut generated = String::from("const EMBEDDED_FRONTEND_ASSETS: &[(&str, &[u8])] = &[\n");
    for file in files {
        let relative = file
            .strip_prefix(&public)
            .expect("public asset")
            .to_str()
            .expect("UTF-8 asset path")
            .replace('\\', "/");
        let absolute = file.canonicalize().expect("asset identity");
        generated.push_str(&format!(
            "({relative:?}, include_bytes!({:?})),\n",
            absolute.to_str().expect("UTF-8 absolute path")
        ));
    }
    generated.push_str("];\n");
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("build output"));
    fs::write(out.join("frontend_assets.rs"), generated)
        .expect("write embedded frontend asset table");
    tauri_build::build();
}
