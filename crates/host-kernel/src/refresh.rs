use std::collections::BTreeMap;
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
    /// 读取是否完整；不完整时不能当作全量数据计算 Frontier/依赖图。
    #[serde(default = "default_complete")]
    pub complete: bool,
    /// 不完整读取的可读详情（例如截断原因）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub issues: Vec<IssueRecord>,
    /// 只保存实际打开并成功读取过的正文，不把列表刷新变成全量正文下载。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub documents: BTreeMap<String, StoredIssueDocument>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredIssueDocument {
    pub body: String,
    pub fetched_at_ms: u64,
}

fn default_complete() -> bool {
    true
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

pub fn remove_project_data(host_dir: &Path, project_id: &str) -> io::Result<()> {
    let path = host_dir.join("projects").join(project_id);
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}
