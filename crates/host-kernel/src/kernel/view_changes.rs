use super::super::*;

impl HostKernel {
    pub(crate) fn change_notes_path(&self) -> PathBuf {
        self.data.host_dir.join("change-notes.json")
    }

    pub(crate) fn persist_change_notes(&self) -> Result<(), KernelError> {
        write_json(&self.change_notes_path(), &self.change_notes)
    }

    pub(crate) fn load_change_notes(&mut self) {
        let Ok(raw) = fs::read_to_string(self.change_notes_path()) else {
            return;
        };
        if let Ok(notes) = serde_json::from_str(&raw) {
            self.change_notes = notes;
        }
    }

    pub(crate) fn clear_pending_notes(
        &mut self,
        project_id: &str,
        issue_id: Option<&str>,
    ) -> Result<(), KernelError> {
        self.change_notes
            .retain(|note| note.project_id != project_id || note.issue_id.as_deref() != issue_id);
        self.persist_change_notes()
    }

    pub(crate) fn run_for_changes(
        &self,
        run_id: Option<&str>,
        issue_id: Option<&str>,
    ) -> Result<&RunSummary, KernelError> {
        if let Some(run_id) = run_id.filter(|id| !id.is_empty()) {
            let run = self
                .runs
                .iter()
                .find(|run| run.id == run_id)
                .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
            if run.is_archived() {
                return Err(KernelError::Denied("run is archived".into()));
            }
            return Ok(run);
        }
        let issue_id = issue_id
            .filter(|id| !id.is_empty())
            .ok_or_else(|| KernelError::Protocol("missing runId".into()))?;
        self.runs
            .iter()
            .rev()
            .find(|run| {
                run.issue_id.as_deref() == Some(issue_id) && run.is_active() && !run.is_archived()
            })
            .or_else(|| {
                self.runs
                    .iter()
                    .rev()
                    .find(|run| run.issue_id.as_deref() == Some(issue_id) && !run.is_archived())
            })
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))
    }

    pub(crate) fn view_changes(
        &self,
        run_id: Option<&str>,
        issue_id: Option<&str>,
        scope: ChangeScope,
    ) -> Result<ViewChanges, KernelError> {
        let run = self.run_for_changes(run_id, issue_id)?;
        if self.isolation_directory_unconfirmed(run) {
            return Ok(ViewChanges {
                run_id: run.id.clone(),
                issue_id: run.issue_id.clone(),
                working_directory: run.working_directory.clone(),
                isolated: run.isolated,
                scope,
                available: false,
                unavailable_reason: Some(super::isolation::pending_directory_note(
                    self.appearance.language,
                )),
                repos: Vec::new(),
                notes: Vec::new(),
            });
        }
        Ok(changes::compute_view(
            run,
            scope,
            &self.change_notes,
            self.appearance.language,
        ))
    }

    pub(crate) fn write_change_note(
        &mut self,
        run_id: &str,
        repo: String,
        path: String,
        line: u32,
        text: String,
    ) -> Result<(), KernelError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(KernelError::Protocol("missing text".into()));
        }
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if run.is_archived() {
            return Err(KernelError::Denied("run is archived".into()));
        }
        self.change_notes.push(changes::new_note(
            &run,
            if repo.trim().is_empty() {
                ".".into()
            } else {
                repo
            },
            path,
            line,
            text.to_string(),
        ));
        self.persist_change_notes()
    }

    pub(crate) fn delete_change_note(&mut self, note_id: &str) -> Result<(), KernelError> {
        let before = self.change_notes.len();
        self.change_notes.retain(|note| note.id != note_id);
        if self.change_notes.len() == before {
            return Err(KernelError::Protocol("unknown note".into()));
        }
        self.persist_change_notes()
    }
}
