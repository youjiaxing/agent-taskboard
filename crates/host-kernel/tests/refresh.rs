mod common;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use common::{ReadMode, SeamTracker};
use host_kernel::{
    BoardEmptyReason, BootRequest, HostEvent, HostKernel, IssueRecord, KernelError, KernelPorts,
    LoopbackServer, MemoryAgent, MemoryLaunchEnv, MemorySessionFactory, MemoryTracker,
    ProbeContext, ProbeOutcome, RefreshStatus, RunStatus, SystemAppearance, TrackerReadError,
    TrackerReadOutcome, TrackerSeam, TrackerWriteError, TrackerWriteOp,
    DEFAULT_REFRESH_INTERVAL_MS, PENDING_CONFIRM_MS,
};

struct BlockingTracker {
    inner: SeamTracker,
    gate: Mutex<(Option<BlockedCall>, bool)>,
    changed: Condvar,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BlockedCall {
    Read,
    Probe,
    Write,
}

impl BlockingTracker {
    fn new() -> Self {
        Self {
            inner: SeamTracker::new(),
            gate: Mutex::new((None, false)),
            changed: Condvar::new(),
        }
    }

    fn add_issue(&self, issue: IssueRecord) {
        self.inner.add_issue(issue);
    }

    fn set_issues(&self, repository: &str, issues: Vec<IssueRecord>) {
        self.inner.set_issues(repository, issues);
    }

    fn block_reads(&self) {
        self.block(BlockedCall::Read);
    }

    fn block_probes(&self) {
        self.block(BlockedCall::Probe);
    }

    fn block_writes(&self) {
        self.block(BlockedCall::Write);
    }

    fn block(&self, call: BlockedCall) {
        *self.gate.lock().unwrap() = (Some(call), false);
    }

    fn wait_until_blocked(&self) {
        let mut gate = self.gate.lock().unwrap();
        while !gate.1 {
            gate = self.changed.wait(gate).unwrap();
        }
    }

    fn release(&self) {
        let mut gate = self.gate.lock().unwrap();
        gate.0 = None;
        self.changed.notify_all();
    }

    fn wait_for_release(&self, call: BlockedCall) {
        let mut gate = self.gate.lock().unwrap();
        if gate.0 == Some(call) {
            gate.1 = true;
            self.changed.notify_all();
            while gate.0 == Some(call) {
                gate = self.changed.wait(gate).unwrap();
            }
        }
    }
}

impl TrackerSeam for BlockingTracker {
    fn probe(&self, ctx: &ProbeContext<'_>) -> ProbeOutcome {
        self.wait_for_release(BlockedCall::Probe);
        self.inner.probe(ctx)
    }

    fn read_all(&self, ctx: &ProbeContext<'_>) -> Result<TrackerReadOutcome, TrackerReadError> {
        self.wait_for_release(BlockedCall::Read);
        self.inner.read_all(ctx)
    }

    fn read_issue_document(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: &str,
    ) -> Result<host_kernel::IssueDocument, TrackerReadError> {
        self.wait_for_release(BlockedCall::Read);
        self.inner.read_issue_document(ctx, issue_id)
    }

    fn write_issue(
        &self,
        ctx: &ProbeContext<'_>,
        issue_id: Option<&str>,
        op: &TrackerWriteOp,
    ) -> Result<IssueRecord, TrackerWriteError> {
        self.wait_for_release(BlockedCall::Write);
        self.inner.write_issue(ctx, issue_id, op)
    }
}

fn post_rpc(url: &str, body: &str) -> String {
    let address = url.trim_start_matches("http://");
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(
        stream,
        "POST /rpc HTTP/1.1\r\nHost: {address}\r\nOrigin: {url}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len(),
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn rpc_json(response: &str) -> serde_json::Value {
    let (_, body) = response
        .split_once("\r\n\r\n")
        .expect("RPC response must contain a body");
    serde_json::from_str(body).expect("RPC response body must be JSON")
}

fn boot_req(root: &Path) -> BootRequest {
    BootRequest {
        app_local_data_dir: root.to_path_buf(),
        app_log_dir: root.join("logs"),
        system_locale: "zh-Hans-CN".into(),
        system_appearance: SystemAppearance::Light,
        host_display_name: "Studio".into(),
    }
}

fn make_dir(root: &Path, name: &str) -> std::path::PathBuf {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn boot(root: &Path, tracker: Arc<MemoryTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn boot_seam(root: &Path, tracker: Arc<SeamTracker>) -> HostKernel {
    HostKernel::boot_with(boot_req(root), tracker).unwrap()
}

fn register(host: &mut HostKernel, dir: &Path, name: &str, repository: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repository,
    }))
    .unwrap()
    .snapshot
    .focused_project_id
}

fn frontier_ids(host: &HostKernel) -> Vec<String> {
    host.snapshot()
        .board
        .unwrap()
        .columns
        .unwrap()
        .frontier
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

fn refresh_status(host: &HostKernel) -> RefreshStatus {
    host.snapshot().board.unwrap().refresh
}

fn boot_with_runs(
    root: &Path,
    tracker: Arc<BlockingTracker>,
) -> (HostKernel, Arc<MemorySessionFactory>) {
    let sessions = MemorySessionFactory::new();
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker,
            agents: vec![Arc::new(MemoryAgent::installed_grok())],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")),
            sessions: Arc::clone(&sessions) as _,
        },
    )
    .unwrap();
    (host, sessions)
}

fn start_bound_run(host: &mut HostKernel, project_id: &str, issue_id: &str) -> String {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "issueId": issue_id,
        "agentId": "grok-build",
        "values": {
            "model": "grok-4.6",
            "effort": "high",
            "permission-mode": "normal",
            "always-approve": "false",
            "sandbox": "off",
            "initial-instruction": "",
            "additional-args": ""
        },
        "openingText": issue_id,
    }))
    .unwrap()
    .snapshot
    .runs
    .iter()
    .rev()
    .find(|run| run.issue_id.as_deref() == Some(issue_id) && run.status == RunStatus::Running)
    .expect("bound Run must start")
    .id
    .clone()
}

#[test]
fn loopback_snapshot_stays_responsive_while_tracker_refresh_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker.clone()).unwrap();
    register(&mut host, &dir, "garden", "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    tracker.block_reads();
    let refresh_url = url.clone();
    let refresh = std::thread::spawn(move || post_rpc(&refresh_url, r#"{"op":"refresh"}"#));
    tracker.wait_until_blocked();

    let (sent, received) = std::sync::mpsc::channel();
    let snapshot_url = url.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let response = post_rpc(&snapshot_url, r#"{"op":"snapshot"}"#);
        let _ = sent.send((started.elapsed(), response));
    });
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    let _ = refresh.join();
    let (elapsed, response) = responsive.expect("snapshot exceeded the 250ms navigation gate");
    assert!(
        elapsed <= Duration::from_millis(250),
        "snapshot took {elapsed:?}"
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

#[test]
fn loopback_snapshot_stays_responsive_while_issue_document_load_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker.clone()).unwrap();
    register(&mut host, &dir, "garden", "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    tracker.block_reads();
    let document_url = url.clone();
    let document = std::thread::spawn(move || {
        post_rpc(
            &document_url,
            r#"{"op":"loadIssueDocument","issueId":"you/garden#1"}"#,
        )
    });
    tracker.wait_until_blocked();

    let (sent, received) = std::sync::mpsc::channel();
    let snapshot_url = url.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let response = post_rpc(&snapshot_url, r#"{"op":"snapshot"}"#);
        let _ = sent.send((started.elapsed(), response));
    });
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    let _ = document.join();
    let (elapsed, response) = responsive.expect("snapshot exceeded the 250ms document-load gate");
    assert!(
        elapsed <= Duration::from_millis(250),
        "snapshot took {elapsed:?}"
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

#[test]
fn loopback_snapshot_stays_responsive_while_action_refresh_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker.clone()).unwrap();
    register(&mut host, &dir, "garden", "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    tracker.block_reads();
    let claim_url = url.clone();
    let claim = std::thread::spawn(move || {
        post_rpc(
            &claim_url,
            r#"{"op":"claimIssue","issueId":"you/garden#1"}"#,
        )
    });
    tracker.wait_until_blocked();

    let (sent, received) = std::sync::mpsc::channel();
    let snapshot_url = url.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let response = post_rpc(&snapshot_url, r#"{"op":"snapshot"}"#);
        let _ = sent.send((started.elapsed(), response));
    });
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    let claim_response = claim.join().unwrap();
    let (elapsed, response) = responsive.expect("snapshot exceeded the 250ms action gate");
    assert!(
        elapsed <= Duration::from_millis(250),
        "snapshot took {elapsed:?}"
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        claim_response.starts_with("HTTP/1.1 200"),
        "{claim_response}"
    );
}

#[test]
fn loopback_snapshot_stays_responsive_while_run_exit_refresh_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "running"));
    let (mut host, sessions) = boot_with_runs(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    start_bound_run(&mut host, &project_id, "you/garden#1");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    sessions.last_session().unwrap().finish(0);
    tracker.block_reads();
    let exit_url = url.clone();
    let exit_observer = std::thread::spawn(move || post_rpc(&exit_url, r#"{"op":"snapshot"}"#));
    tracker.wait_until_blocked();

    let received = begin_snapshot_measurement(&url);
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    assert!(exit_observer.join().unwrap().starts_with("HTTP/1.1 200"));
    assert_navigation_gate_result(responsive, "Run-exit refresh");
}

#[test]
fn loopback_snapshot_stays_responsive_while_pending_auto_advance_refresh_is_blocked() {
    const T0: u64 = 1_000_000;

    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "first").label("ready-for-agent"),
            IssueRecord::open("you/garden", 2, "second").label("ready-for-agent"),
        ],
    );
    let (mut host, sessions) = boot_with_runs(tmp.path(), Arc::clone(&tracker));
    host.handle(serde_json::json!({ "op": "tick", "nowMs": T0 }))
        .unwrap();
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "setHostAutoAdvance",
        "enabled": true,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setProjectAutoAdvance",
        "projectId": project_id,
        "enabled": true,
    }))
    .unwrap();
    start_bound_run(&mut host, &project_id, "you/garden#1");
    sessions.last_session().unwrap().set_session_end(true);
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "first")
                .closed_at("2026-08-29T00:00:00Z")
                .assignee("me")
                .label("ready-for-agent"),
            IssueRecord::open("you/garden", 2, "second").label("ready-for-agent"),
        ],
    );
    sessions.last_session().unwrap().finish(0);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert!(host.snapshot().pending_confirmation.is_some());
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();

    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();
    tracker.block_reads();
    let tick_url = url.clone();
    let tick = std::thread::spawn(move || {
        post_rpc(
            &tick_url,
            &serde_json::json!({
                "op": "tick",
                "nowMs": T0 + PENDING_CONFIRM_MS,
            })
            .to_string(),
        )
    });
    tracker.wait_until_blocked();

    let received = begin_snapshot_measurement(&url);
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    assert!(tick.join().unwrap().starts_with("HTTP/1.1 200"));
    assert_navigation_gate_result(responsive, "pending auto-advance refresh");
    let mut snapshot = kernel.lock().unwrap().snapshot();
    for _ in 0..100 {
        if snapshot.pending_confirmation.is_none() && sessions.spawn_count() == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
        snapshot = kernel.lock().unwrap().snapshot();
    }
    assert!(snapshot.pending_confirmation.is_none());
    assert_eq!(sessions.spawn_count(), 2);
    assert!(snapshot.runs.iter().any(|run| {
        run.issue_id.as_deref() == Some("you/garden#2") && run.status == RunStatus::Running
    }));
}

#[test]
fn loopback_snapshot_stays_responsive_while_pending_auto_advance_claim_is_blocked() {
    const T0: u64 = 1_000_000;

    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "first").label("ready-for-agent"),
            IssueRecord::open("you/garden", 2, "second").label("ready-for-agent"),
        ],
    );
    let (mut host, sessions) = boot_with_runs(tmp.path(), Arc::clone(&tracker));
    host.handle(serde_json::json!({ "op": "tick", "nowMs": T0 }))
        .unwrap();
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "setHostAutoAdvance",
        "enabled": true,
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "setProjectAutoAdvance",
        "projectId": project_id,
        "enabled": true,
    }))
    .unwrap();
    start_bound_run(&mut host, &project_id, "you/garden#1");
    sessions.last_session().unwrap().set_session_end(true);
    tracker.set_issues(
        "you/garden",
        vec![
            IssueRecord::open("you/garden", 1, "first")
                .closed_at("2026-08-29T00:00:00Z")
                .assignee("me")
                .label("ready-for-agent"),
            IssueRecord::open("you/garden", 2, "second").label("ready-for-agent"),
        ],
    );
    sessions.last_session().unwrap().finish(0);
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap();
    assert!(host.snapshot().pending_confirmation.is_some());
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();

    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();
    tracker.block_writes();
    let tick_url = url.clone();
    let tick = std::thread::spawn(move || {
        post_rpc(
            &tick_url,
            &serde_json::json!({
                "op": "tick",
                "nowMs": T0 + PENDING_CONFIRM_MS,
            })
            .to_string(),
        )
    });
    tracker.wait_until_blocked();

    let received = begin_snapshot_measurement(&url);
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    assert!(tick.join().unwrap().starts_with("HTTP/1.1 200"));
    assert_navigation_gate_result(responsive, "pending auto-advance Claim");
    let mut snapshot = kernel.lock().unwrap().snapshot();
    for _ in 0..100 {
        if snapshot.pending_confirmation.is_none() && sessions.spawn_count() == 2 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
        snapshot = kernel.lock().unwrap().snapshot();
    }
    assert!(snapshot.pending_confirmation.is_none());
    assert_eq!(sessions.spawn_count(), 2);
    assert!(snapshot.runs.iter().any(|run| {
        run.issue_id.as_deref() == Some("you/garden#2") && run.status == RunStatus::Running
    }));
}

#[test]
fn loopback_snapshot_stays_responsive_while_remote_rpc_is_blocked() {
    let remote_root = tempfile::tempdir().unwrap();
    let remote_dir = make_dir(remote_root.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "remote ready"));
    let mut remote_host =
        HostKernel::boot_with(boot_req(remote_root.path()), tracker.clone()).unwrap();
    let project_id = register(&mut remote_host, &remote_dir, "garden", "you/garden");
    let remote_kernel = Arc::new(Mutex::new(remote_host));
    let remote_server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&remote_kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let remote_url = remote_server.protocol_url().to_string();
    let pairing_code = remote_kernel
        .lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": remote_url.trim_end_matches('/'),
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap()
        .code;

    let client_root = tempfile::tempdir().unwrap();
    let mut client = HostKernel::boot(boot_req(client_root.path())).unwrap();
    let paired = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": remote_url.trim_end_matches('/'),
            "code": pairing_code,
        }))
        .unwrap();
    let remote_id = paired
        .snapshot
        .hosts
        .iter()
        .find(|host| !host.local)
        .unwrap()
        .id
        .clone();
    let client_kernel = Arc::new(Mutex::new(client));
    let client_server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&client_kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let client_url = client_server.protocol_url().to_string();

    tracker.block_reads();
    let claim_url = client_url.clone();
    let claim = std::thread::spawn(move || {
        post_rpc(
            &claim_url,
            &serde_json::json!({
                "op": "claimIssue",
                "issueId": "you/garden#1",
                "clientId": "remote-client",
                "clientView": {
                    "focusedHostId": remote_id,
                    "focusedProjectId": project_id,
                    "selectedIssueId": "you/garden#1",
                },
            })
            .to_string(),
        )
    });
    tracker.wait_until_blocked();

    let (sent, received) = std::sync::mpsc::channel();
    let snapshot_url = client_url.clone();
    std::thread::spawn(move || {
        let started = Instant::now();
        let response = post_rpc(
            &snapshot_url,
            r#"{"op":"snapshot","clientId":"local-client","clientView":{"focusedHostId":"local"}}"#,
        );
        let _ = sent.send((started.elapsed(), response));
    });
    let responsive = received.recv_timeout(Duration::from_millis(250));
    tracker.release();
    let claim_response = claim.join().unwrap();
    let (elapsed, response) = responsive.expect("snapshot exceeded the 250ms remote-RPC gate");
    assert!(
        elapsed <= Duration::from_millis(250),
        "snapshot took {elapsed:?}"
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert!(
        claim_response.starts_with("HTTP/1.1 200"),
        "{claim_response}"
    );
}

#[test]
fn loopback_snapshot_stays_responsive_while_pairing_rpc_is_blocked() {
    let remote = TcpListener::bind("127.0.0.1:0").unwrap();
    let remote_address = format!("http://{}", remote.local_addr().unwrap());
    let (accepted_tx, accepted_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let remote_thread = std::thread::spawn(move || {
        let (mut stream, _) = remote.accept().unwrap();
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).unwrap();
        assert!(
            String::from_utf8_lossy(&request[..read]).contains("redeemPairing"),
            "pairing request must reach the remote Host"
        );
        accepted_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        let body = serde_json::json!({
            "pairing": {
                "hostId": "remote-test",
                "displayName": "Remote Test",
                "token": "paired-token",
            }
        })
        .to_string();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len(),
        )
        .unwrap();
    });

    let tmp = tempfile::tempdir().unwrap();
    let host = HostKernel::boot(boot_req(tmp.path())).unwrap();
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();
    let pair_url = url.clone();
    let pairing = std::thread::spawn(move || {
        post_rpc(
            &pair_url,
            &serde_json::json!({
                "op": "pairRemoteHost",
                "address": remote_address,
                "code": "123456",
            })
            .to_string(),
        )
    });
    accepted_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    let received = begin_snapshot_measurement(&url);
    let responsive = received.recv_timeout(Duration::from_millis(250));
    release_tx.send(()).unwrap();
    remote_thread.join().unwrap();
    assert!(pairing.join().unwrap().starts_with("HTTP/1.1 200"));
    assert_navigation_gate_result(responsive, "pairing RPC");
}

#[test]
fn background_issue_document_load_uses_the_explicit_project_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let first_dir = make_dir(tmp.path(), "work/first");
    let second_dir = make_dir(tmp.path(), "work/second");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "shared issue"));
    tracker.set_issue_body("you/garden#1", "Loaded for the requested Project.");
    let mut host = boot(tmp.path(), tracker);
    let first_project = register(&mut host, &first_dir, "first", "you/garden");
    let second_project = register(&mut host, &second_dir, "second", "you/garden");
    assert_ne!(first_project, second_project);
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();
    let view = serde_json::json!({
        "clientId": "client-b",
        "focusedHostId": "local",
        "focusedProjectId": second_project,
        "selectedIssueId": "you/garden#1",
    });
    let load = post_rpc(
        &url,
        &serde_json::json!({
            "op": "loadIssueDocument",
            "issueId": "you/garden#1",
            "clientView": view,
        })
        .to_string(),
    );
    assert!(load.starts_with("HTTP/1.1 200"), "{load}");

    let mut selected_document = serde_json::Value::Null;
    for _ in 0..50 {
        let response = post_rpc(
            &url,
            &serde_json::json!({
                "op": "snapshot",
                "clientView": view,
            })
            .to_string(),
        );
        let json = rpc_json(&response);
        selected_document = json["snapshot"]["board"]["selected"]["document"].clone();
        if selected_document["kind"] == "ready" {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(selected_document["kind"], "ready", "{selected_document}");
    assert_eq!(
        selected_document["body"],
        "Loaded for the requested Project."
    );

    let first = rpc_json(&post_rpc(
        &url,
        &serde_json::json!({
            "op": "snapshot",
            "clientView": {
                "clientId": "client-a",
                "focusedHostId": "local",
                "focusedProjectId": first_project,
                "selectedIssueId": "you/garden#1",
            },
        })
        .to_string(),
    ));
    assert_eq!(
        first["snapshot"]["board"]["selected"]["document"]["kind"],
        "unloaded"
    );
}

#[test]
fn loopback_snapshot_stays_responsive_while_tracker_probe_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let first_dir = make_dir(tmp.path(), "work/garden");
    let second_dir = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker.clone()).unwrap();
    register(&mut host, &first_dir, "garden", "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    tracker.block_probes();
    let register_url = url.clone();
    let register = std::thread::spawn(move || {
        post_rpc(
            &register_url,
            &serde_json::json!({
                "op": "registerProject",
                "name": "notes",
                "localPath": second_dir,
                "repository": "you/notes",
            })
            .to_string(),
        )
    });
    tracker.wait_until_blocked();

    assert_snapshot_within_navigation_gate(&url, "probe");
    tracker.release();
    assert!(register.join().unwrap().starts_with("HTTP/1.1 200"));
}

#[test]
fn loopback_snapshot_stays_responsive_while_tracker_write_is_blocked() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(BlockingTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = HostKernel::boot_with(boot_req(tmp.path()), tracker.clone()).unwrap();
    register(&mut host, &dir, "garden", "you/garden");
    let kernel = Arc::new(Mutex::new(host));
    let server = LoopbackServer::attach_without_host_tick(
        Arc::clone(&kernel),
        0,
        host_kernel::LoopbackAssets::Builtin,
        |_| {},
    )
    .unwrap();
    let url = server.protocol_url().to_string();

    tracker.block_writes();
    let claim_url = url.clone();
    let claim = std::thread::spawn(move || {
        post_rpc(
            &claim_url,
            r#"{"op":"claimIssue","issueId":"you/garden#1"}"#,
        )
    });
    tracker.wait_until_blocked();

    assert_snapshot_within_navigation_gate(&url, "write");
    tracker.release();
    assert!(claim.join().unwrap().starts_with("HTTP/1.1 200"));
}

fn assert_snapshot_within_navigation_gate(url: &str, blocked_call: &str) {
    let received = begin_snapshot_measurement(url);
    let responsive = received.recv_timeout(Duration::from_millis(250));
    assert_navigation_gate_result(responsive, blocked_call);
}

fn begin_snapshot_measurement(url: &str) -> std::sync::mpsc::Receiver<(Duration, String)> {
    let (sent, received) = std::sync::mpsc::channel();
    let snapshot_url = url.to_string();
    std::thread::spawn(move || {
        let started = Instant::now();
        let response = post_rpc(&snapshot_url, r#"{"op":"snapshot"}"#);
        let _ = sent.send((started.elapsed(), response));
    });
    received
}

fn assert_navigation_gate_result(
    responsive: Result<(Duration, String), std::sync::mpsc::RecvTimeoutError>,
    blocked_call: &str,
) {
    let (elapsed, response) =
        responsive.unwrap_or_else(|_| panic!("snapshot exceeded the 250ms {blocked_call} gate"));
    assert!(
        elapsed <= Duration::from_millis(250),
        "snapshot took {elapsed:?} while {blocked_call} was blocked"
    );
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

fn snapshot_path(host: &HostKernel, project_id: &str) -> std::path::PathBuf {
    host.snapshot()
        .data
        .host_dir
        .join("projects")
        .join(project_id)
        .join("tracker-snapshot")
}

#[test]
fn never_fetched_project_does_not_draw_four_columns() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(BoardEmptyReason::NoData));
    assert!(board.columns.is_none());
    assert_eq!(board.refresh, RefreshStatus::NeverFetched);
}

#[test]
fn successful_refresh_persists_last_data_and_keeps_it_when_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");

    let path = snapshot_path(&host, &project_id);
    assert!(path.is_file());
    match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => assert!(fetched_at_ms > 0),
        other => panic!("expected ready, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);

    let before = std::fs::read(&path).unwrap();
    tracker.fail_read("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Offline { fetched_at_ms, .. } => assert!(fetched_at_ms > 0),
        other => panic!("expected offline, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    assert_eq!(std::fs::read(&path).unwrap(), before);

    drop(host);
    let host = boot(tmp.path(), tracker);
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    match refresh_status(&host) {
        RefreshStatus::Offline { .. } | RefreshStatus::Ready { .. } => {}
        other => panic!("expected last data after reboot, got {other:?}"),
    }
}

#[test]
fn snapshot_persistence_failure_is_reported_as_incomplete_data() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    let path = snapshot_path(&host, &project_id);
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();

    match refresh_status(&host) {
        RefreshStatus::TrackerError { detail, .. } => assert!(detail
            .as_deref()
            .is_some_and(|detail| detail.contains("could not be persisted"))),
        other => panic!("expected tracker-error, got {other:?}"),
    }
    assert!(!host.snapshot().projects[0].tracker_synced);
}

#[test]
fn refresh_emits_refreshing_then_a_terminal_status() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let out = host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let kinds: Vec<_> = out
        .events
        .iter()
        .filter_map(|event| match event {
            HostEvent::RefreshStatusChanged { status, .. } => Some(status.kind()),
            _ => None,
        })
        .collect();
    assert_eq!(kinds.first().copied(), Some("refreshing"));
    assert_eq!(kinds.last().copied(), Some("ready"));
}

#[test]
fn opening_focusing_foreground_and_manual_refresh_pull_immediately() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    let notes_id = host
        .handle(serde_json::json!({
            "op": "registerProject",
            "name": "notes",
            "localPath": notes,
            "repository": "you/notes",
        }))
        .unwrap()
        .snapshot
        .projects
        .iter()
        .find(|project| project.name == "notes")
        .unwrap()
        .id
        .clone();

    let after_register_garden = tracker.read_count("you/garden");
    let after_register_notes = tracker.read_count("you/notes");
    assert!(after_register_garden >= 1);
    assert!(after_register_notes >= 1);

    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 1);

    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({ "op": "showWindow" }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 2);

    host.handle(serde_json::json!({
        "op": "refresh",
        "projectId": notes_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/notes"), after_register_notes + 1);
    assert_eq!(tracker.read_count("you/garden"), after_register_garden + 2);
}

#[test]
fn visible_project_polls_every_sixty_seconds_and_hidden_does_not() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready {
            fetched_at_ms,
            next_refresh_in_ms,
        } => {
            assert_eq!(next_refresh_in_ms, Some(DEFAULT_REFRESH_INTERVAL_MS));
            fetched_at_ms
        }
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");

    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 30_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline);
    match refresh_status(&host) {
        RefreshStatus::Ready {
            next_refresh_in_ms, ..
        } => assert_eq!(next_refresh_in_ms, Some(30_000)),
        other => panic!("expected countdown, got {other:?}"),
    }

    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let hidden_at = match refresh_status(&host) {
        RefreshStatus::Ready {
            fetched_at_ms,
            next_refresh_in_ms,
        } => {
            assert!(next_refresh_in_ms.is_none());
            fetched_at_ms
        }
        other => panic!("expected ready without countdown, got {other:?}"),
    };
    let after_hide = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": hidden_at + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_hide);
}

#[test]
fn another_visible_client_can_keep_polling_when_window_is_hidden() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn run_end_refreshes_even_when_nobody_is_watching() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "notes",
        "localPath": notes,
        "repository": "you/notes",
    }))
    .unwrap();
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let garden_reads = tracker.read_count("you/garden");
    let notes_reads = tracker.read_count("you/notes");
    host.handle(serde_json::json!({
        "op": "noteRunEnded",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 1);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
}

#[test]
fn claim_close_check_and_auto_advance_only_refresh_the_involved_project() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let notes = make_dir(tmp.path(), "work/notes");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    tracker.add_issue(IssueRecord::open("you/notes", 2, "note"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let garden_id = register(&mut host, &garden, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": "notes",
        "localPath": notes,
        "repository": "you/notes",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "focusProject",
        "projectId": garden_id,
    }))
    .unwrap();
    let garden_reads = tracker.read_count("you/garden");
    let notes_reads = tracker.read_count("you/notes");

    host.handle(serde_json::json!({
        "op": "claimIssue",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 1);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
    assert!(frontier_ids(&host).is_empty());

    host.handle(serde_json::json!({
        "op": "checkIssueClosed",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "autoAdvance",
        "projectId": garden_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), garden_reads + 3);
    assert_eq!(tracker.read_count("you/notes"), notes_reads);
}

#[test]
fn last_data_is_not_used_to_claim_or_advance_when_read_fails() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    let path = snapshot_path(&host, &project_id);
    let before = std::fs::read(&path).unwrap();
    tracker.fail_read("you/garden");

    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("offline")));
    let release = host
        .handle(serde_json::json!({
            "op": "releaseIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(release, KernelError::Denied(_)));
    let advance = host
        .handle(serde_json::json!({
            "op": "autoAdvance",
            "projectId": project_id,
        }))
        .unwrap_err();
    assert!(matches!(advance, KernelError::Denied(_)));
    let bound = host
        .handle(serde_json::json!({
            "op": "startBoundRun",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(bound, KernelError::Denied(_)));

    let started = host
        .handle(serde_json::json!({
            "op": "startUnboundRun",
            "projectId": project_id,
        }))
        .unwrap();
    let run = started.snapshot.runs.first().expect("offline unbound Run");
    assert!(run.unbound);
    let run_id = run.id.clone();
    host.handle(serde_json::json!({
        "op": "injectRunInput",
        "runId": run_id,
        "text": "yes",
    }))
    .unwrap();
    host.handle(serde_json::json!({
        "op": "stopRun",
        "runId": run_id,
    }))
    .unwrap();

    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    assert_eq!(std::fs::read(&path).unwrap(), before);

    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
}

#[test]
fn rate_limit_pauses_auto_refresh_and_is_not_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    tracker.fail_rate_limited("you/garden", Some(120_000));
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::RateLimited {
            fetched_at_ms,
            retry_at_ms,
        } => {
            assert_eq!(fetched_at_ms, Some(fetched));
            assert_eq!(retry_at_ms, Some(fetched + 120_000));
        }
        other => panic!("expected rate-limited, got {other:?}"),
    }
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("rate-limited")));
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit);

    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 120_000,
    }))
    .unwrap();
    match refresh_status(&host) {
        RefreshStatus::Ready { .. } => {}
        other => panic!("expected ready after retry-after, got {other:?}"),
    }
}

#[test]
fn rate_limit_without_retry_after_stays_paused_until_manual_success() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    tracker.fail_rate_limited("you/garden", None);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS * 3,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit);
    tracker.clear_read_script("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Ready { .. } => {}
        other => panic!("expected manual recovery, got {other:?}"),
    }
}

#[test]
fn auth_failure_is_project_degraded_not_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    tracker.fail_auth("you/garden");
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    assert!(matches!(
        refresh_status(&host),
        RefreshStatus::AuthFailed { .. }
    ));
    assert!(matches!(
        host.snapshot().projects[0].connection,
        host_kernel::ProjectConnection::AuthFailed { .. }
    ));
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);
    let claim = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(claim, KernelError::Denied(message) if message.contains("auth")));
}

#[test]
fn check_issue_closed_uses_live_read_not_last_data() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    assert_eq!(frontier_ids(&host), vec!["you/garden#1"]);

    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready").closed_at("2026-08-22T12:00:00Z")],
    );
    host.handle(serde_json::json!({
        "op": "checkIssueClosed",
        "issueId": "you/garden#1",
    }))
    .unwrap();
    let columns = host.snapshot().board.unwrap().columns.unwrap();
    assert!(columns.frontier.is_empty());
    assert_eq!(columns.recently_completed[0].id, "you/garden#1");
}

#[test]
fn refresh_interval_is_configurable_and_survives_reboot() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({
        "op": "setRefreshInterval",
        "intervalMs": 15_000,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    drop(host);
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    match refresh_status(&host) {
        RefreshStatus::Ready {
            next_refresh_in_ms, ..
        } => assert_eq!(next_refresh_in_ms, Some(15_000)),
        other => panic!("expected persisted 15s interval, got {other:?}"),
    }
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + 15_000,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn incomplete_read_persists_snapshot_and_board_across_reboot() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready")],
    );
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("truncated at 500 issues".into()),
    );
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");

    let board = host.snapshot().board.unwrap();
    assert_eq!(board.empty, Some(BoardEmptyReason::IncompleteRead));
    assert!(board.columns.is_none());
    let fetched = match board.refresh {
        RefreshStatus::Incomplete {
            fetched_at_ms: Some(fetched),
            next_refresh_in_ms,
            detail,
        } => {
            assert_eq!(next_refresh_in_ms, Some(DEFAULT_REFRESH_INTERVAL_MS));
            assert_eq!(detail.as_deref(), Some("truncated at 500 issues"));
            fetched
        }
        other => panic!("expected incomplete, got {other:?}"),
    };

    // 快照标记不完整并保留详情
    let path = snapshot_path(&host, &project_id);
    let stored: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored["complete"], false);
    assert_eq!(stored["detail"], "truncated at 500 issues");
    assert_eq!(stored["issues"][0]["number"], 1);

    // 到点自动重试，未完整前看板一直不画 Frontier
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
    assert!(host.snapshot().board.unwrap().columns.is_none());

    // 重启后仍是不完整状态
    drop(host);
    let host = boot_seam(tmp.path(), Arc::clone(&tracker));
    let board = host.snapshot().board.unwrap();
    assert!(board.columns.is_none());
    match board.refresh {
        RefreshStatus::Incomplete { .. } => {}
        other => panic!("expected incomplete after reboot, got {other:?}"),
    }
}

#[test]
fn offline_after_incomplete_read_is_shown_as_offline() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(SeamTracker::new());
    tracker.set_issues(
        "you/garden",
        vec![IssueRecord::open("you/garden", 1, "ready")],
    );
    tracker.set_read_mode(
        "you/garden",
        ReadMode::Incomplete("truncated at 500 issues".into()),
    );
    let mut host = boot_seam(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    match refresh_status(&host) {
        RefreshStatus::Incomplete { .. } => {}
        other => panic!("expected incomplete, got {other:?}"),
    }
    tracker.set_read_mode("you/garden", ReadMode::Offline);
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    match refresh_status(&host) {
        RefreshStatus::Offline { .. } => {}
        other => panic!("expected offline after a later failed read, got {other:?}"),
    }
}

#[test]
fn run_end_refreshes_even_when_rate_limited() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    tracker.fail_rate_limited("you/garden", Some(120_000));
    host.handle(serde_json::json!({ "op": "refresh" })).unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::RateLimited { fetched_at_ms, .. } => fetched_at_ms.expect("last data"),
        other => panic!("expected rate-limited, got {other:?}"),
    };
    let after_limit = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(
        tracker.read_count("you/garden"),
        after_limit,
        "interval auto-refresh must stay paused"
    );
    host.handle(serde_json::json!({
        "op": "noteRunEnded",
        "projectId": project_id,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_limit + 1);
}

#[test]
fn stale_client_view_without_heartbeat_stops_polling() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);

    let after_poll = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS * 4,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_poll);
}

#[test]
fn tick_heartbeat_keeps_visible_client_past_ttl() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    let project_id = register(&mut host, &dir, "garden", "you/garden");
    host.handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    host.handle(serde_json::json!({
        "op": "setClientView",
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let baseline = tracker.read_count("you/garden");
    let later = fetched + DEFAULT_REFRESH_INTERVAL_MS * 4;
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": later,
        "clientId": "phone",
        "projectId": project_id,
        "visible": true,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), baseline + 1);
}

#[test]
fn host_and_client_ticks_do_not_duplicate_interval_fetch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host = boot(tmp.path(), Arc::clone(&tracker));
    register(&mut host, &dir, "garden", "you/garden");
    let fetched = match refresh_status(&host) {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
    }))
    .unwrap();
    let after_host_tick = tracker.read_count("you/garden");
    host.handle(serde_json::json!({
        "op": "tick",
        "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS + 500,
    }))
    .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_host_tick);
}

#[test]
fn tauri_client_viewing_remote_host_keeps_that_project_watched() {
    let host_dir = tempfile::tempdir().unwrap();
    let client_dir = tempfile::tempdir().unwrap();
    let garden = make_dir(host_dir.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
    let mut host_kernel = boot(host_dir.path(), Arc::clone(&tracker));
    let project_id = register(&mut host_kernel, &garden, "garden", "you/garden");
    host_kernel
        .handle(serde_json::json!({ "op": "hideWindow" }))
        .unwrap();
    let host = Arc::new(Mutex::new(host_kernel));
    let server = LoopbackServer::attach_client_transport(Arc::clone(&host), |_| {}).unwrap();
    let address = server.protocol_url().trim_end_matches('/').to_string();
    let code = host
        .lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "beginPairingOffer",
            "address": address,
        }))
        .unwrap()
        .snapshot
        .pairing_offer
        .unwrap()
        .code;

    let mut client = HostKernel::boot(boot_req(client_dir.path())).unwrap();
    let remote_id = client
        .handle(serde_json::json!({
            "op": "pairRemoteHost",
            "address": address,
            "code": code,
        }))
        .unwrap()
        .snapshot
        .hosts
        .iter()
        .find(|item| !item.local)
        .unwrap()
        .id
        .clone();
    client
        .handle(serde_json::json!({
            "op": "focusHost",
            "hostId": remote_id,
        }))
        .unwrap();

    let fetched = match host.lock().unwrap().snapshot().board.unwrap().refresh {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready, got {other:?}"),
    };
    let without_view = tracker.read_count("you/garden");
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": fetched + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();
    assert_eq!(
        tracker.read_count("you/garden"),
        without_view,
        "hidden Host window without Client heartbeat must not poll"
    );

    client
        .handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "tauri-desktop",
            "projectId": project_id,
            "visible": true,
        }))
        .unwrap();
    let after_view = tracker.read_count("you/garden");
    assert!(
        after_view > without_view,
        "visible remote Client must refresh immediately"
    );
    let watched_at = match host.lock().unwrap().snapshot().board.unwrap().refresh {
        RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
        other => panic!("expected ready after remote Client view, got {other:?}"),
    };
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": watched_at + DEFAULT_REFRESH_INTERVAL_MS,
        }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_view + 1);

    client
        .handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "tauri-desktop",
            "projectId": "",
            "visible": false,
        }))
        .unwrap();
    let after_hide = tracker.read_count("you/garden");
    host.lock()
        .unwrap()
        .handle(serde_json::json!({
            "op": "tick",
            "nowMs": watched_at + DEFAULT_REFRESH_INTERVAL_MS * 2,
        }))
        .unwrap();
    assert_eq!(tracker.read_count("you/garden"), after_hide);
}

#[test]
fn claim_without_last_data_still_live_reads_the_focused_project() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.fail_read("you/garden");
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, &dir, "garden", "you/garden");
    let err = host
        .handle(serde_json::json!({
            "op": "claimIssue",
            "issueId": "you/garden#1",
        }))
        .unwrap_err();
    assert!(matches!(err, KernelError::Denied(message) if message.contains("never-fetched")));
}
