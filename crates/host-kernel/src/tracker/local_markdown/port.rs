use super::*;
use crate::tracker_seam::TrackerPort;

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

    fn read_issue_content(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueDocument, TrackerReadError> {
        let mut document = TrackerPort::read_issue_document(self, ctx, issue_id)?;
        document.body = editable_body(&document.body);
        Ok(document)
    }

    fn read_issue_relations(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<IssueRecord, TrackerReadError> {
        let issues = match TrackerPort::read_all(self, ctx)? {
            crate::tracker_seam::TrackerReadOutcome::Complete { issues } => issues,
            crate::tracker_seam::TrackerReadOutcome::Incomplete { detail, .. } => {
                return Err(TrackerReadError::Failed {
                    detail: Some(format!(
                        "cannot validate Issue relations from incomplete data: {detail}"
                    )),
                });
            }
        };
        issues
            .into_iter()
            .find(|issue| issue.id() == issue_id)
            .ok_or_else(|| TrackerReadError::Failed {
                detail: Some("unknown issue".into()),
            })
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
