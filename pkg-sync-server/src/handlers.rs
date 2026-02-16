use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use globset::{Glob, GlobSetBuilder};
use pkg_sync_common::*;

use crate::auth::AuthenticatedMachine;
use crate::db;
use crate::AppState;

pub async fn submit_report(
    State(state): State<Arc<AppState>>,
    machine: AuthenticatedMachine,
    Json(report): Json<PackageReport>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let mut tx = state.db.begin().await.map_err(internal_error)?;

    // Update last_seen
    sqlx::query("UPDATE machines SET last_seen = datetime('now') WHERE id = ?")
        .bind(machine.id)
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;

    // Replace all packages for this machine
    sqlx::query("DELETE FROM packages WHERE machine_id = ?")
        .bind(machine.id)
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;

    for pkg in &report.packages {
        sqlx::query(
            "INSERT INTO packages (machine_id, name, version, source) VALUES (?, ?, ?, ?)",
        )
        .bind(machine.id)
        .bind(&pkg.name)
        .bind(&pkg.version)
        .bind(pkg.source.to_string())
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;
    }

    // Replace exclusion patterns
    sqlx::query("DELETE FROM exclusions WHERE machine_id = ?")
        .bind(machine.id)
        .execute(&mut *tx)
        .await
        .map_err(internal_error)?;

    for pattern in &report.exclusions {
        sqlx::query("INSERT INTO exclusions (machine_id, pattern) VALUES (?, ?)")
            .bind(machine.id)
            .bind(pattern)
            .execute(&mut *tx)
            .await
            .map_err(internal_error)?;
    }

    tx.commit().await.map_err(internal_error)?;

    tracing::info!(
        "Machine '{}' reported {} packages",
        machine.name,
        report.packages.len()
    );
    Ok(StatusCode::NO_CONTENT)
}

#[derive(serde::Deserialize)]
pub struct DiffParams {
    pub a: String,
    pub b: String,
}

pub async fn get_diff(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DiffParams>,
) -> Result<Json<DiffResponse>, (StatusCode, Json<ApiError>)> {
    let machine_a_id = db::get_machine_by_name(&state.db, &params.a)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(&format!("Machine '{}' not found", params.a)))?;

    let machine_b_id = db::get_machine_by_name(&state.db, &params.b)
        .await
        .map_err(internal_error)?
        .ok_or_else(|| not_found(&format!("Machine '{}' not found", params.b)))?;

    // Fetch packages for both machines
    let packages_a = fetch_packages(&state.db, machine_a_id)
        .await
        .map_err(internal_error)?;
    let packages_b = fetch_packages(&state.db, machine_b_id)
        .await
        .map_err(internal_error)?;

    // Fetch exclusions from both machines and combine them
    let exclusions_a = fetch_exclusions(&state.db, machine_a_id)
        .await
        .map_err(internal_error)?;
    let exclusions_b = fetch_exclusions(&state.db, machine_b_id)
        .await
        .map_err(internal_error)?;

    let mut builder = GlobSetBuilder::new();
    for pattern in exclusions_a.iter().chain(exclusions_b.iter()) {
        if let Ok(glob) = Glob::new(pattern) {
            builder.add(glob);
        }
    }
    let exclusion_set = builder.build().map_err(|e| internal_error(e))?;

    // Build maps, filtering out excluded packages
    let map_a: HashMap<&str, &PackageInfo> = packages_a
        .iter()
        .filter(|p| !exclusion_set.is_match(&p.name))
        .map(|p| (p.name.as_str(), p))
        .collect();

    let map_b: HashMap<&str, &PackageInfo> = packages_b
        .iter()
        .filter(|p| !exclusion_set.is_match(&p.name))
        .map(|p| (p.name.as_str(), p))
        .collect();

    let mut only_in_a = Vec::new();
    let mut only_in_b = Vec::new();
    let mut version_diffs = Vec::new();

    for (name, pkg) in &map_a {
        match map_b.get(name) {
            None => only_in_a.push((*pkg).clone()),
            Some(other) => {
                if pkg.version != other.version {
                    version_diffs.push(VersionDiff {
                        package: name.to_string(),
                        version_a: pkg.version.clone(),
                        version_b: other.version.clone(),
                    });
                }
            }
        }
    }

    for (name, pkg) in &map_b {
        if !map_a.contains_key(name) {
            only_in_b.push((*pkg).clone());
        }
    }

    // Sort for stable output
    only_in_a.sort_by(|a, b| a.name.cmp(&b.name));
    only_in_b.sort_by(|a, b| a.name.cmp(&b.name));
    version_diffs.sort_by(|a, b| a.package.cmp(&b.package));

    Ok(Json(DiffResponse {
        machine_a: params.a,
        machine_b: params.b,
        only_in_a,
        only_in_b,
        version_differences: version_diffs,
    }))
}

pub async fn list_machines(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<MachineInfo>>, (StatusCode, Json<ApiError>)> {
    let rows = sqlx::query_as::<_, (String, i64, Option<String>)>(
        "SELECT m.name, COUNT(p.id), m.last_seen
         FROM machines m
         LEFT JOIN packages p ON p.machine_id = m.id
         GROUP BY m.id",
    )
    .fetch_all(&state.db)
    .await
    .map_err(internal_error)?;

    let machines = rows
        .into_iter()
        .map(|(name, count, last_seen)| MachineInfo {
            name,
            package_count: count,
            last_seen,
        })
        .collect();

    Ok(Json(machines))
}

async fn fetch_packages(
    pool: &sqlx::SqlitePool,
    machine_id: i64,
) -> anyhow::Result<Vec<PackageInfo>> {
    let rows = sqlx::query_as::<_, (String, String, String)>(
        "SELECT name, version, source FROM packages WHERE machine_id = ?",
    )
    .bind(machine_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|(name, version, source)| PackageInfo {
            name,
            version,
            source: if source == "aur" {
                PackageSource::Aur
            } else {
                PackageSource::Repo
            },
        })
        .collect())
}

async fn fetch_exclusions(pool: &sqlx::SqlitePool, machine_id: i64) -> anyhow::Result<Vec<String>> {
    let rows =
        sqlx::query_scalar::<_, String>("SELECT pattern FROM exclusions WHERE machine_id = ?")
            .bind(machine_id)
            .fetch_all(pool)
            .await?;

    Ok(rows)
}

fn internal_error(e: impl std::fmt::Display) -> (StatusCode, Json<ApiError>) {
    tracing::error!("{}", e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ApiError {
            error: "Internal server error".to_string(),
        }),
    )
}

fn not_found(msg: &str) -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            error: msg.to_string(),
        }),
    )
}
