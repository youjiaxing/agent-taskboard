use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, DependencyRef, GitHubTracker, HostKernel, IssueEdit, MemoryTracker, ProbeContext,
    ScriptedGitHub, SystemAppearance, TrackerPort,
};

fn scripted_host(issues: Vec<serde_json::Value>) -> HostKernel {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().to_path_buf();
    std::mem::forget(tmp);
    let dir = root.join("work/garden");
    std::fs::create_dir_all(&dir).unwrap();
    let tracker = GitHubTracker::scripted(ScriptedGitHub {
        env: [("GH_TOKEN".into(), "tok".into())].into(),
        accept_tokens: ["tok".into()].into(),
        issues: BTreeMap::from([("you/garden".into(), issues)]),
        ..Default::default()
    });
    let mut host = HostKernel::boot_with(
        BootRequest {
            app_local_data_dir: root,
            app_log_dir: dir.parent().unwrap().join("logs"),
            system_locale: "en-US".into(),
            system_appearance: SystemAppearance::Light,
            host_display_name: "Studio".into(),
        },
        Arc::new(tracker),
    )
    .unwrap();
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "garden",
        "localPath": dir,
        "repository": "you/garden",
    }))
    .unwrap();
    host
}

fn node(extra: serde_json::Value) -> serde_json::Value {
    let mut base = serde_json::json!({
        "number": 50,
        "title": "GitHub 四列看板与 Frontier",
        "state": "OPEN",
        "url": "https://github.com/you/garden/issues/50",
        "repository": { "nameWithOwner": "you/garden" },
        "assignees": { "nodes": [{ "login": "ada" }] },
        "labels": { "nodes": [{ "name": "ready-for-agent" }] },
        "parent": {
            "number": 45,
            "title": "spec",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        },
        "subIssues": { "nodes": [{
            "number": 51,
            "title": "child",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        }] },
        "issueDependenciesSummary": { "blockedBy": 1 },
        "blockedBy": { "nodes": [{
            "number": 12,
            "title": "gate",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/other" }
        }] },
        "blocking": { "nodes": [] },
        "relatedIssues": { "nodes": [{
            "number": 99,
            "title": "related not blocking",
            "state": "OPEN",
            "repository": { "nameWithOwner": "you/garden" }
        }] },
        "body": "Blocked by: #77\n\n- [ ] #51"
    });
    if let (Some(base_obj), Some(extra_obj)) = (base.as_object_mut(), extra.as_object()) {
        for (key, value) in extra_obj {
            base_obj.insert(key.clone(), value.clone());
        }
    }
    base
}

fn plain_node(number: u64, title: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        "state": "OPEN",
        "url": format!("https://github.com/you/garden/issues/{number}"),
        "repository": { "nameWithOwner": "you/garden" },
        "assignees": { "nodes": [] },
        "labels": { "nodes": [] },
        "parent": null,
        "subIssues": { "nodes": [] },
        "issueDependenciesSummary": { "blockedBy": 0 },
        "blockedBy": { "nodes": [] },
        "blocking": { "nodes": [] },
    })
}

fn issue_ref(number: u64, title: &str) -> serde_json::Value {
    serde_json::json!({
        "number": number,
        "title": title,
        "state": "OPEN",
        "repository": { "nameWithOwner": "you/garden" },
    })
}

fn probe_ctx<'a>(github_host: &'a str, repository: &'a str) -> ProbeContext<'a> {
    ProbeContext {
        tracker: host_kernel::TrackerKind::Github,
        github_host,
        repository,
        secrets_pat: None,
        secrets_path: Path::new("/tmp/host-secrets.json"),
    }
}

fn scripted(script: ScriptedGitHub) -> GitHubTracker {
    GitHubTracker::scripted(script)
}

#[path = "github_adapter/mapping.rs"]
mod mapping;
#[path = "github_adapter/pagination.rs"]
mod pagination;
