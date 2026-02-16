use std::collections::HashSet;
use std::process::Command;

use anyhow::{Context, Result};
use pkg_sync_common::{PackageInfo, PackageSource};

pub fn collect_packages() -> Result<Vec<PackageInfo>> {
    // Get explicitly installed packages with versions
    let output = run_pacman(&["-Qe"])?;

    // Get foreign (AUR) packages — non-fatal if none exist
    let foreign: HashSet<String> = run_pacman(&["-Qqm"])
        .unwrap_or_default()
        .into_iter()
        .collect();

    let packages = output
        .into_iter()
        .filter_map(|line| {
            let (name, version) = line.split_once(' ')?;
            let source = if foreign.contains(name) {
                PackageSource::Aur
            } else {
                PackageSource::Repo
            };
            Some(PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                source,
            })
        })
        .collect();

    Ok(packages)
}

fn run_pacman(args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new("pacman")
        .args(args)
        .output()
        .context("Failed to run pacman")?;

    if !output.status.success() {
        anyhow::bail!(
            "pacman {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8(output.stdout)?
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}
