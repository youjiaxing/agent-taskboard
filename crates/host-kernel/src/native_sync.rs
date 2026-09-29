use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crate::agent::NativeSyncLaunchContext;
use crate::run::{NativeSyncDesiredState, NativeSyncState, NativeSyncSummary, RunSummary};
use crate::{HostKernel, KernelError};
use serde::{Deserialize, Serialize};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_OUTPUT: usize = 16 * 1024;
const MAX_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone)]
pub(crate) struct NativeSyncJob {
    pub(crate) run_id: String,
    pub(crate) revision: u64,
    pub(crate) session_id: String,
    pub(crate) desired_state: NativeSyncDesiredState,
    pub(crate) context: NativeSyncLaunchContext,
}

#[derive(Debug, Clone)]
pub(crate) enum NativeSyncOutcome {
    Synced,
    Unsupported(String),
    Failed(String),
}

#[derive(Debug, Clone)]
pub(crate) struct NativeSyncResult {
    pub(crate) run_id: String,
    pub(crate) revision: u64,
    pub(crate) outcome: NativeSyncOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeSyncPrivate {
    pub(crate) revision: u64,
    pub(crate) context: NativeSyncLaunchContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredRunRecord {
    #[serde(flatten)]
    pub(crate) summary: RunSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) native_sync_private: Option<NativeSyncPrivate>,
}

pub(crate) struct NativeSyncQueue {
    pub(crate) jobs: mpsc::Sender<NativeSyncJob>,
    pub(crate) results: mpsc::Receiver<NativeSyncResult>,
}

pub(crate) fn spawn_worker(lock_dir: PathBuf) -> NativeSyncQueue {
    let (job_tx, job_rx) = mpsc::channel::<NativeSyncJob>();
    let (result_tx, result_rx) = mpsc::channel::<NativeSyncResult>();
    thread::Builder::new()
        .name("native-session-sync".into())
        .spawn(move || {
            let _ = fs::create_dir_all(&lock_dir);
            while let Ok(job) = job_rx.recv() {
                let outcome = execute(&lock_dir, &job);
                if result_tx
                    .send(NativeSyncResult {
                        run_id: job.run_id,
                        revision: job.revision,
                        outcome,
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .expect("native sync worker");
    NativeSyncQueue {
        jobs: job_tx,
        results: result_rx,
    }
}

impl HostKernel {
    pub(crate) fn prepare_native_sync_intent(
        &self,
        run: &RunSummary,
        private: &mut BTreeMap<String, NativeSyncPrivate>,
        desired_state: NativeSyncDesiredState,
        now_ms: u64,
    ) -> NativeSyncSummary {
        let revision = private
            .get(&run.id)
            .map(|record| record.revision.saturating_add(1))
            .unwrap_or(1);
        let agent = self.agents.iter().find(|agent| agent.id() == run.agent_id);
        let (state, last_error) = if agent.is_none_or(|agent| !agent.native_sync_supported()) {
            (
                NativeSyncState::Unsupported,
                Some("native sync unsupported".into()),
            )
        } else if run.native_session_id.is_none() {
            (
                NativeSyncState::MissingNativeSession,
                Some("native session id was not captured".into()),
            )
        } else if !private.contains_key(&run.id) {
            (
                NativeSyncState::Failed,
                Some("legacy-context-unavailable".into()),
            )
        } else {
            (NativeSyncState::Pending, None)
        };
        if let Some(previous) = private.get_mut(&run.id) {
            previous.revision = revision;
        }
        NativeSyncSummary {
            desired_state,
            state,
            attempts: 0,
            next_retry_at_ms: (state == NativeSyncState::Pending).then_some(now_ms),
            last_error,
        }
    }

    pub(crate) fn reconcile_native_sync(&mut self) {
        self.collect_native_sync_results();
        let candidates = self
            .runs
            .iter()
            .filter_map(|run| {
                let sync = run.native_sync.as_ref()?;
                let due = sync.state == NativeSyncState::Pending
                    || (sync.state == NativeSyncState::Failed
                        && sync.next_retry_at_ms.is_some_and(|at| at <= self.now_ms));
                due.then_some(run.id.clone())
            })
            .collect::<Vec<_>>();
        for run_id in candidates {
            if self.native_sync_inflight.contains(&run_id) {
                continue;
            }
            let Some(run) = self.runs.iter().find(|run| run.id == run_id).cloned() else {
                continue;
            };
            let Some(sync) = run.native_sync.clone() else {
                continue;
            };
            let Some(session_id) = run.native_session_id.clone() else {
                continue;
            };
            let Some(private) = self.native_sync_private.get(&run.id).cloned() else {
                continue;
            };
            let mut runs = self.runs.clone();
            let Some(updated) = runs.iter_mut().find(|item| item.id == run.id) else {
                continue;
            };
            updated.native_sync = Some(NativeSyncSummary {
                desired_state: sync.desired_state,
                state: NativeSyncState::Syncing,
                attempts: sync.attempts,
                next_retry_at_ms: None,
                last_error: sync.last_error.clone(),
            });
            if self
                .commit_run_records_with_private(runs, self.native_sync_private.clone())
                .is_err()
            {
                continue;
            }
            if self
                .native_sync_jobs
                .send(NativeSyncJob {
                    run_id: run.id.clone(),
                    revision: private.revision,
                    session_id,
                    desired_state: sync.desired_state,
                    context: private.context,
                })
                .is_ok()
            {
                self.native_sync_inflight.insert(run.id);
            } else {
                let mut failed = self.runs.clone();
                if let Some(updated) = failed.iter_mut().find(|item| item.id == run.id) {
                    updated.native_sync = Some(NativeSyncSummary {
                        desired_state: sync.desired_state,
                        state: NativeSyncState::Failed,
                        attempts: sync.attempts.saturating_add(1),
                        next_retry_at_ms: Some(self.now_ms.saturating_add(1_000)),
                        last_error: Some("native sync worker unavailable".into()),
                    });
                }
                let _ =
                    self.commit_run_records_with_private(failed, self.native_sync_private.clone());
            }
        }
    }

    pub(crate) fn collect_native_sync_results(&mut self) {
        while let Ok(result) = self.native_sync_results.try_recv() {
            self.native_sync_inflight.remove(&result.run_id);
            let Some(private) = self.native_sync_private.get(&result.run_id) else {
                continue;
            };
            if private.revision != result.revision {
                continue;
            }
            let Some(index) = self.runs.iter().position(|run| run.id == result.run_id) else {
                continue;
            };
            let Some(current) = self.runs[index].native_sync.clone() else {
                continue;
            };
            if current.state != NativeSyncState::Syncing {
                continue;
            }
            let mut runs = self.runs.clone();
            let updated = &mut runs[index];
            updated.native_sync = Some(match result.outcome {
                NativeSyncOutcome::Synced => NativeSyncSummary {
                    desired_state: current.desired_state,
                    state: NativeSyncState::Synced,
                    attempts: current.attempts,
                    next_retry_at_ms: None,
                    last_error: None,
                },
                NativeSyncOutcome::Unsupported(error) => NativeSyncSummary {
                    desired_state: current.desired_state,
                    state: NativeSyncState::Unsupported,
                    attempts: current.attempts,
                    next_retry_at_ms: None,
                    last_error: Some(error),
                },
                NativeSyncOutcome::Failed(error) => {
                    let attempts = current.attempts.saturating_add(1);
                    NativeSyncSummary {
                        desired_state: current.desired_state,
                        state: NativeSyncState::Failed,
                        attempts,
                        next_retry_at_ms: (attempts < MAX_ATTEMPTS).then_some(
                            self.now_ms
                                .saturating_add(1_000u64.saturating_mul(1u64 << attempts.min(10))),
                        ),
                        last_error: Some(error),
                    }
                }
            });
            let _ = self.commit_run_records_with_private(runs, self.native_sync_private.clone());
        }
    }

    pub(crate) fn retry_native_sync(&mut self, run_id: &str) -> Result<(), KernelError> {
        let mut runs = self.runs.clone();
        let run = runs
            .iter_mut()
            .find(|run| run.id == run_id)
            .ok_or_else(|| KernelError::Protocol("unknown run".into()))?;
        let desired_state = if run.is_archived() {
            NativeSyncDesiredState::Archived
        } else {
            NativeSyncDesiredState::Active
        };
        let mut private = self.native_sync_private.clone();
        run.native_sync =
            Some(self.prepare_native_sync_intent(run, &mut private, desired_state, self.now_ms));
        self.commit_run_records_with_private(runs, private)?;
        Ok(())
    }
}

fn execute(lock_dir: &Path, job: &NativeSyncJob) -> NativeSyncOutcome {
    if let Err(error) = fs::create_dir_all(lock_dir) {
        return NativeSyncOutcome::Failed(format!(
            "could not create native sync lease dir: {error}"
        ));
    }
    let lock_path = lock_dir.join(format!("{}.lock", job.run_id));
    let Ok(mut lease) = claim_lease(&lock_path) else {
        return NativeSyncOutcome::Failed("native sync lease is busy".into());
    };

    let subcommand = match job.desired_state {
        NativeSyncDesiredState::Archived => "archive",
        NativeSyncDesiredState::Active => "unarchive",
    };
    let help = match run_cli(&job.context, &[subcommand, "--help"], &mut lease) {
        Ok(output) => output,
        Err(error) if error.starts_with("unsupported:") => {
            release_lease(&lock_path, &mut lease);
            return NativeSyncOutcome::Unsupported(error);
        }
        Err(error) => {
            release_lease(&lock_path, &mut lease);
            return NativeSyncOutcome::Failed(error);
        }
    };
    if !help.to_ascii_lowercase().contains(subcommand) {
        release_lease(&lock_path, &mut lease);
        return NativeSyncOutcome::Unsupported(format!(
            "Codex CLI does not advertise `{subcommand}`"
        ));
    }

    let result = run_cli(&job.context, &[subcommand, &job.session_id], &mut lease);
    release_lease(&lock_path, &mut lease);
    match result {
        Ok(_) => NativeSyncOutcome::Synced,
        Err(error) => NativeSyncOutcome::Failed(error),
    }
}

fn claim_lease(path: &Path) -> Result<std::fs::File, std::io::Error> {
    match OpenOptions::new().create_new(true).write(true).open(path) {
        Ok(mut file) => {
            let _ = writeln!(file, "{}", std::process::id());
            Ok(file)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if reclaim_stale_lease(path) {
                let _ = fs::remove_file(path);
                OpenOptions::new().create_new(true).write(true).open(path)
            } else {
                Err(error)
            }
        }
        Err(error) => Err(error),
    }
}

fn reclaim_stale_lease(path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(path) else {
        return false;
    };
    let mut lines = contents.lines();
    let Ok(host_pid) = lines.next().unwrap_or_default().trim().parse::<u32>() else {
        return false;
    };
    if process_alive(host_pid) {
        return false;
    }
    if let Ok(child_pid) = lines.next().unwrap_or_default().trim().parse::<u32>() {
        kill_orphaned_child(child_pid);
    }
    true
}

fn process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(true)
    }
    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}")])
            .output();
        output
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
            .unwrap_or(true)
    }
    #[cfg(not(any(unix, windows)))]
    {
        true
    }
}

fn kill_orphaned_child(pid: u32) {
    #[cfg(unix)]
    {
        let group = format!("-{pid}");
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &group])
            .status();
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status();
    }
}

fn release_lease(path: &Path, lease: &mut std::fs::File) {
    let _ = lease.flush();
    let _ = fs::remove_file(path);
}

fn run_cli(
    context: &NativeSyncLaunchContext,
    args: &[&str],
    lease: &mut std::fs::File,
) -> Result<String, String> {
    let cwd = if context.cwd.is_dir() {
        &context.cwd
    } else {
        &context.fallback_cwd
    };
    let mut command = Command::new(&context.program);
    command
        .args(&context.argv_prefix)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(&context.environment)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run native sync CLI: {error}"))?;
    lease
        .set_len(0)
        .and_then(|_| lease.seek(SeekFrom::Start(0)))
        .and_then(|_| writeln!(lease, "{}\n{}", std::process::id(), child.id()))
        .and_then(|_| lease.flush())
        .map_err(|error| format!("could not update native sync lease: {error}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = thread::spawn(move || read_limited(stdout));
    let stderr_reader = thread::spawn(move || read_limited(stderr));
    let status = wait_child(&mut child)?;
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if !status.success() {
        let detail = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };
        if detail.contains("unrecognized subcommand")
            || detail.contains("unknown subcommand")
            || detail.contains("Found argument") && detail.contains("wasn't expected")
        {
            return Err(format!("unsupported: {detail}"));
        }
        return Err(if detail.is_empty() {
            format!("native sync CLI exited with {status}")
        } else {
            detail
        });
    }
    Ok(stdout)
}

fn wait_child(child: &mut Child) -> Result<std::process::ExitStatus, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < COMMAND_TIMEOUT => {
                thread::sleep(Duration::from_millis(25));
            }
            Ok(None) => {
                kill_child(child);
                return Err("native sync CLI timed out".into());
            }
            Err(error) => {
                kill_child(child);
                return Err(format!("could not wait for native sync CLI: {error}"));
            }
        }
    }
}

fn kill_child(child: &mut Child) {
    #[cfg(unix)]
    {
        let group = format!("-{}", child.id());
        let _ = Command::new("/bin/kill")
            .args(["-KILL", "--", &group])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn read_limited<R: Read>(reader: Option<R>) -> String {
    let Some(reader) = reader else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = reader.take(MAX_OUTPUT as u64).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn native_sync_runs_archive_with_saved_context() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir().unwrap();
        let executable = tmp.path().join("codex");
        fs::write(
            &executable,
            "#!/bin/sh\nif [ \"$1\" = \"archive\" ] && [ \"$2\" = \"--help\" ]; then echo archive; exit 0; fi\nif [ \"$1\" = \"archive\" ]; then echo \"$2\" > \"$SYNC_RESULT\"; exit 0; fi\nexit 1\n",
        )
        .unwrap();
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
        let result_path = tmp.path().join("result");
        let context = NativeSyncLaunchContext {
            program: executable,
            argv_prefix: Vec::new(),
            cwd: tmp.path().to_path_buf(),
            fallback_cwd: tmp.path().to_path_buf(),
            environment: BTreeMap::from([(
                "SYNC_RESULT".into(),
                result_path.display().to_string(),
            )]),
        };
        let job = NativeSyncJob {
            run_id: "run-1".into(),
            revision: 3,
            session_id: "session-1".into(),
            desired_state: NativeSyncDesiredState::Archived,
            context,
        };
        assert!(matches!(
            execute(&tmp.path().join("locks"), &job),
            NativeSyncOutcome::Synced
        ));
        assert_eq!(fs::read_to_string(result_path).unwrap().trim(), "session-1");
    }
}
