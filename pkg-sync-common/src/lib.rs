use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PackageSource {
    Repo,
    Aur,
}

impl std::fmt::Display for PackageSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackageSource::Repo => write!(f, "repo"),
            PackageSource::Aur => write!(f, "aur"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageReport {
    pub packages: Vec<PackageInfo>,
    pub exclusions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResponse {
    pub machine_a: String,
    pub machine_b: String,
    pub only_in_a: Vec<PackageInfo>,
    pub only_in_b: Vec<PackageInfo>,
    pub version_differences: Vec<VersionDiff>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionDiff {
    pub package: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MachineInfo {
    pub name: String,
    pub package_count: i64,
    pub last_seen: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
}
