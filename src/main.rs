use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

fn main() -> Result<()> {
    let compare_dir = PathBuf::from(".compare");
    let before_dir = compare_dir.join("before");
    let after_dir = compare_dir.join("after");

    build_before_site(&before_dir).context("failed to build before site")?;
    build_after_site(&after_dir).context("failed to build after site")?;

    Ok(())
}

fn build_before_site(output_dir: &Path) -> Result<()> {
    let status = Command::new("nix-shell")
        .args(["--command"])
        .arg(format!(
            "zola build --output-dir {}",
            output_dir.to_string_lossy()
        ))
        .status()?;
    if !status.success() {
        bail!("failed with status: {status}");
    }

    Ok(())
}

fn build_after_site(output_dir: &Path) -> Result<()> {
    let status = Command::new("cargo")
        .args(["run", "--package", "site", "--", "build", "--output-dir"])
        .arg(output_dir)
        .status()?;
    if !status.success() {
        bail!("failed with status: {status}");
    }

    Ok(())
}
