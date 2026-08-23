use serde::{Deserialize, Serialize};

use crate::issue::{IssueRecord, TriageRole};
use crate::Language;

pub const PENDING_CONFIRM_MS: u64 = 60_000;
pub const DEFAULT_RESTORE_DELAY_MS: u64 = 60_000;

const EXCLUDED_LABELS: &[&str] = &[
    "grilling",
    "prototype",
    "needs-info",
    "ready-for-human",
    "needs-triage",
    "wayfinder:grilling",
    "wayfinder:prototype",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingConfirmation {
    pub project_id: String,
    pub issue_id: String,
    pub run_id: String,
    pub agent_id: String,
    pub deadline_ms: u64,
    pub remaining_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingAdvance {
    pub project_id: String,
    pub issue_id: String,
    pub run_id: String,
    pub agent_id: String,
    pub deadline_ms: u64,
}

impl PendingAdvance {
    pub(crate) fn to_snapshot(&self, now_ms: u64) -> PendingConfirmation {
        PendingConfirmation {
            project_id: self.project_id.clone(),
            issue_id: self.issue_id.clone(),
            run_id: self.run_id.clone(),
            agent_id: self.agent_id.clone(),
            deadline_ms: self.deadline_ms,
            remaining_ms: self.deadline_ms.saturating_sub(now_ms),
        }
    }
}

pub fn in_auto_pool(issue: &IssueRecord) -> bool {
    issue.open
        && !issue.claimed()
        && !issue.unfinished_blocker()
        && issue.triage_role() == Some(TriageRole::ReadyForAgent)
        && !excluded(issue)
}

pub fn excluded(issue: &IssueRecord) -> bool {
    issue.labels.iter().any(|label| is_excluded_label(label))
}

pub fn is_excluded_label(label: &str) -> bool {
    EXCLUDED_LABELS.contains(&label)
}

pub fn normal_completion(
    issue_closed: bool,
    hooks_attached: bool,
    session_end: bool,
    stop_failure: bool,
    process_ok: bool,
) -> bool {
    issue_closed && hooks_attached && session_end && !stop_failure && process_ok
}

pub fn self_check_text(language: Language) -> String {
    match language {
        Language::ZhCn => "请检查当前工作是否已经做完。该继续就继续，确认做完再关票。".into(),
        Language::En => "Check whether the current work is finished. Continue if it is not. Close the issue only after you confirm it is done.".into(),
    }
}

pub fn clamp_restore_delay_ms(delay_ms: u64) -> u64 {
    delay_ms.min(600_000)
}

pub fn default_restore_delay_ms() -> u64 {
    DEFAULT_RESTORE_DELAY_MS
}
