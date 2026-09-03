use super::*;
use crate::issue::{parse_issue_id, DependencyRef, IssueRecord, IssueRef};
use crate::tracker_seam::TrackerPort;
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

/// Tracker adapter for the repository-local Markdown convention used by Matt's
/// local tracker. The project `repository` field contains the absolute checkout
/// path; files are discovered below `.scratch/*/issues/*.md`.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalMarkdownTracker;

impl LocalMarkdownTracker {
    fn root(ctx: &ProbeContext<'_>) -> PathBuf {
        PathBuf::from(ctx.repository)
    }

    fn issue_files(root: &Path) -> Vec<PathBuf> {
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

    fn validate_relation_update<F>(
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

    fn parse_file(
        root: &Path,
        path: &Path,
    ) -> Result<(IssueRecord, String, Vec<LocalReference>, Vec<String>), String> {
        let body =
            std::fs::read_to_string(path).map_err(|err| format!("{}: {err}", path.display()))?;
        Self::parse_document(root, path, body)
    }

    fn parse_document(
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

    fn locate_issue(root: &Path, issue_id: &str) -> Result<PathBuf, TrackerWriteError> {
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

    fn atomic_write(path: &Path, contents: &str) -> Result<(), TrackerWriteError> {
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

    fn write_status(
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

    fn transition_open_status(
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

    fn rewrite_parent(
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

    fn rewrite_blocked_by(
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

    fn rewrite_blocked_by_set(
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

    fn write_blocked_by_references(
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

#[derive(Debug, Clone)]
struct LocalReference {
    raw: String,
    file: Option<String>,
    number: Option<u64>,
    title: String,
}

fn normalize_metadata(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_field_line(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("**").unwrap_or(trimmed);
    let (name, value) = trimmed.split_once(':')?;
    let name = name.strip_suffix("**").unwrap_or(name).trim();
    if name.is_empty() || !name.chars().next()?.is_ascii_alphabetic() {
        return None;
    }
    Some((
        normalize_metadata(name),
        value.trim().trim_matches('*').trim().to_string(),
    ))
}

fn is_local_metadata_field(name: &str) -> bool {
    matches!(
        name,
        "status"
            | "type"
            | "assignee"
            | "assignees"
            | "part of"
            | "parent"
            | "blocked by"
            | "closed"
    )
}

fn header_fields(text: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::<String, Vec<String>>::new();
    for line in text.lines() {
        if line.trim_start().starts_with("## ") {
            break;
        }
        if let Some((name, value)) = parse_field_line(line) {
            fields.entry(name).or_default().push(value);
        }
    }
    fields
}

fn section_lines(text: &str, heading: &str) -> Vec<String> {
    let wanted = normalize_metadata(heading);
    let mut in_section = false;
    let mut result = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(section) = trimmed.strip_prefix("## ") {
            let current = normalize_metadata(section);
            if in_section && current != wanted {
                break;
            }
            in_section = current == wanted;
            continue;
        }
        if in_section {
            result.push(line.to_string());
        }
    }
    result
}

fn blocked_by_reference_lines(text: &str) -> Vec<String> {
    let section = section_lines(text, "Blocked by");
    if !section.is_empty() {
        return section;
    }
    header_fields(text)
        .get("blocked by")
        .cloned()
        .unwrap_or_default()
}

fn normalize_reference_title(value: &str) -> String {
    normalize_metadata(
        value
            .trim()
            .trim_start_matches(['-', '*', '+'])
            .trim()
            .trim_matches(['`', '*', '_', '.', ',', ';', ':'])
            .trim_start_matches(['—', '–', ':', '-'])
            .trim(),
    )
}

fn parse_reference(raw: &str) -> Option<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return None;
    }
    let cleaned = raw
        .trim_start_matches(['-', '*', '+'])
        .trim()
        .trim_matches(['`', '*', '_'])
        .trim();
    if cleaned.is_empty() || normalize_metadata(cleaned).starts_with("none") {
        return None;
    }
    if let Some(start) = cleaned.find('(') {
        if let Some(end) = cleaned[start + 1..].find(')') {
            let target = &cleaned[start + 1..start + 1 + end];
            let file = target
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(target)
                .to_string();
            return Some(LocalReference {
                raw: raw.into(),
                file: Some(file),
                number: None,
                title: String::new(),
            });
        }
    }
    let digits = cleaned.strip_prefix('#').unwrap_or(cleaned);
    let digit_end = digits
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(digits.len());
    if digit_end == 0 {
        return Some(LocalReference {
            raw: raw.into(),
            file: None,
            number: None,
            title: String::new(),
        });
    }
    let number = digits[..digit_end].parse::<u64>().ok();
    let rest = digits[digit_end..].trim();
    let title = rest
        .strip_prefix(['—', '–', ':', '-'])
        .unwrap_or(rest)
        .trim();
    let file = cleaned
        .split_whitespace()
        .find(|part| part.to_ascii_lowercase().ends_with(".md"))
        .map(|part| part.trim_matches(['`', '*', '_', ',', ';']).to_string());
    Some(LocalReference {
        raw: raw.into(),
        file,
        number,
        title: normalize_reference_title(title),
    })
}

fn parse_reference_line(raw: &str) -> Vec<LocalReference> {
    let raw = raw.trim();
    if raw.is_empty() || normalize_metadata(raw).starts_with("none") {
        return Vec::new();
    }
    let mut parts = raw
        .split([',', ';'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.len() == 1 {
        // A markdown link may contain commas in its label; parse it as one token.
        parts = vec![raw];
    }
    parts.into_iter().filter_map(parse_reference).collect()
}

fn local_slug(title: &str) -> String {
    let mut slug = String::new();
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "issue".into()
    } else {
        slug.chars().take(48).collect()
    }
}

fn graph_cycle_node(adjacency: &BTreeMap<String, Vec<String>>) -> Option<String> {
    fn visit(
        node: &str,
        adjacency: &BTreeMap<String, Vec<String>>,
        visiting: &mut BTreeSet<String>,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if !visited.insert(node.to_string()) {
            return false;
        }
        visiting.insert(node.to_string());
        let cycle = adjacency
            .get(node)
            .into_iter()
            .flatten()
            .any(|neighbor| visit(neighbor, adjacency, visiting, visited));
        visiting.remove(node);
        cycle
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    adjacency
        .keys()
        .find(|node| visit(node, adjacency, &mut visiting, &mut visited))
        .cloned()
}

impl TrackerPort for LocalMarkdownTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        let root = Self::root(ctx);
        if root.is_dir() && (root.join(".scratch").is_dir() || !Self::issue_files(&root).is_empty())
        {
            ProbeOutcome::Ready {
                source: CredentialSource::LocalFile,
            }
        } else {
            ProbeOutcome::Failed {
                source: Some(CredentialSource::LocalFile),
                kind: AuthFailureKind::Unreachable,
                cli_detected: false,
                detail: Some(".scratch/*/issues/*.md not found".into()),
            }
        }
    }

    fn read_issues(&self, ctx: &ProbeContext<'_>) -> Result<Vec<IssueRecord>, TrackerReadError> {
        match self.read_all(ctx)? {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues }
            | crate::tracker_seam::TrackerReadOutcome::Incomplete { issues, .. } => Ok(issues),
        }
    }

    fn read_all(
        &self,
        ctx: &ProbeContext<'_>,
    ) -> Result<crate::tracker_seam::TrackerReadOutcome, TrackerReadError> {
        let root = Self::root(ctx);
        if !root.is_dir() {
            return Err(TrackerReadError::Offline {
                source: Some(CredentialSource::LocalFile),
                cli_detected: false,
                detail: Some("local project directory does not exist".into()),
            });
        }
        let mut parsed = Vec::new();
        let mut problems = Vec::new();
        for path in Self::issue_files(&root) {
            match Self::parse_file(&root, &path) {
                Ok((issue, body, references, errors)) => {
                    if !errors.is_empty() {
                        problems.push(format!("{}: {}", path.display(), errors.join(", ")));
                    }
                    parsed.push((issue, body, references));
                }
                Err(error) => problems.push(error),
            }
        }
        let repository = root.to_string_lossy().to_string();
        let mut by_number = BTreeMap::<u64, Vec<IssueRecord>>::new();
        for (issue, _, _) in &parsed {
            by_number
                .entry(issue.number)
                .or_default()
                .push(issue.clone());
        }
        for (number, candidates) in &by_number {
            if candidates.len() > 1 {
                let locations = candidates
                    .iter()
                    .map(|issue| issue.url.trim_start_matches("file://"))
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(format!("duplicate issue number #{number}: {locations}"));
            }
        }
        let mut issues = parsed
            .iter()
            .map(|(issue, _, _)| issue.clone())
            .collect::<Vec<_>>();
        for (index, (_, _, references)) in parsed.iter().enumerate() {
            let issue_id = issues[index].id();
            for reference in references {
                let resolved = reference.file.as_deref().and_then(|file| {
                    parsed
                        .iter()
                        .find(|(issue, _, _)| issue.url.rsplit('/').next() == Some(file))
                        .map(|(issue, _, _)| issue)
                });
                let resolved = if resolved.is_some() {
                    resolved
                } else {
                    let candidates = reference.number.and_then(|number| by_number.get(&number));
                    candidates.and_then(|candidates| {
                        if candidates.len() == 1 {
                            candidates.first()
                        } else if !reference.title.is_empty() {
                            candidates.iter().find(|issue| {
                                normalize_reference_title(&issue.title) == reference.title
                            })
                        } else {
                            None
                        }
                    })
                };
                if let Some(target) = resolved {
                    if target.id() == issue_id {
                        problems.push(format!("{} has a self dependency", issue_id));
                        issues[index].blocked_by.push(DependencyRef::Unclear {
                            repository: Some(repository.clone()),
                            number: Some(target.number),
                        });
                    } else {
                        issues[index].blocked_by.push(DependencyRef::Known(
                            IssueRef::new(
                                target.repository.clone(),
                                target.number,
                                target.title.clone(),
                            )
                            .with_open(target.open),
                        ));
                    }
                } else {
                    if reference.number.is_none() && reference.file.is_none() {
                        problems.push(format!(
                            "{issue_id} has invalid dependency reference: {}",
                            reference.raw
                        ));
                    }
                    issues[index].blocked_by.push(DependencyRef::Unclear {
                        repository: Some(repository.clone()),
                        number: reference.number,
                    });
                }
            }
            let issue_number = issues[index].number;
            if let Some(parent) = issues[index].parent.as_mut() {
                if parent.number == issue_number {
                    problems.push(format!("{issue_id} has a self parent"));
                    issues[index].parent = None;
                    continue;
                }
                if let Some(candidates) = by_number.get(&parent.number) {
                    if candidates.len() == 1 {
                        parent.title = candidates[0].title.clone();
                        parent.open = Some(candidates[0].open);
                    } else {
                        problems.push(format!(
                            "{issue_id} has an ambiguous parent #{}",
                            parent.number
                        ));
                        issues[index].parent = None;
                    }
                } else {
                    problems.push(format!(
                        "{issue_id} references missing parent #{}",
                        parent.number
                    ));
                    issues[index].parent = None;
                }
            }
        }
        let parent_adjacency = issues
            .iter()
            .map(|issue| {
                (
                    issue.id(),
                    issue
                        .parent
                        .as_ref()
                        .map(IssueRef::id)
                        .into_iter()
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(issue_id) = graph_cycle_node(&parent_adjacency) {
            problems.push(format!("parent cycle includes {issue_id}"));
        }
        let mut index_by_id = BTreeMap::new();
        for (index, issue) in issues.iter().enumerate() {
            index_by_id.insert(issue.id(), index);
        }
        let relations = issues
            .iter()
            .enumerate()
            .flat_map(|(child_index, issue)| {
                issue
                    .parent
                    .as_ref()
                    .map(|parent| (child_index, parent.id()))
            })
            .collect::<Vec<_>>();
        for (child_index, parent_id) in relations {
            if let Some(parent_index) = index_by_id.get(&parent_id).copied() {
                let child = IssueRef::new(
                    issues[child_index].repository.clone(),
                    issues[child_index].number,
                    issues[child_index].title.clone(),
                )
                .with_open(issues[child_index].open);
                if !issues[parent_index]
                    .children
                    .iter()
                    .any(|item| item.id() == child.id())
                {
                    issues[parent_index].children.push(child);
                }
            }
        }
        let dependency_relations = issues
            .iter()
            .enumerate()
            .flat_map(|(blocked_index, issue)| {
                issue
                    .blocked_by
                    .iter()
                    .filter_map(move |dependency| match dependency {
                        DependencyRef::Known(blocker) => Some((blocked_index, blocker.id())),
                        DependencyRef::Unclear { .. } => None,
                    })
            })
            .collect::<Vec<_>>();
        for (blocked_index, blocker_id) in dependency_relations {
            if let Some(blocker_index) = index_by_id.get(&blocker_id).copied() {
                let blocked = IssueRef::new(
                    issues[blocked_index].repository.clone(),
                    issues[blocked_index].number,
                    issues[blocked_index].title.clone(),
                )
                .with_open(issues[blocked_index].open);
                if !issues[blocker_index]
                    .blocking
                    .iter()
                    .any(|item| item.id() == blocked.id())
                {
                    issues[blocker_index].blocking.push(blocked);
                }
            }
        }
        let adjacency = issues
            .iter()
            .map(|issue| {
                (
                    issue.id(),
                    issue
                        .blocked_by
                        .iter()
                        .filter_map(|dependency| match dependency {
                            DependencyRef::Known(blocker) => Some(blocker.id()),
                            DependencyRef::Unclear { .. } => None,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        if let Some(issue_id) = graph_cycle_node(&adjacency) {
            problems.push(format!("dependency cycle includes {issue_id}"));
        }
        issues.sort_by_key(|issue| issue.number);
        if problems.is_empty() {
            Ok(crate::tracker_seam::TrackerReadOutcome::Complete { issues })
        } else {
            Ok(crate::tracker_seam::TrackerReadOutcome::Incomplete {
                issues,
                detail: problems.join("; "),
            })
        }
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id).map_err(|error| match error {
            TrackerWriteError::Failed { message } => TrackerReadError::Failed {
                detail: Some(message),
            },
            _ => TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            },
        })?;
        let (issue, body, _, errors) =
            Self::parse_file(&root, &path).map_err(|error| TrackerReadError::Failed {
                detail: Some(error),
            })?;
        if !errors.is_empty() {
            return Err(TrackerReadError::Failed {
                detail: Some(errors.join(", ")),
            });
        }
        Ok(IssueDocument { issue, body })
    }

    fn create_issue(
        &self,
        ctx: &ProbeContext<'_>,
        title: &str,
        body: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        let files = Self::issue_files(&root);
        let issues_dir = files
            .first()
            .and_then(|path| path.parent())
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.join(".scratch").join("taskboard").join("issues"));
        std::fs::create_dir_all(&issues_dir).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let next = files
            .iter()
            .filter_map(|path| {
                path.file_stem()
                    .and_then(|stem| stem.to_str())
                    .and_then(|stem| stem.split_once('-'))
                    .and_then(|(n, _)| n.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let slug = local_slug(title);
        let path = issues_dir.join(format!("{next:02}-{slug}.md"));
        let document = format!(
            "# {next:02} — {}\n\nStatus: ready-for-agent\n\n{}\n",
            title.trim(),
            body.trim()
        );
        let (issue, _, _, errors) = Self::parse_document(&root, &path, document.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &document)?;
        Ok(issue)
    }
    fn update_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        edit: IssueEdit<'_>,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id)?;
        let original = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let (current, _, _, current_errors) = Self::parse_file(&root, &path)
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !current_errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: current_errors.join(", "),
            });
        }
        let title = edit.title.unwrap_or(&current.title).trim();
        if title.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: "title cannot be empty".into(),
            });
        }
        let issue_number_text = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .and_then(|stem| stem.split_once('-'))
            .map(|(number, _)| number.to_string())
            .unwrap_or_else(|| current.number.to_string());
        let mut lines = original.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim_start().starts_with('#'))
        {
            lines[index] = format!("# {} — {}", issue_number_text, title);
        }
        if let Some(new_body) = edit.body {
            let metadata = lines
                .iter()
                .skip(1)
                .take_while(|line| !line.trim_start().starts_with("## "))
                .filter(|line| {
                    parse_field_line(line).is_some_and(|(name, _)| is_local_metadata_field(&name))
                })
                .cloned()
                .collect::<Vec<_>>();
            let comments = section_lines(&original, "Comments");
            let mut rebuilt = vec![
                format!("# {} — {}", issue_number_text, title),
                String::new(),
            ];
            rebuilt.extend(metadata);
            rebuilt.push(String::new());
            rebuilt.extend(new_body.lines().map(ToOwned::to_owned));
            if !comments.is_empty() {
                rebuilt.push(String::new());
                rebuilt.push("## Comments".into());
                rebuilt.extend(comments);
            }
            lines = rebuilt;
        }
        let contents = format!("{}\n", lines.join("\n"));
        let (issue, _, _, errors) = Self::parse_document(&root, &path, contents.clone())
            .map_err(|message| TrackerWriteError::Failed { message })?;
        if !errors.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: errors.join(", "),
            });
        }
        Self::atomic_write(&path, &contents)?;
        Ok(issue)
    }
    fn close_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        Self::write_status(&Self::root(ctx), issue_id, "resolved", false, true)
    }
    fn reopen_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        Self::transition_open_status(&root, issue_id, false)
    }
    fn add_comment(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        body: &str,
    ) -> Result<IssueComment, TrackerWriteError> {
        let body = body.trim();
        if body.is_empty() {
            return Err(TrackerWriteError::Failed {
                message: "comment cannot be empty".into(),
            });
        }
        let root = Self::root(ctx);
        let path = Self::locate_issue(&root, issue_id)?;
        let original = std::fs::read_to_string(&path).map_err(|err| TrackerWriteError::Failed {
            message: err.to_string(),
        })?;
        let mut lines = original.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
        let comment_id = section_lines(&original, "Comments")
            .iter()
            .filter(|line| !line.trim().is_empty())
            .count()
            + 1;
        if let Some(index) = lines
            .iter()
            .position(|line| line.trim().eq_ignore_ascii_case("## comments"))
        {
            let mut end = index + 1;
            while end < lines.len() && !lines[end].trim_start().starts_with("## ") {
                end += 1;
            }
            lines.splice(end..end, [format!("- {body}")]);
        } else {
            lines.extend([String::new(), "## Comments".into(), format!("- {body}")]);
        }
        Self::atomic_write(&path, &format!("{}\n", lines.join("\n")))?;
        Ok(IssueComment {
            id: comment_id.to_string(),
            url: format!("file://{}#comment-{comment_id}", path.display()),
            body: body.into(),
        })
    }
    fn claim_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        Self::write_status(&Self::root(ctx), issue_id, "claimed", false, false)
    }
    fn release_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerWriteError> {
        let root = Self::root(ctx);
        Self::transition_open_status(&root, issue_id, true)
    }
    fn set_parent(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        parent: Option<&str>,
    ) -> Result<(), TrackerWriteError> {
        let root = Self::root(ctx);
        Self::locate_issue(&root, issue_id)?;
        if let Some(parent) = parent {
            let parent_path = Self::locate_issue(&root, parent)?;
            if parent_path == Self::locate_issue(&root, issue_id)? {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot parent itself".into(),
                });
            }
        }
        self.validate_relation_update(
            ctx,
            issue_id,
            parent.into_iter().map(ToOwned::to_owned).collect(),
            "parent",
            |issue| {
                issue
                    .parent
                    .as_ref()
                    .map(IssueRef::id)
                    .into_iter()
                    .collect()
            },
        )?;
        Self::rewrite_parent(
            &root,
            issue_id,
            parent.map(|id| id.rsplit_once('#').map(|(_, number)| number).unwrap_or(id)),
        )
    }
    fn add_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        Self::rewrite_blocked_by(&Self::root(ctx), issue_id, blocking_issue_id, true)
    }
    fn remove_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        blocking_issue_id: &str,
    ) -> Result<(), TrackerWriteError> {
        Self::rewrite_blocked_by(&Self::root(ctx), issue_id, blocking_issue_id, false)
    }
    fn set_blocked_by(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
        _current_issue_ids: &[String],
        blocking_issue_ids: &[String],
    ) -> Result<(), TrackerWriteError> {
        let root = Self::root(ctx);
        Self::locate_issue(&root, issue_id)?;
        let mut wanted = Vec::new();
        for blocking_issue_id in blocking_issue_ids {
            Self::locate_issue(&root, blocking_issue_id)?;
            if blocking_issue_id == issue_id {
                return Err(TrackerWriteError::Failed {
                    message: "issue cannot block itself".into(),
                });
            }
            if !wanted.contains(blocking_issue_id) {
                wanted.push(blocking_issue_id.clone());
            }
        }
        self.validate_relation_update(ctx, issue_id, wanted, "dependency", |issue| {
            issue
                .blocked_by
                .iter()
                .filter_map(|dependency| match dependency {
                    DependencyRef::Known(blocker) => Some(blocker.id()),
                    DependencyRef::Unclear { .. } => None,
                })
                .collect()
        })?;
        Self::rewrite_blocked_by_set(&root, issue_id, blocking_issue_ids)
    }
}
