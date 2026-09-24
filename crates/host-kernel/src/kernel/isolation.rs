use super::super::*;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Instant;

pub(crate) struct IsolationProbe {
    started: Instant,
    receiver: Receiver<Option<PathBuf>>,
}

pub(crate) fn pending_directory_note(language: Language) -> String {
    match language {
        Language::ZhCn => "尚未确认隔离执行目录。请在官方终端检查创建结果，稍后重试；也可停止并重新启动。".into(),
        Language::En => "The isolated work directory is not confirmed. Check its creation in the official terminal and retry, or stop and start again.".into(),
    }
}

impl HostKernel {
    pub(crate) fn isolation_directory_unconfirmed(&self, run: &RunSummary) -> bool {
        run.isolation_pending.is_some()
            || (run.isolated
                && self.projects.iter().any(|project| {
                    project.id == run.project_id
                        && launch::same_path(&project.local_path, Path::new(&run.working_directory))
                }))
    }

    pub(crate) fn isolation_tree_conflicts(&self, run: &RunSummary, tree: &Path) -> bool {
        let position = self.runs.iter().position(|item| item.id == run.id);
        let project = self
            .projects
            .iter()
            .find(|project| project.id == run.project_id);
        self.runs.iter().enumerate().any(|(index, other)| {
            if other.id == run.id {
                return false;
            }
            if other.isolated && launch::same_path(Path::new(&other.working_directory), tree) {
                return true;
            }
            let same_project = self.projects.iter().any(|candidate| {
                candidate.id == other.project_id
                    && project.is_some_and(|project| {
                        launch::same_path(&project.local_path, &candidate.local_path)
                    })
            });
            let candidate_was_absent = other
                .isolation_pending
                .as_ref()
                .is_some_and(|before| !before.iter().any(|path| launch::same_path(path, tree)));
            // Ended failed launches must not block new work. Conversely, an old
            // pending Run must never adopt a tree first seen after a later launch.
            same_project
                && candidate_was_absent
                && (other.is_active() || position.is_some_and(|position| index > position))
        })
    }

    pub(crate) fn discover_isolated_directories(&mut self) {
        let pending: Vec<_> = self
            .runs
            .iter()
            .filter(|run| run.isolation_pending.is_some())
            .cloned()
            .collect();
        self.isolation_probes
            .retain(|id, _| pending.iter().any(|run| run.id == *id));
        for run in pending {
            let Some(project) = self
                .projects
                .iter()
                .find(|project| project.id == run.project_id)
            else {
                continue;
            };
            let project_dir = project.local_path.clone();
            let probe = self
                .isolation_probes
                .get(&run.id)
                .map(|probe| (probe.receiver.try_recv(), probe.started.elapsed()));
            if let Some((result, elapsed)) = probe {
                match result {
                    Ok(Some(tree)) => {
                        // A new tree cannot safely identify either of two concurrent
                        // launches that both observed the same pre-launch tree list.
                        let ambiguous = self.isolation_tree_conflicts(&run, &tree);
                        if tree.exists() && !ambiguous {
                            let mut runs = self.runs.clone();
                            let current = runs
                                .iter_mut()
                                .find(|current| current.id == run.id)
                                .expect("pending Run");
                            current.working_directory = tree.display().to_string();
                            current.isolation_pending = None;
                            current.isolation_note = None;
                            // Keep the commit captured at launch, even if the Agent
                            // has already committed by the time discovery completes.
                            for baseline in &mut current.git_baselines {
                                baseline.path = tree.display().to_string();
                                baseline.display_path = tree.display().to_string();
                            }
                            match self.commit_run_records(runs) {
                                Ok(()) => {
                                    self.isolation_probes.remove(&run.id);
                                    continue;
                                }
                                Err(_) => return,
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => continue,
                    Ok(None) | Err(TryRecvError::Disconnected) => {}
                }
                if elapsed < Duration::from_secs(1) {
                    continue;
                }
            }
            let Some(agent) = self
                .agents
                .iter()
                .find(|agent| agent.id() == run.agent_id)
                .cloned()
            else {
                continue;
            };
            let before = run.isolation_pending.expect("pending tree list");
            let (sender, receiver) = mpsc::channel();
            // Local git/Adapter I/O must not hold the Host mutex or delay navigation.
            if std::thread::Builder::new()
                .name("run-isolation-probe".into())
                .spawn(move || {
                    let tree = agent
                        .isolation_tree_after_launch(&project_dir, &before)
                        .or_else(|| launch::new_git_worktree(&project_dir, &before));
                    let _ = sender.send(tree);
                })
                .is_ok()
            {
                self.isolation_probes.insert(
                    run.id,
                    IsolationProbe {
                        started: Instant::now(),
                        receiver,
                    },
                );
            }
        }
    }
}
