mod report;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

use anyhow::{bail, Context, Result};
use clap::Parser;
use walkdir::WalkDir;

use crate::report::render_report;

enum Difference {
    Added,
    Changed { before: String, after: String },
    Removed,
}

struct Comparison {
    pub identical: BTreeSet<String>,
    pub differences: BTreeMap<String, Difference>,
}

#[derive(Parser)]
struct Args {
    /// Whether to open the report in the browser after running.
    #[clap(long)]
    open: bool,
}

fn main() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Args::parse();

    let compare_dir = PathBuf::from(".compare");
    let before_dir = compare_dir.join("before");
    let after_dir = compare_dir.join("after");
    let both_dirs = [&before_dir, &after_dir];

    for output_dir in &both_dirs {
        log::info!("Removing output directory {output_dir:?}");
        if let Err(err) = fs::remove_dir_all(output_dir) {
            if err.kind() != io::ErrorKind::NotFound {
                return Err(err.into());
            }
        }
    }

    log::info!("Building before site");
    build_before_site(&before_dir).context("failed to build before site")?;

    log::info!("Building after site");
    build_after_site(&after_dir).context("failed to build after site")?;

    for output_dir in &both_dirs {
        log::info!("Formatting {output_dir:?} with Prettier");
        format_with_prettier(output_dir)
            .with_context(|| format!("failed to format {output_dir:?} with Prettier"))?;
    }

    log::info!("Collecting before site files");
    let before_site = collect_files(&before_dir).context("failed to collect before site files")?;

    log::info!("Collecting after site files");
    let after_site = collect_files(&after_dir).context("failed to collect after site files")?;

    log::info!("Comparing before and after");
    let comparison = compare_sites(before_site, after_site)?;

    log::info!("Generating report");
    let report = render_report(comparison).context("failed to render report")?;

    let report_path = compare_dir.join("report.html");
    fs::write(&report_path, report).context("failed to write report to file")?;
    log::info!("Report written to {:?}", report_path);

    if args.open {
        opener::open(report_path)?;
    }

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

fn format_with_prettier(dir: &Path) -> Result<()> {
    let status = Command::new("pnpm")
        .arg("prettier")
        .arg(dir)
        .arg("--write")
        // Since the comparison directory is ignored by Git, Prettier ignores it
        // too, unless we tell it otherwise.
        .arg("--ignore-path")
        .status()?;
    if !status.success() {
        bail!("failed with status: {status}");
    }

    Ok(())
}

fn collect_files(dir: &Path) -> Result<BTreeMap<String, String>> {
    let walker = WalkDir::new(dir).into_iter();

    let mut files = BTreeMap::new();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();

        let Some(filename) = entry.file_name().to_str() else {
            log::warn!("{entry:?} does not have a filename");
            continue;
        };

        if !path.is_dir() {
            if filename.ends_with(".png") || filename.ends_with(".ico") {
                log::warn!("Skipping file: {path:?}");
                continue;
            }

            let contents = fs::read_to_string(&path)
                .with_context(|| format!("failed to read to string: {path:?}"))?;
            let path = path.strip_prefix(dir)?.to_string_lossy().to_string();
            let path = format!("/{path}");

            files.insert(path, contents);
        } else {
        }
    }

    Ok(files)
}

fn compare_sites(
    before: BTreeMap<String, String>,
    after: BTreeMap<String, String>,
) -> Result<Comparison> {
    let mut identical = BTreeSet::new();
    let mut differences = BTreeMap::new();

    for (path, before_content) in before.iter() {
        match after.get(path) {
            Some(after_content) => {
                if after_content != before_content {
                    differences.insert(
                        path.clone(),
                        Difference::Changed {
                            before: before_content.clone(),
                            after: after_content.clone(),
                        },
                    );
                } else {
                    identical.insert(path.clone());
                }
            }
            None => {
                differences.insert(path.clone(), Difference::Removed);
            }
        }
    }

    for path in after.keys() {
        if !before.contains_key(path) {
            differences.insert(path.clone(), Difference::Added);
        }
    }

    Ok(Comparison {
        identical,
        differences,
    })
}
