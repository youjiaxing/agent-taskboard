use super::super::*;

impl HostKernel {
    pub(crate) fn runs_path(&self) -> PathBuf {
        self.data.host_dir.join("runs.json")
    }

    pub(crate) fn ensure_run_persistence_writable(&self) -> Result<(), KernelError> {
        if self.run_persistence_recovery.is_none() {
            return Ok(());
        }
        Err(KernelError::Denied(
            ShellCopy::for_language(self.appearance.language).run_persistence_write_blocked,
        ))
    }

    pub(crate) fn persist_run_records(&self, runs: &Vec<RunSummary>) -> Result<(), KernelError> {
        write_json(&self.runs_path(), runs)
    }

    pub(crate) fn commit_run_records(&mut self, runs: Vec<RunSummary>) -> Result<(), KernelError> {
        self.ensure_run_persistence_writable()?;
        if let Err(err) = self.persist_run_records(&runs) {
            self.note_run_persistence_write_error(&err);
            return Err(err);
        }
        self.runs = runs;
        self.run_persistence_write_error = None;
        Ok(())
    }

    pub(crate) fn note_run_persistence_write_error(&mut self, err: &KernelError) {
        self.run_persistence_write_error = Some(RunPersistenceWriteError {
            detail: err.to_string(),
            retryable: true,
        });
    }

    pub(crate) fn usage_samples_path(&self) -> PathBuf {
        self.data.host_dir.join("usage-samples.json")
    }

    pub(crate) fn persist_usage_samples(&self) -> Result<(), KernelError> {
        write_json(&self.usage_samples_path(), &self.usage_samples)
    }

    pub(crate) fn load_usage_samples(&mut self) {
        let Ok(raw) = fs::read_to_string(self.usage_samples_path()) else {
            return;
        };
        if let Ok(samples) = serde_json::from_str(&raw) {
            self.usage_samples = samples;
        }
    }

    pub(crate) fn decorate_runs(&self, runs: &[RunSummary]) -> Vec<RunSummary> {
        runs.iter()
            .map(|run| {
                let mut run = run.clone();
                run.telemetry = usage::run_telemetry(&self.usage_samples, &run.id);
                run
            })
            .collect()
    }

    pub(crate) fn usage_open_for_focus(&self) -> bool {
        if self.focused_host_id != LOCAL_HOST_ID {
            return self
                .remote_view
                .as_ref()
                .filter(|view| view.host_id == self.focused_host_id)
                .map(|view| view.usage_open)
                .unwrap_or(false);
        }
        self.usage_open
    }

    pub(crate) fn usage_for_focus(&self) -> UsagePage {
        if self.focused_host_id != LOCAL_HOST_ID {
            if let Some(view) = &self.remote_view {
                if view.host_id == self.focused_host_id {
                    return view.usage.clone();
                }
            }
        }
        self.build_usage()
    }

    pub(crate) fn build_usage(&self) -> UsagePage {
        let runs = self
            .runs
            .iter()
            .map(|run| usage::UsageRun {
                id: run.id.clone(),
                project_id: run.project_id.clone(),
                project_name: self
                    .projects
                    .iter()
                    .find(|project| project.id == run.project_id)
                    .map(|project| project.name.clone())
                    .unwrap_or_else(|| run.project_id.clone()),
                agent_id: run.agent_id.clone(),
                agent_name: run.agent_name.clone(),
                issue_id: run.issue_id.clone(),
                started_at_ms: run.started_at_ms,
                archived: run.is_archived(),
            })
            .collect::<Vec<_>>();
        usage::build_usage_page(
            &self.usage_query,
            self.now_ms,
            usage::local_offset_secs(),
            &runs,
            &self.usage_samples,
        )
    }

    pub(crate) fn ingest_telemetry(&mut self) {
        let incoming: Vec<TelemetrySample> = self
            .agents
            .iter()
            .flat_map(|agent| agent.drain_telemetry())
            .collect();
        if incoming.is_empty() {
            return;
        }
        let mut seen = Vec::new();
        for mut sample in incoming {
            let Some(run) = self.runs.iter().find(|run| run.id == sample.run_id) else {
                continue;
            };
            sample.project_id = run.project_id.clone();
            sample.agent_id = run.agent_id.clone();
            if sample.at_ms == 0 {
                sample.at_ms = self.now_ms;
            }
            if !seen.contains(&sample.run_id) {
                seen.push(sample.run_id.clone());
            }
            self.usage_samples.push(sample);
        }
        let keep_after = self.now_ms.saturating_sub(usage::SAMPLE_RETENTION_MS);
        self.usage_samples
            .retain(|sample| sample.at_ms >= keep_after);
        for run_id in seen {
            self.pending_events.push(HostEvent::Telemetry { run_id });
        }
        let _ = self.persist_usage_samples();
    }

    pub(crate) fn open_usage_for_run(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if run.is_archived() {
            return Err(KernelError::Denied("run is archived".into()));
        }
        self.usage_open = true;
        self.usage_query.highlighted_run_id = Some(run_id.to_string());
        Ok(())
    }

    pub(crate) fn open_run_from_usage(&mut self, run_id: &str) -> Result<(), KernelError> {
        let run = self
            .runs
            .iter()
            .find(|run| run.id == run_id)
            .cloned()
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        if run.is_archived() {
            return Err(KernelError::Denied("run is archived".into()));
        }
        self.usage_open = false;
        self.usage_query.highlighted_run_id = None;
        self.focused_run_id = Some(run.id.clone());
        if self
            .projects
            .iter()
            .any(|project| project.id == run.project_id)
        {
            self.focused_project_id = Some(run.project_id.clone());
        }
        if let Some(issue_id) = run.issue_id {
            self.selected_issue_id = Some(issue_id);
        }
        Ok(())
    }
}
