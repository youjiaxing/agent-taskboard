use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::tracker::TrackerKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInference {
    pub name: String,
    pub local_path: PathBuf,
    pub tracker: TrackerKind,
    pub github_host: String,
    pub repository: String,
}

pub(crate) fn normalize_github_host(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok("github.com".into());
    }
    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if host.is_empty() || host.contains(' ') {
        return Err("github host is invalid".into());
    }
    Ok(host.to_ascii_lowercase())
}

pub(crate) fn normalize_repository(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/').trim_end_matches(".git");
    if Path::new(trimmed).is_absolute() {
        return Ok(trimmed.to_string());
    }
    if let Some((owner, repo)) = owner_repo(trimmed) {
        return Ok(format!("{owner}/{repo}"));
    }
    Err("repository must be owner/repo".into())
}

pub(crate) fn require_local_directory(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("missing localPath".into());
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err("local directory must be an absolute path".into());
    }
    let meta = fs::metadata(&path).map_err(|_| "local directory does not exist".to_string())?;
    if !meta.is_dir() {
        return Err("local directory does not exist".into());
    }
    Ok(path)
}

pub(crate) fn same_local_directory(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

pub(crate) fn require_name(raw: &str) -> Result<String, String> {
    let name = raw.trim();
    if name.is_empty() {
        return Err("missing name".into());
    }
    Ok(name.to_string())
}

pub(crate) fn infer_github_project(local_path: &Path) -> Option<ProjectInference> {
    let name = local_path.file_name()?.to_string_lossy().trim().to_string();
    if name.is_empty() {
        return None;
    }
    if has_local_markdown_tracker(local_path) {
        return Some(ProjectInference {
            name,
            local_path: local_path.to_path_buf(),
            tracker: TrackerKind::LocalMarkdown,
            github_host: "local".into(),
            repository: local_path.to_string_lossy().to_string(),
        });
    }
    let config = git_config_path(local_path)?;
    let text = fs::read_to_string(config).ok()?;
    let (github_host, repository) = github_remote_from_config(&text)?;
    Some(ProjectInference {
        name,
        local_path: local_path.to_path_buf(),
        tracker: TrackerKind::Github,
        github_host,
        repository,
    })
}

fn has_local_markdown_tracker(local_path: &Path) -> bool {
    let scratch = local_path.join(".scratch");
    let Ok(features) = fs::read_dir(scratch) else {
        return false;
    };
    features.flatten().any(|feature| {
        fs::read_dir(feature.path().join("issues"))
            .ok()
            .is_some_and(|entries| {
                entries.flatten().any(|entry| {
                    entry.path().extension().is_some_and(|ext| ext == "md")
                        && entry
                            .path()
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .and_then(|stem| stem.split_once('-'))
                            .and_then(|(number, _)| number.parse::<u64>().ok())
                            .is_some()
                })
            })
    })
}

fn git_config_path(dir: &Path) -> Option<PathBuf> {
    let git = dir.join(".git");
    if git.is_dir() {
        return Some(git.join("config"));
    }
    if git.is_file() {
        let text = fs::read_to_string(&git).ok()?;
        for line in text.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("gitdir:") {
                let gitdir = rest.trim();
                let path = Path::new(gitdir);
                let resolved = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    dir.join(path)
                };
                return Some(resolved.join("config"));
            }
        }
    }
    None
}

fn github_remote_from_config(text: &str) -> Option<(String, String)> {
    let mut current_remote = None;
    let mut origin = None;
    let mut first = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = remote_section(trimmed) {
            current_remote = Some(name);
            continue;
        }
        if trimmed.starts_with('[') {
            current_remote = None;
            continue;
        }
        let Some(remote) = current_remote.as_deref() else {
            continue;
        };
        let Some(url) = remote_url(trimmed) else {
            continue;
        };
        let Some(parsed) = parse_github_remote_url(&url) else {
            continue;
        };
        if remote == "origin" {
            origin = Some(parsed);
        } else if first.is_none() {
            first = Some(parsed);
        }
    }
    origin.or(first)
}

fn remote_section(line: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix("[remote \"")?.strip_suffix("\"]")?;
    Some(rest.to_string())
}

fn remote_url(line: &str) -> Option<String> {
    let (key, value) = line.split_once('=')?;
    if key.trim() != "url" {
        return None;
    }
    Some(value.trim().to_string())
}

fn parse_github_remote_url(url: &str) -> Option<(String, String)> {
    let url = url.trim().trim_end_matches('/');
    let url = url.strip_suffix(".git").unwrap_or(url);
    if let Some(rest) = url.strip_prefix("git@") {
        let (host, repo) = rest.split_once(':')?;
        return github_host_and_repo(host, repo);
    }
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("ssh://git@"))
        .or_else(|| url.strip_prefix("ssh://"))?;
    let rest = rest.strip_prefix("git@").unwrap_or(rest);
    let (host, repo) = rest.split_once('/')?;
    github_host_and_repo(host, repo)
}

fn github_host_and_repo(host: &str, repo: &str) -> Option<(String, String)> {
    let host = host
        .trim()
        .trim_start_matches('[')
        .split(']')
        .next()
        .unwrap_or(host)
        .split(':')
        .next()
        .unwrap_or(host)
        .trim();
    if !is_github_host(host) {
        return None;
    }
    let (owner, name) = owner_repo(repo)?;
    Some((host.to_ascii_lowercase(), format!("{owner}/{name}")))
}

fn is_github_host(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "github.com" || host.starts_with("github.") || host.contains(".github.")
}

fn owner_repo(raw: &str) -> Option<(&str, &str)> {
    let raw = raw.trim().trim_start_matches('/');
    let (owner, rest) = raw.split_once('/')?;
    let repo = rest.split('/').next().unwrap_or(rest);
    if owner.is_empty()
        || repo.is_empty()
        || owner.contains(' ')
        || repo.contains(' ')
        || owner.contains('\\')
        || repo.contains('\\')
    {
        return None;
    }
    Some((owner, repo))
}
