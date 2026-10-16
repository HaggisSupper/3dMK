use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, Context, Result};

pub fn convert_to_glb(input: &Path) -> Result<PathBuf> {
    let output = input.with_extension("glb");

    let status = Command::new("assimp")
        .args([
            "export",
            input
                .to_str()
                .ok_or_else(|| anyhow!("invalid input path"))?,
            output
                .to_str()
                .ok_or_else(|| anyhow!("invalid output path"))?,
        ])
        .status()
        .with_context(|| "failed to launch assimp conversion command")?;

    if !status.success() {
        return Err(anyhow!("assimp conversion failed with status {status}"));
    }

    Ok(output)
}
