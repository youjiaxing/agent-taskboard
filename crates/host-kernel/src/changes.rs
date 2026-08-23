use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::pairing;
use crate::{Language, RunSummary};

pub const MAX_REPO_DEPTH: usize = 3;
const MAX_FILE_BYTES: u64 = 1_048_576;
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

const SKIP_DIRS: &[&str] = &[
    ".cache",
    ".direnv",
    ".git",
    ".next",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "Pods",
    "target",
    "venv",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeScope {
    ThisRound,
    Uncommitted,
}

impl ChangeScope {
    pub fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.unwrap_or("this-round").trim() {
            "" | "this-round" => Ok(Self::ThisRound),
            "uncommitted" => Ok(Self::Uncommitted),
            other => Err(format!("unknown change scope {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBaseline {
    pub path: String,
    pub display_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeNote {
    pub id: String,
    pub run_id: String,
    pub project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub repo: String,
    pub path: String,
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeLineKind {
    Context,
    Add,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeLine {
    pub kind: ChangeLineKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_line: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeHunk {
    pub header: String,
    pub lines: Vec<ChangeLine>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeFile {
    pub path: String,
    pub hunks: Vec<ChangeHunk>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeRepo {
    pub path: String,
    pub display_path: String,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_commit: Option<String>,
    #[serde(default)]
    pub files: Vec<ChangeFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewChanges {
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub working_directory: String,
    pub isolated: bool,
    pub scope: ChangeScope,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
    #[serde(default)]
    pub repos: Vec<ChangeRepo>,
    #[serde(default)]
    pub notes: Vec<ChangeNote>,
}

pub fn isolated_tree_gone(language: Language) -> String {
    match language {
        Language::ZhCn => "隔离执行目录已经不在，不能查看改动。".into(),
        Language::En => "The isolated work directory is gone, so changes cannot be viewed.".into(),
    }
}

pub fn nested_repo_gone(language: Language) -> String {
    match language {
        Language::ZhCn => "这个子仓库已经不在工作目录里。".into(),
        Language::En => "This nested repository is no longer in the work directory.".into(),
    }
}

pub fn no_git_repo(language: Language) -> String {
    match language {
        Language::ZhCn => "这个工作目录里找不到 git 仓库。".into(),
        Language::En => "No git repository was found in this work directory.".into(),
    }
}

pub fn working_dir_gone(language: Language) -> String {
    match language {
        Language::ZhCn => "工作目录已经不在，不能查看改动。".into(),
        Language::En => "The work directory is gone, so changes cannot be viewed.".into(),
    }
}

pub fn appeared_after_start(language: Language) -> String {
    match language {
        Language::ZhCn => "这个仓库是启动后才出现的，只有未提交。".into(),
        Language::En => {
            "This repository appeared after the Run started, so only uncommitted changes are available.".into()
        }
    }
}

pub fn record_baselines(root: &Path) -> Vec<GitBaseline> {
    discover_repos(root)
        .into_iter()
        .map(|path| GitBaseline {
            display_path: display_path(root, &path),
            commit: git_head(&path),
            path: path.to_string_lossy().into_owned(),
        })
        .collect()
}

pub fn compute_view(
    run: &RunSummary,
    scope: ChangeScope,
    notes: &[ChangeNote],
    language: Language,
) -> ViewChanges {
    let working = PathBuf::from(&run.working_directory);
    let mut view = ViewChanges {
        run_id: run.id.clone(),
        issue_id: run.issue_id.clone(),
        working_directory: run.working_directory.clone(),
        isolated: run.isolated,
        scope,
        available: true,
        unavailable_reason: None,
        repos: Vec::new(),
        notes: pending_notes(notes, &run.project_id, run.issue_id.as_deref()),
    };
    if run.working_directory.is_empty() || !working.exists() {
        view.available = false;
        view.unavailable_reason = Some(if run.isolated {
            isolated_tree_gone(language)
        } else {
            working_dir_gone(language)
        });
        return view;
    }

    let current = discover_repos(&working);
    let mut seen = Vec::new();
    for baseline in &run.git_baselines {
        let path = PathBuf::from(&baseline.path);
        let repo = if path.exists() && is_git_repo(&path) {
            diff_repo(
                &path,
                &baseline.display_path,
                baseline.commit.as_deref(),
                scope,
            )
        } else {
            ChangeRepo {
                path: baseline.path.clone(),
                display_path: baseline.display_path.clone(),
                available: false,
                unavailable_reason: Some(if baseline.display_path == "." {
                    working_dir_gone(language)
                } else {
                    nested_repo_gone(language)
                }),
                start_commit: baseline.commit.clone(),
                files: Vec::new(),
            }
        };
        seen.push(canonical(&path).unwrap_or(path));
        view.repos.push(repo);
    }
    for path in current {
        let key = canonical(&path).unwrap_or_else(|| path.clone());
        if seen.iter().any(|seen| same_path(seen, &key)) {
            continue;
        }
        let display = display_path(&working, &path);
        let repo = match scope {
            ChangeScope::ThisRound => ChangeRepo {
                path: path.to_string_lossy().into_owned(),
                display_path: display,
                available: false,
                unavailable_reason: Some(appeared_after_start(language)),
                start_commit: None,
                files: Vec::new(),
            },
            ChangeScope::Uncommitted => diff_repo(&path, &display, None, scope),
        };
        view.repos.push(repo);
    }
    if view.repos.is_empty() {
        view.available = false;
        view.unavailable_reason = Some(no_git_repo(language));
    }
    view
}

pub fn format_notes(notes: &[ChangeNote]) -> String {
    notes.iter().map(format_note).collect::<Vec<_>>().join("\n")
}

pub fn format_note(note: &ChangeNote) -> String {
    let location = if note.repo == "." || note.repo.is_empty() {
        format!("{}:{}", note.path, note.line)
    } else {
        format!("{}/{}:{}", note.repo, note.path, note.line)
    };
    format!("{location} {}", note.text.trim())
}

pub fn append_notes(opening: &str, notes: &[ChangeNote]) -> String {
    let formatted = format_notes(notes);
    if formatted.is_empty() {
        return opening.to_string();
    }
    if opening.trim().is_empty() {
        formatted
    } else {
        format!("{}\n\n{formatted}", opening.trim_end())
    }
}

pub fn pending_notes(
    notes: &[ChangeNote],
    project_id: &str,
    issue_id: Option<&str>,
) -> Vec<ChangeNote> {
    notes
        .iter()
        .filter(|note| note.project_id == project_id && note.issue_id.as_deref() == issue_id)
        .cloned()
        .collect()
}

pub fn new_note(
    run: &RunSummary,
    repo: String,
    path: String,
    line: u32,
    text: String,
) -> ChangeNote {
    ChangeNote {
        id: pairing::random_id(),
        run_id: run.id.clone(),
        project_id: run.project_id.clone(),
        issue_id: run.issue_id.clone(),
        repo,
        path,
        line,
        text,
    }
}

fn diff_repo(
    repo: &Path,
    display_path: &str,
    start_commit: Option<&str>,
    scope: ChangeScope,
) -> ChangeRepo {
    let mut repo_view = ChangeRepo {
        path: repo.to_string_lossy().into_owned(),
        display_path: display_path.to_string(),
        available: true,
        unavailable_reason: None,
        start_commit: start_commit.map(ToOwned::to_owned),
        files: Vec::new(),
    };
    match scope {
        ChangeScope::ThisRound => {
            let spec = start_commit
                .filter(|value| !value.is_empty())
                .unwrap_or(EMPTY_TREE);
            repo_view.files = diff_against(repo, spec);
        }
        ChangeScope::Uncommitted => {
            if git_head(repo).is_none() {
                repo_view.files = untracked_files(repo);
            } else {
                repo_view.files = diff_against(repo, "HEAD");
            }
        }
    }
    repo_view
}

fn diff_against(repo: &Path, spec: &str) -> Vec<ChangeFile> {
    let mut files = parse_diff(&git_stdout(
        repo,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--find-renames",
            "-U3",
            spec,
        ],
    ));
    merge_files(&mut files, untracked_files(repo));
    files
}

fn untracked_files(repo: &Path) -> Vec<ChangeFile> {
    git_stdout(repo, &["ls-files", "--others", "--exclude-standard"])
        .lines()
        .filter(|line| !line.is_empty())
        .filter_map(|rel| untracked_file(repo, rel))
        .collect()
}

fn untracked_file(repo: &Path, rel: &str) -> Option<ChangeFile> {
    let path = repo.join(rel);
    let meta = fs::metadata(&path).ok()?;
    if !meta.is_file() {
        return None;
    }
    if meta.len() > MAX_FILE_BYTES {
        return Some(ChangeFile {
            path: rel.replace('\\', "/"),
            hunks: vec![ChangeHunk {
                header: "@@".into(),
                lines: vec![ChangeLine {
                    kind: ChangeLineKind::Add,
                    old_line: None,
                    new_line: Some(1),
                    text: "[file too large]".into(),
                }],
            }],
        });
    }
    let bytes = fs::read(&path).ok()?;
    if bytes.contains(&0) {
        return Some(ChangeFile {
            path: rel.replace('\\', "/"),
            hunks: vec![ChangeHunk {
                header: "@@".into(),
                lines: vec![ChangeLine {
                    kind: ChangeLineKind::Add,
                    old_line: None,
                    new_line: Some(1),
                    text: "[binary]".into(),
                }],
            }],
        });
    }
    let text = String::from_utf8_lossy(&bytes);
    let lines = text
        .lines()
        .enumerate()
        .map(|(index, line)| ChangeLine {
            kind: ChangeLineKind::Add,
            old_line: None,
            new_line: Some((index + 1) as u32),
            text: line.to_string(),
        })
        .collect::<Vec<_>>();
    Some(ChangeFile {
        path: rel.replace('\\', "/"),
        hunks: vec![ChangeHunk {
            header: format!("@@ -0,0 +1,{} @@", lines.len().max(1)),
            lines,
        }],
    })
}

fn merge_files(files: &mut Vec<ChangeFile>, extra: Vec<ChangeFile>) {
    for file in extra {
        if !files.iter().any(|seen| seen.path == file.path) {
            files.push(file);
        }
    }
}

fn parse_diff(text: &str) -> Vec<ChangeFile> {
    let mut files = Vec::new();
    let mut current_path: Option<String> = None;
    let mut hunks: Vec<ChangeHunk> = Vec::new();
    let mut hunk: Option<ChangeHunk> = None;
    let mut old_line = 0u32;
    let mut new_line = 0u32;

    let flush_file = |path: &mut Option<String>,
                      hunks: &mut Vec<ChangeHunk>,
                      hunk: &mut Option<ChangeHunk>,
                      files: &mut Vec<ChangeFile>| {
        if let Some(hunk) = hunk.take() {
            hunks.push(hunk);
        }
        if let Some(path) = path.take() {
            files.push(ChangeFile {
                path,
                hunks: std::mem::take(hunks),
            });
        } else {
            hunks.clear();
        }
    };

    for raw in text.lines() {
        if let Some(path) = parse_diff_git_path(raw) {
            flush_file(&mut current_path, &mut hunks, &mut hunk, &mut files);
            current_path = Some(path);
            continue;
        }
        if raw.starts_with("+++ ") {
            if let Some(path) = strip_diff_prefix(raw.trim_start_matches("+++ ").trim()) {
                if path != "/dev/null" {
                    current_path = Some(path);
                }
            }
            continue;
        }
        if raw.starts_with("Binary files ") || raw.contains("differ") && raw.starts_with("Binary") {
            let path = current_path.clone().unwrap_or_else(|| "binary".into());
            hunks.push(ChangeHunk {
                header: raw.to_string(),
                lines: vec![ChangeLine {
                    kind: ChangeLineKind::Context,
                    old_line: None,
                    new_line: None,
                    text: raw.to_string(),
                }],
            });
            current_path = Some(path);
            continue;
        }
        if raw.starts_with("@@") {
            if let Some(hunk) = hunk.take() {
                hunks.push(hunk);
            }
            let (old, new) = parse_hunk_header(raw);
            old_line = old;
            new_line = new;
            hunk = Some(ChangeHunk {
                header: raw.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        let Some(current) = hunk.as_mut() else {
            continue;
        };
        if let Some(text) = raw.strip_prefix('+') {
            current.lines.push(ChangeLine {
                kind: ChangeLineKind::Add,
                old_line: None,
                new_line: Some(new_line),
                text: text.to_string(),
            });
            new_line += 1;
        } else if let Some(text) = raw.strip_prefix('-') {
            current.lines.push(ChangeLine {
                kind: ChangeLineKind::Delete,
                old_line: Some(old_line),
                new_line: None,
                text: text.to_string(),
            });
            old_line += 1;
        } else if let Some(text) = raw.strip_prefix(' ') {
            current.lines.push(ChangeLine {
                kind: ChangeLineKind::Context,
                old_line: Some(old_line),
                new_line: Some(new_line),
                text: text.to_string(),
            });
            old_line += 1;
            new_line += 1;
        }
    }
    flush_file(&mut current_path, &mut hunks, &mut hunk, &mut files);
    files
}

fn parse_diff_git_path(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let _old = parts.next()?;
    let new = parts.next()?;
    strip_diff_prefix(new)
}

fn strip_diff_prefix(path: &str) -> Option<String> {
    let path = path.trim();
    if path == "/dev/null" {
        return None;
    }
    let path = path
        .strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .unwrap_or(path);
    Some(path.replace('\\', "/"))
}

fn parse_hunk_header(header: &str) -> (u32, u32) {
    let mut old = 0u32;
    let mut new = 0u32;
    for token in header.split_whitespace() {
        if let Some(rest) = token.strip_prefix('-') {
            old = rest
                .split(',')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        } else if let Some(rest) = token.strip_prefix('+') {
            new = rest
                .split(',')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        }
    }
    (old, new)
}

pub fn discover_repos(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    visit_repos(root, 0, &mut found);
    found.sort();
    found
}

fn visit_repos(dir: &Path, depth: usize, found: &mut Vec<PathBuf>) {
    if depth > MAX_REPO_DEPTH {
        return;
    }
    if is_git_repo(dir) {
        found.push(dir.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if SKIP_DIRS.iter().any(|skip| *skip == name) {
            continue;
        }
        visit_repos(&path, depth + 1, found);
    }
}

fn is_git_repo(dir: &Path) -> bool {
    dir.join(".git").exists()
}

fn git_head(repo: &Path) -> Option<String> {
    let sha = git_stdout(repo, &["rev-parse", "HEAD"]).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

fn git_stdout(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(["--no-optional-locks"])
        .args(args)
        .current_dir(repo)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        _ => String::new(),
    }
}

fn display_path(root: &Path, path: &Path) -> String {
    if same_path(root, path) {
        return ".".into();
    }
    if let Ok(rel) = path.strip_prefix(root) {
        let text = rel.to_string_lossy().replace('\\', "/");
        if !text.is_empty() {
            return text;
        }
    }
    if let (Some(root), Some(path)) = (canonical(root), canonical(path)) {
        if let Ok(rel) = path.strip_prefix(root) {
            let text = rel.to_string_lossy().replace('\\', "/");
            if !text.is_empty() {
                return text;
            }
        }
    }
    path.to_string_lossy().replace('\\', "/")
}

fn canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (canonical(left), canonical(right)) {
        (Some(left), Some(right)) => left == right,
        _ => left == right,
    }
}
