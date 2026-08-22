use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::issue::IssueRecord;
use crate::owner;

pub const DEFAULT_REFRESH_INTERVAL_MS: u64 = 60_000;
pub const MIN_REFRESH_INTERVAL_MS: u64 = 15_000;
pub const MAX_REFRESH_INTERVAL_MS: u64 = 600_000;

pub fn clamp_refresh_interval_ms(interval_ms: u64) -> u64 {
    if interval_ms == 0 {
        DEFAULT_REFRESH_INTERVAL_MS
    } else {
        interval_ms.clamp(MIN_REFRESH_INTERVAL_MS, MAX_REFRESH_INTERVAL_MS)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTrackerSnapshot {
    pub fetched_at_ms: u64,
    pub issues: Vec<IssueRecord>,
}

pub fn wall_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

pub fn snapshot_path(host_dir: &Path, project_id: &str) -> PathBuf {
    host_dir
        .join("projects")
        .join(project_id)
        .join("tracker-snapshot")
}

pub fn load_snapshot(path: &Path) -> Option<StoredTrackerSnapshot> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn save_snapshot(path: &Path, snapshot: &StoredTrackerSnapshot) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let body = serde_json::to_vec_pretty(snapshot)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    let tmp = path.with_extension("json.tmp");
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&body)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
    }
    owner::replace_file(&tmp, path)
}

pub fn remove_project_data(host_dir: &Path, project_id: &str) {
    let _ = fs::remove_dir_all(host_dir.join("projects").join(project_id));
}
