use super::*;
use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};
use crate::tracker_seam::TrackerPort;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

mod parse;
mod port;

/// Tracker adapter for the repository-local Markdown convention used by Matt's
/// local tracker. The project `repository` field contains the absolute checkout
/// path; files are discovered below `.scratch/*/issues/*.md`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalMarkdownTracker;

pub(crate) use parse::*;

impl LocalMarkdownTracker {
    pub(crate) fn root(ctx: &ProbeContext<'_>) -> PathBuf {
        PathBuf::from(ctx.repository)
    }

    pub(crate) fn issue_files(root: &Path) -> Vec<PathBuf> {
        let scratch = root.join(".scratch");
        let Ok(features) = std::fs::read_dir(scratch) else {
            return Vec::new();
        };
        let mut files = Vec::new();
        for feature in features.flatten() {
            let issues = feature.path().join("issues");
            let Ok(entries) = std::fs::read_dir(issues) else {
                continue;
            };
            files.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
                path.extension().is_some_and(|ext| ext == "md")
                    && path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .and_then(|name| name.split_once('-'))
                        .and_then(|(n, _)| n.parse::<u64>().ok())
                        .is_some()
            }));
        }
        files.sort();
        files
    }

    pub fn content_revision(root: &Path) -> std::io::Result<u64> {
        let mut hasher = DefaultHasher::new();
        for path in Self::issue_files(root) {
            path.hash(&mut hasher);
            std::fs::read(&path)?.hash(&mut hasher);
        }
        Ok(hasher.finish())
    }

    pub(crate) fn validate_relation_update<F>(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        replacement: Vec<String>,
        relation: &str,
        edges: F,
    ) -> Result<(), TrackerWriteError>
    where
        F: Fn(&IssueRecord) -> Vec<String>,
    {
        let outcome = self
            .read_all(ctx)
            .map_err(|error| TrackerWriteError::Failed {
                message: format!("cannot validate {relation} graph: {error:?}"),
            })?;
        let issues = match outcome {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues } => issues,
            crate::tracker_seam::TrackerReadOutcome::Incomplete { detail, .. } => {
                return Err(TrackerWriteError::Failed {
                    message: format!(
                        "cannot update {relation}: tracker data is incomplete: {detail}"
                    ),
                });
            }
        };
        let mut adjacency = issues
            .iter()
            .map(|issue| (issue.id(), edges(issue)))
            .collect::<BTreeMap<_, _>>();
        adjacency.insert(issue_id.to_string(), replacement);
        if graph_cycle_node(&adjacency).is_some() {
            return Err(TrackerWriteError::Failed {
                message: format!("{relation} update would create a cycle"),
            });
        }
        Ok(())
    }

    pub(crate) fn parse_file(
        root: &Path,
        path: &Path,
    ) -> Result<(IssueRecord, String, Vec<LocalReference>, Vec<String>), String> {
        let body =
            std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
        Self::parse_document(root, path, body)
    }

    pub(crate) fn parse_document(
        root: &Path,
        path: &Path,
        body: String,
    ) -> Result<(IssueRecord, String, Vec<LocalReference>, Vec<String>), String> {
        let stem = path
            .file_stem()
            .ok_or_else(|| format!("{} has no filename", path.display()))?
            .to_string_lossy();
        let (number_text, _) = stem
            .split_once('-')
            .ok_or_else(|| format!("{} must be named NN-slug.md", path.display()))?;
        let number = number_text
            .parse::<u64>()
            .map_err(|_| format!("{} has an invalid issue number", path.display()))?;
        let fields = header_fields(&body);
        let mut metadata_errors = Vec::new();
        let status_values = fields.get("status").cloned().unwrap_or_default();
        let status = status_values.last().cloned().unwrap_or_default();
        let normalized_status = normalize_metadata(&status);
        if status_values.len() > 1
            && status_values
                .iter()
                .map(|value| normalize_metadata(value))
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            metadata_errors.push("conflicting Status metadata".into());
        }
        let type_values = fields.get("type").cloned().unwrap_or_default();
        let issue_type = type_values.last().map(|value| normalize_metadata(value));
        if type_values.len() > 1
            && type_values
                .iter()
                .map(|value| normalize_metadata(value))
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            metadata_errors.push("conflicting Type metadata".into());
        }
        let is_wayfinder = issue_type.is_some();
        let valid_wayfinder_type = issue_type
            .as_deref()
            .is_some_and(|value| matches!(value, "research" | "prototype" | "grilling" | "task"));
        if is_wayfinder && !valid_wayfinder_type {
            metadata_errors.push(format!(
                "invalid Type: {}",
                issue_type.as_deref().unwrap_or("<empty>")
            ));
        }
        let implementation_statuses = [
            "needs-triage",
            "needs-info",
            "ready-for-agent",
            "ready-for-human",
            "wontfix",
            "claimed",
            "resolved",
        ];
        let wayfinder_statuses = ["open", "ready-for-agent", "claimed", "resolved", "wontfix"];
        if !is_wayfinder {
            if status_values.is_empty() {
                metadata_errors.push("missing Status metadata".into());
            } else if !implementation_statuses.contains(&normalized_status.as_str()) {
                metadata_errors.push(format!("invalid Status: {status}"));
            }
        } else if !normalized_status.is_empty()
            && !wayfinder_statuses.contains(&normalized_status.as_str())
        {
            metadata_errors.push(format!("invalid Status: {status}"));
        }
        let closed_values = fields.get("closed").cloned().unwrap_or_default();
        let closed_legacy = match closed_values.last().map(|value| normalize_metadata(value)) {
            None => None,
            Some(value) if value == "true" => Some(true),
            Some(value) if value == "false" => Some(false),
            Some(value) => {
                metadata_errors.push(format!("invalid Closed: {value}"));
                None
            }
        };
        let terminal_status = matches!(normalized_status.as_str(), "resolved" | "wontfix");
        if terminal_status && closed_legacy == Some(false) {
            metadata_errors.push("Status terminal value conflicts with Closed: false".into());
        }
        let closed = terminal_status && closed_legacy != Some(false) || closed_legacy == Some(true);
        let open = !closed;
        if closed_legacy == Some(true) {
            metadata_errors.retain(|error| !error.starts_with("missing Status"));
        }
        let repository = root.to_string_lossy().to_string();
        let title = body
            .lines()
            .find_map(|line| {
                let heading = line.trim().strip_prefix('#')?.trim();
                let heading = heading
                    .strip_prefix(&format!("{number_text} "))
                    .unwrap_or(heading);
                let heading = heading
                    .strip_prefix('—')
                    .or_else(|| heading.strip_prefix('–'))
                    .or_else(|| heading.strip_prefix('-'))
                    .or_else(|| heading.strip_prefix(':'))
                    .unwrap_or(heading)
                    .trim();
                (!heading.is_empty()).then(|| heading.to_string())
            })
            .unwrap_or_else(|| stem.to_string());
        let mut labels = Vec::new();
        if implementation_statuses.contains(&normalized_status.as_str())
            && !labels.iter().any(|label| label == &normalized_status)
        {
            labels.push(normalized_status.clone());
        }
        if let Some(issue_type) = issue_type.as_deref().filter(|_| valid_wayfinder_type) {
            labels.push(format!("type:{issue_type}"));
        }
        if !normalized_status.is_empty() {
            labels.push(format!("status:{normalized_status}"));
        }
        labels.sort();
        labels.dedup();
        let assignee_values = fields
            .get("assignee")
            .or_else(|| fields.get("assignees"))
            .into_iter()
            .flatten()
            .flat_map(|value| value.split([',', ';']).map(|item| item.trim().to_string()))
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        let mut assignees = assignee_values;
        if normalized_status == "claimed" && assignees.is_empty() {
            assignees.push(MEMORY_TRACKER_ACTOR.into());
        }
        let parent_raw = fields
            .get("part of")
            .or_else(|| fields.get("parent"))
            .and_then(|values| values.last())
            .cloned()
            .or_else(|| {
                let section = section_lines(&body, "Parent");
                (!section.is_empty()).then(|| section.join(" "))
            });
        let parent = parent_raw
            .as_deref()
            .and_then(parse_reference)
            .and_then(|reference| {
                reference.number.map(|parent_number| {
                    IssueRef::new(repository.clone(), parent_number, reference.title)
                        .with_open(true)
                })
            });
        let references = blocked_by_reference_lines(&body)
            .into_iter()
            .flat_map(|line| parse_reference_line(&line))
            .collect::<Vec<_>>();
        let issue = IssueRecord {
            repository,
            number,
            title,
            url: format!("file://{}", path.display()),
            open,
            closed_at: closed.then(|| "local-markdown".into()),
            assignees,
            labels,
            parent,
            children: Vec::new(),
            blocked_by: Vec::new(),
            blocking: Vec::new(),
        };
        Ok((issue, body, references, metadata_errors))
    }

    pub(crate) fn locate_issue(root: &Path, issue_id: &str) -> Result<PathBuf, TrackerWriteError> {
        let (repository, number) =
            parse_issue_id(issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "invalid issue id".into(),
            })?;
        if repository != root.to_string_lossy() {
            return Err(TrackerWriteError::Failed {
                message: "issue belongs to another local tracker".into(),
            });
        }
        let matches = Self::issue_files(root)
            .into_iter()
            .filter(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.split_once('-'))
                    .and_then(|(n, _)| n.parse::<u64>().ok())
                    == Some(number)
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [path] => Ok(path.clone()),
            [] => Err(TrackerWriteError::Failed {
                message: "unknown issue".into(),
            }),
            _ => Err(TrackerWriteError::Failed {
                message: format!("issue #{number} is ambiguous"),
            }),
        }
    }

    pub(crate) fn atomic_write(path: &Path, contents: &str) -> Result<(), TrackerWriteError> {
        let parent = path.parent().ok_or_else(|| TrackerWriteError::Failed {
            message: "issue has no parent directory".into(),
        })?;
        let tmp = parent.join(format!(
            ".{}.tmp-{}",
            path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("issue"),
            std::process::id()
        ));
        std::fs::write(&tmp, contents).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        if let Err(err) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(TrackerWriteError::Failed {
                message: err.to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn write_status(
        root: &Path,
        issue_id: &str,
        status: &str,
        clear_assignees: bool,
        clear_closed: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let (_, _, _, existing_errors) = Self::parse_document(root, &path, body.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !existing_errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: existing_errors.join(", "),
            });
        }
        let mut lines: Vec<String> = body.lines().map(ToOwned::to_owned).collect();
        let mut replaced = false;
        for line in &mut lines {
            if line.trim_start().starts_with("## ") {
                break;
            }
            if clear_assignees
                && parse_field_line(line)
                    .is_some_and(|(name, _)| name == "assignee" || name == "assignees")
            {
                *line = String::new();
                continue;
            }
            if clear_closed && parse_field_line(line).is_some_and(|(name, _)| name == "closed") {
                *line = String::new();
                continue;
            }
            if let Some((name, _)) = parse_field_line(line) {
                if name == "status" {
                    *line = format!("Status: {status}");
                    replaced = true;
                }
            }
        }
        if !replaced {
            lines.insert(1.min(lines.len()), format!("Status: {status}"));
        }
        let contents = format!("{}\n", lines.join("\n"));
        let (issue, _, _, errors) = Self::parse_document(root, &path, contents.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &contents)?;
        Ok(issue)
    }

    pub(crate) fn transition_open_status(
        root: &Path,
        issue_id: &str,
        clear_assignees: bool,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let wayfinder = header_fields(&body)
            .get("type")
            .and_then(|values| values.last())
            .is_some_and(|value| {
                matches!(
                    normalize_metadata(value).as_str(),
                    "research" | "prototype" | "grilling" | "task"
                )
            });
        Self::write_status(
            root,
            issue_id,
            if wayfinder { "open" } else { "ready-for-agent" },
            clear_assignees,
            true,
        )
    }

    pub(crate) fn rewrite_parent(
        root: &Path,
        issue_id: &str,
        value: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let lines = body.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let mut rewritten = Vec::with_capacity(lines.len());
        let mut in_header = true;
        let mut index = 0;
        while index < lines.len() {
            let line = &lines[index];
            if let Some(section) = line.trim().strip_prefix("## ") {
                in_header = false;
                if normalize_metadata(section) == "parent" {
                    index += 1;
                    while index < lines.len() && !lines[index].trim_start().starts_with("## ") {
                        index += 1;
                    }
                    continue;
                }
            }
            if in_header
                && parse_field_line(line)
                    .is_some_and(|(name, _)| matches!(name.as_str(), "part of" | "parent"))
            {
                index += 1;
                continue;
            }
            rewritten.push(line.clone());
            index += 1;
        }
        if let Some(value) = value {
            rewritten.insert(1.min(rewritten.len()), format!("Part of: {value}"));
        }
        Self::atomic_write(&path, &format!("{}\n", rewritten.join("\n")))
    }

    pub(crate) fn rewrite_blocked_by(
        root: &Path,
        issue_id: &str,
        blocking_issue_id: &str,
        add: bool,
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let (_, blocker_number) =
            parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                message: "unknown blocker".into(),
            })?;
        Self::locate_issue(root, blocking_issue_id)?;
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let refs = blocked_by_reference_lines(&body).join(", ");
        let mut references = parse_reference_line(&refs)
            .into_iter()
            .filter(|reference| reference.number != Some(blocker_number))
            .map(|reference| {
                reference
                    .number
                    .map(|number| number.to_string())
                    .unwrap_or(reference.raw)
            })
            .collect::<Vec<_>>();
        if add {
            references.push(blocker_number.to_string());
        }
        Self::write_blocked_by_references(&path, &body, &references)
    }

    pub(crate) fn rewrite_blocked_by_set(
        root: &Path,
        issue_id: &str,
        blocking_issue_ids: &[String],
    ) -> Result<(), TrackerWriteError> {
        let path = Self::locate_issue(root, issue_id)?;
        let mut references = Vec::new();
        for blocking_issue_id in blocking_issue_ids {
            let (_, blocker_number) =
                parse_issue_id(blocking_issue_id).ok_or_else(|| TrackerWriteError::Failed {
                    message: "unknown blocker".into(),
                })?;
            let blocker_path = Self::locate_issue(root, blocking_issue_id)?;
            if blocker_path == path {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot block itself".into(),
                });
            }
            let number = blocker_number.to_string();
            if !references.contains(&number) {
                references.push(number);
            }
        }
        let body = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        Self::write_blocked_by_references(&path, &body, &references)
    }

    pub(crate) fn write_blocked_by_references(
        path: &Path,
        body: &str,
        references: &[String],
    ) -> Result<(), TrackerWriteError> {
        let replacement = if references.is_empty() {
            "Blocked by: None".to_string()
        } else {
            format!("Blocked by: {}", references.join(", "))
        };
        let mut lines: Vec<String> = body.lines().map(ToOwned::to_owned).collect();
        if let Some(line) = lines
            .iter_mut()
            .find(|line| parse_field_line(line).is_some_and(|(name, _)| name == "blocked by"))
        {
            *line = replacement;
        } else if let Some(index) = lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("## blocked by"))
        {
            let mut end = index + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with("## ") {
                end += 1;
            }
            let rendered = if references.is_empty() {
                vec!["- None".to_string()]
            } else {
                references
                    .iter()
                    .map(|reference| format!("- {reference}"))
                    .collect()
            };
            lines.splice(index + 1..end, rendered);
        } else {
            lines.insert(1.min(lines.len()), replacement);
        }
        Self::atomic_write(&path, &format!("{}\n", lines.join("\n")))
    }
}
