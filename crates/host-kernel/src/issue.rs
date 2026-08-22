use serde::{Deserialize, Serialize};

pub const DEFAULT_TRIAGE_LABELS: &[(&str, TriageRole)] = &[
    ("needs-triage", TriageRole::NeedsTriage),
    ("needs-info", TriageRole::NeedsInfo),
    ("ready-for-agent", TriageRole::ReadyForAgent),
    ("ready-for-human", TriageRole::ReadyForHuman),
    ("wontfix", TriageRole::Wontfix),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TriageRole {
    NeedsTriage,
    NeedsInfo,
    ReadyForAgent,
    ReadyForHuman,
    Wontfix,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRef {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub open: Option<bool>,
}

impl IssueRef {
    pub fn id(&self) -> String {
        issue_id(&self.repository, self.number)
    }

    pub fn new(repository: impl Into<String>, number: u64, title: impl Into<String>) -> Self {
        Self {
            repository: repository.into(),
            number,
            title: title.into(),
            open: None,
        }
    }

    pub fn with_open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum DependencyRef {
    Known(IssueRef),
    Unclear {
        repository: Option<String>,
        number: Option<u64>,
    },
}

impl DependencyRef {
    pub fn unfinished(&self) -> bool {
        match self {
            Self::Unclear { .. } => true,
            Self::Known(issue) => issue.open.unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueRecord {
    pub repository: String,
    pub number: u64,
    pub title: String,
    pub url: String,
    pub open: bool,
    pub closed_at: Option<String>,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub parent: Option<IssueRef>,
    pub children: Vec<IssueRef>,
    pub blocked_by: Vec<DependencyRef>,
    pub blocking: Vec<IssueRef>,
}

impl IssueRecord {
    pub fn open(repository: impl Into<String>, number: u64, title: impl Into<String>) -> Self {
        let repository = repository.into();
        let title = title.into();
        Self {
            url: format!("https://github.com/{repository}/issues/{number}"),
            repository,
            number,
            title,
            open: true,
            closed_at: None,
            assignees: Vec::new(),
            labels: Vec::new(),
            parent: None,
            children: Vec::new(),
            blocked_by: Vec::new(),
            blocking: Vec::new(),
        }
    }

    pub fn id(&self) -> String {
        issue_id(&self.repository, self.number)
    }

    pub fn closed_at(mut self, closed_at: impl Into<String>) -> Self {
        self.open = false;
        self.closed_at = Some(closed_at.into());
        self
    }

    pub fn assignee(mut self, login: impl Into<String>) -> Self {
        self.assignees.push(login.into());
        self
    }

    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.labels.push(name.into());
        self
    }

    pub fn parent(
        mut self,
        repository: impl Into<String>,
        number: u64,
        title: impl Into<String>,
    ) -> Self {
        self.parent = Some(IssueRef::new(repository, number, title).with_open(true));
        self
    }

    pub fn child(
        mut self,
        repository: impl Into<String>,
        number: u64,
        title: impl Into<String>,
    ) -> Self {
        self.children
            .push(IssueRef::new(repository, number, title).with_open(true));
        self
    }

    pub fn blocked_by(
        mut self,
        repository: impl Into<String>,
        number: u64,
        title: impl Into<String>,
        open: bool,
    ) -> Self {
        self.blocked_by.push(DependencyRef::Known(
            IssueRef::new(repository, number, title).with_open(open),
        ));
        self
    }

    pub fn blocked_by_unclear(mut self, repository: impl Into<String>, number: u64) -> Self {
        self.blocked_by.push(DependencyRef::Unclear {
            repository: Some(repository.into()),
            number: Some(number),
        });
        self
    }

    pub fn blocking(
        mut self,
        repository: impl Into<String>,
        number: u64,
        title: impl Into<String>,
    ) -> Self {
        self.blocking
            .push(IssueRef::new(repository, number, title).with_open(true));
        self
    }

    pub fn claimed(&self) -> bool {
        !self.assignees.is_empty()
    }

    pub fn unfinished_blocker(&self) -> bool {
        self.blocked_by.iter().any(DependencyRef::unfinished)
    }

    pub fn triage_role(&self) -> Option<TriageRole> {
        triage_role_from_labels(&self.labels)
    }
}

pub fn issue_id(repository: &str, number: u64) -> String {
    format!("{repository}#{number}")
}

pub fn triage_role_from_labels(labels: &[String]) -> Option<TriageRole> {
    labels.iter().find_map(|label| {
        DEFAULT_TRIAGE_LABELS
            .iter()
            .find(|(name, _)| *name == label)
            .map(|(_, role)| *role)
    })
}

pub fn label_mapping_active(issues: &[IssueRecord]) -> bool {
    issues.iter().any(|issue| issue.triage_role().is_some())
}
