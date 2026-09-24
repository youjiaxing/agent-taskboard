mod common;

use std::collections::BTreeMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use common::{
    boot_board as boot, boot_board_seam as boot_seam, boot_req, browser_e2e_guard, make_dir,
    pin_board_test_time, register_project as register, start_bound_grok, ReadMode, SeamTracker,
    BOARD_TEST_NOW_MS,
};
use host_kernel::{
    AgentField, AgentFieldKind, BoardEmptyReason, CenterView, DependencyGraph, DependencyGraphMode,
    FrontierEmptyReason, GraphRelation, HostKernel, IssueRecord, KernelPorts, LoopbackAssets,
    LoopbackServer, MemoryAgent, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker,
    TrackerRouter, TriageRole, DEFAULT_RECENT_LIMIT,
};

fn run_browser_e2e(host: HostKernel, script_name: &str, envs: &[(&str, &Path)]) {
    run_browser_e2e_with_outcome(host, script_name, envs, |_| {});
}

fn run_browser_e2e_with_outcome(
    host: HostKernel,
    script_name: &str,
    envs: &[(&str, &Path)],
    on_outcome: impl Fn(host_kernel::CommandOutcome) + Send + Sync + 'static,
) {
    let _browser_guard = browser_e2e_guard();
    let kernel = Arc::new(Mutex::new(host));
    let dist = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/dist")
        .canonicalize()
        .expect("built desktop client");
    let client = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        LoopbackAssets::Directory(dist),
        on_outcome,
    )
    .unwrap();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new("node");
    command
        .arg(repo.join("apps/desktop/e2e").join(script_name))
        .current_dir(&repo)
        .env("BOARD_URL", client.protocol_url().to_string());
    for (name, value) in envs {
        command.env(name, value);
    }
    let output = command.output().expect("playwright");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{script_name} browser e2e failed\nstdout:{stdout}\nstderr:{stderr}"
    );
}

fn run_browser_e2e_with_pty_disconnect(
    host: HostKernel,
    session_factory: Arc<MemorySessionFactory>,
    script_name: &str,
) {
    let disconnect_once = Arc::new(AtomicBool::new(false));
    let callback_factory = Arc::clone(&session_factory);
    let callback_once = Arc::clone(&disconnect_once);
    run_browser_e2e_with_outcome(host, script_name, &[], move |outcome| {
        let has_active_run = outcome.snapshot.runs.iter().any(|run| run.is_active());
        if has_active_run && !callback_once.swap(true, Ordering::SeqCst) {
            callback_factory
                .last_session()
                .expect("bound Run PTY")
                .disconnect();
        }
    });
}

fn run_browser_script(script_name: &str, board_url: &str, envs: &[(&str, &str)]) {
    let _browser_guard = browser_e2e_guard();
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut command = Command::new("node");
    command
        .arg(repo.join("apps/desktop/e2e").join(script_name))
        .current_dir(&repo)
        .env("BOARD_URL", board_url);
    for (name, value) in envs {
        command.env(name, value);
    }
    let output = command.output().expect("playwright");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{script_name} browser e2e failed\nstdout:{stdout}\nstderr:{stderr}"
    );
}

fn ids(cards: &[host_kernel::IssueCard]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn node_ids(graph: &host_kernel::DependencyGraph) -> Vec<String> {
    graph.nodes.iter().map(|node| node.id.clone()).collect()
}

fn edge_pairs(graph: &host_kernel::DependencyGraph) -> Vec<(String, String)> {
    graph
        .edges
        .iter()
        .map(|edge| (edge.from.clone(), edge.to.clone()))
        .collect()
}

fn node_rank(graph: &host_kernel::DependencyGraph, id: &str) -> u32 {
    graph
        .nodes
        .iter()
        .find(|node| node.id == id)
        .expect("node")
        .rank
}

#[path = "board/browser_journeys.rs"]
mod browser_journeys;
#[path = "board/browser_journeys_edge_cases.rs"]
mod browser_journeys_edge_cases;
#[path = "board/browser_journeys_recovery.rs"]
mod browser_journeys_recovery;
#[path = "board/kernel_projection.rs"]
mod kernel_projection;
#[path = "board/kernel_projection_edge_cases.rs"]
mod kernel_projection_edge_cases;
