use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pkg_sync_common::PackageReport;

mod config;
mod pacman;

#[derive(Parser)]
#[command(name = "pkg-sync-client")]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "/etc/pkg-sync/config.toml")]
    config: PathBuf,

    /// Collect and print packages without sending to server
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load(&cli.config)?;

    let packages = pacman::collect_packages()?;
    println!("Collected {} packages", packages.len());

    if cli.dry_run {
        for pkg in &packages {
            println!("  {} {} ({})", pkg.name, pkg.version, pkg.source);
        }
        if !config.exclude.patterns.is_empty() {
            println!("\nExclusion patterns:");
            for p in &config.exclude.patterns {
                println!("  {}", p);
            }
        }
        return Ok(());
    }

    let report = PackageReport {
        packages,
        exclusions: config.exclude.patterns.clone(),
    };

    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/v1/report",
        config.server.url.trim_end_matches('/')
    );

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.server.api_key))
        .json(&report)
        .send()
        .await?;

    if response.status().is_success() {
        println!("Successfully reported packages to server");
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Server returned {}: {}", status, body);
    }

    Ok(())
}
