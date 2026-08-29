use std::path::Path;
use std::sync::Arc;

use host_kernel::{
    BootRequest, BucketKind, HostEvent, HostKernel, KernelPorts, MemoryAgent, MemoryLaunchEnv,
    MemorySessionFactory, MemoryTracker, RunStatus, SystemAppearance, TelemetryLane,
    TelemetrySample, TokenCounts, UsageRange,
};

const NOON: u64 = 1_787_486_400_000; // 2026-08-23 12:00:00 UTC
const HOUR: u64 = 3_600_000;
const DAY: u64 = 86_400_000;

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

struct Harness {
    host: HostKernel,
    agent: Arc<MemoryAgent>,
}

fn harness(root: &Path) -> Harness {
    let agent = Arc::new(MemoryAgent::installed_grok());
    let host = HostKernel::boot_with_ports(
        boot_req(root),
        KernelPorts {
            tracker: Arc::new(MemoryTracker::new()),
            agents: vec![Arc::clone(&agent) as _],
            launch_env: Arc::new(MemoryLaunchEnv::with_path("/mem/bin")) as _,
            sessions: MemorySessionFactory::new(),
        },
    )
    .unwrap();
    Harness { host, agent }
}

fn register(host: &mut HostKernel, dir: &Path, name: &str, repo: &str) -> String {
    host.handle(serde_json::json!({
        "op": "registerProject",
        "name": name,
        "localPath": dir,
        "repository": repo,
    }))
    .unwrap()
    .snapshot
    .projects
    .iter()
    .find(|project| project.name == name)
    .unwrap()
    .id
    .clone()
}

fn grok_values() -> serde_json::Value {
    serde_json::json!({
        "model": "grok-4.6",
        "effort": "high",
        "permission-mode": "normal",
        "always-approve": "false",
        "sandbox": "off",
        "initial-instruction": "",
        "additional-args": ""
    })
}

fn start_run(host: &mut HostKernel, project_id: &str) -> String {
    host.handle(serde_json::json!({
        "op": "startUnboundRun",
        "projectId": project_id,
        "agentId": "grok-build",
        "values": grok_values(),
        "openingText": "work",
    }))
    .unwrap()
    .snapshot
    .runs
    .iter()
    .rev()
    .find(|run| run.project_id == project_id && run.status == RunStatus::Running)
    .unwrap()
    .id
    .clone()
}

fn snapshot(host: &mut HostKernel) -> host_kernel::CommandOutcome {
    host.handle(serde_json::json!({ "op": "snapshot" }))
        .unwrap()
}

fn open_usage(host: &mut HostKernel) -> host_kernel::CommandOutcome {
    host.handle(serde_json::json!({ "op": "openUsage" }))
        .unwrap()
}

fn feed(
    agent: &MemoryAgent,
    run_id: &str,
    model: &str,
    lane: TelemetryLane,
    tokens: TokenCounts,
    ttft_ms: Option<u64>,
    tokens_per_sec: Option<u64>,
    at_ms: u64,
) {
    agent.push_telemetry(TelemetrySample {
        run_id: run_id.to_string(),
        project_id: String::new(),
        agent_id: String::new(),
        model: model.to_string(),
        lane,
        tokens,
        ttft_ms,
        tokens_per_sec,
        at_ms,
    });
}

fn tokens(
    input: Option<u64>,
    output: Option<u64>,
    cache_read: Option<u64>,
    cache_write: Option<u64>,
    reasoning: Option<u64>,
    total: Option<u64>,
) -> TokenCounts {
    TokenCounts {
        input,
        output,
        cache_read,
        cache_write,
        reasoning,
        total,
    }
}

#[test]
fn usage_opens_from_host_as_independent_page_defaulting_to_today() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();

    let before = snapshot(&mut h.host);
    assert!(!before.snapshot.usage_open);
    assert_eq!(before.snapshot.center_view, host_kernel::CenterView::Board);
    assert_eq!(before.snapshot.copy.usage, "用量");
    assert!(before.snapshot.copy.proxy_disclaimer.contains("Clash"));
    assert!(!before.snapshot.copy.proxy_disclaimer.contains("美元"));

    let out = open_usage(&mut h.host);
    assert!(out.snapshot.usage_open);
    assert_eq!(out.snapshot.center_view, host_kernel::CenterView::Board);
    assert_eq!(out.snapshot.usage.range, UsageRange::Today);
    assert_eq!(out.snapshot.usage.bucket_kind, BucketKind::Hour);
    assert!(out.snapshot.usage.runs.is_empty());
    assert_eq!(out.snapshot.usage.totals, TokenCounts::missing());
}

#[test]
fn missing_token_fields_stay_absent_not_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let run_id = start_run(&mut h.host, &project_id);
    feed(
        &h.agent,
        &run_id,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(12), None, Some(4), None, None, Some(16)),
        Some(180),
        Some(40),
        NOON,
    );
    open_usage(&mut h.host);
    let out = snapshot(&mut h.host);

    let lane = &out.snapshot.runs[0].telemetry[0];
    assert_eq!(lane.tokens.input, Some(12));
    assert_eq!(lane.tokens.output, None);
    assert_eq!(lane.tokens.cache_read, Some(4));
    assert_eq!(lane.tokens.cache_write, None);
    assert_eq!(lane.tokens.reasoning, None);
    let json = serde_json::to_value(&lane.tokens).unwrap();
    assert_eq!(json["output"], serde_json::Value::Null);
    assert!(json.get("cacheWrite").is_none() || json["cacheWrite"].is_null());
    assert_ne!(json["output"], serde_json::json!(0));

    let row = &out.snapshot.usage.runs[0];
    assert_eq!(row.tokens.output, None);
    assert_eq!(row.tokens.input, Some(12));
    assert_eq!(out.snapshot.usage.totals.output, None);
    assert_eq!(out.snapshot.usage.cache_hit_rate, Some(4.0 / 16.0));
}

#[test]
fn run_telemetry_keeps_models_on_separate_lanes() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let run_id = start_run(&mut h.host, &project_id);
    feed(
        &h.agent,
        &run_id,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(10), Some(20), None, None, None, Some(30)),
        Some(200),
        Some(50),
        NOON,
    );
    feed(
        &h.agent,
        &run_id,
        "grok-code",
        TelemetryLane::Subagent,
        tokens(Some(3), Some(7), None, None, None, Some(10)),
        Some(80),
        Some(90),
        NOON + 10,
    );
    feed(
        &h.agent,
        &run_id,
        "grok-3",
        TelemetryLane::Switched,
        tokens(Some(1), Some(1), None, None, None, Some(2)),
        Some(40),
        Some(20),
        NOON + 20,
    );
    let out = snapshot(&mut h.host);
    let telemetry = &out.snapshot.runs[0].telemetry;
    assert_eq!(telemetry.len(), 3);
    assert_eq!(telemetry[0].model, "grok-4.6");
    assert_eq!(telemetry[0].lane, TelemetryLane::Main);
    assert_eq!(telemetry[1].model, "grok-code");
    assert_eq!(telemetry[1].lane, TelemetryLane::Subagent);
    assert_eq!(telemetry[2].model, "grok-3");
    assert_eq!(telemetry[2].lane, TelemetryLane::Switched);
    assert_eq!(telemetry[0].tokens.total, Some(30));
    assert_eq!(telemetry[1].tokens.total, Some(10));
    let json = serde_json::to_value(&out.snapshot.runs[0]).unwrap();
    assert!(json.get("tokenTotal").is_none());
    assert!(json.get("totalTokens").is_none());
}

#[test]
fn bucket_total_is_missing_when_any_run_row_is_missing_a_field() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let pond = make_dir(tmp.path(), "work/pond");
    let mut h = harness(tmp.path());
    let garden_id = register(&mut h.host, &garden, "garden", "you/garden");
    let pond_id = register(&mut h.host, &pond, "pond", "you/pond");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let first = start_run(&mut h.host, &garden_id);
    let second = start_run(&mut h.host, &pond_id);
    feed(
        &h.agent,
        &first,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(10), Some(5), Some(1), Some(1), Some(2), Some(19)),
        Some(120),
        Some(40),
        NOON,
    );
    feed(
        &h.agent,
        &second,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(4), None, Some(1), Some(1), Some(0), None),
        Some(130),
        Some(30),
        NOON,
    );
    h.host
        .handle(serde_json::json!({
            "op": "setUsageRange",
            "range": "24-hours",
        }))
        .unwrap();
    open_usage(&mut h.host);
    let out = snapshot(&mut h.host);
    let bucket = out
        .snapshot
        .usage
        .buckets
        .iter()
        .find(|bucket| bucket.start_ms <= NOON && NOON < bucket.start_ms + HOUR)
        .expect("hour bucket");
    assert_eq!(bucket.tokens.input, Some(14));
    assert_eq!(bucket.tokens.output, None);
    assert_eq!(bucket.tokens.total, None);
    assert_eq!(out.snapshot.usage.totals.output, None);
}

#[test]
fn spike_uses_the_model_recent_median_not_a_fixed_threshold() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let run_id = start_run(&mut h.host, &project_id);
    for i in 0..6 {
        feed(
            &h.agent,
            &run_id,
            "grok-4.6",
            TelemetryLane::Main,
            tokens(Some(1), Some(1), None, None, None, Some(2)),
            Some(200),
            Some(100),
            NOON + i * 1_000,
        );
    }
    feed(
        &h.agent,
        &run_id,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(1), Some(1), None, None, None, Some(2)),
        Some(2_000),
        Some(100),
        NOON + 6_000,
    );
    feed(
        &h.agent,
        &run_id,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(1), Some(1), None, None, None, Some(2)),
        Some(200),
        Some(30),
        NOON + 7_000,
    );
    let out = snapshot(&mut h.host);
    let recent = &out.snapshot.runs[0].telemetry[0].recent;
    assert!(!recent[0].spike);
    assert!(recent[6].spike, "TTFT 2000ms vs median 200ms should spike");
    assert!(recent[7].spike, "rate 30 vs median 100 is a >60% drop");
    assert!(out
        .events
        .iter()
        .any(|event| matches!(event, HostEvent::Telemetry { run_id: id } if id == &run_id)));
}

#[test]
fn usage_and_run_observation_jump_both_ways() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let run_id = start_run(&mut h.host, &project_id);
    feed(
        &h.agent,
        &run_id,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(8), Some(2), None, None, None, Some(10)),
        Some(90),
        Some(40),
        NOON,
    );
    snapshot(&mut h.host);

    let to_usage = h
        .host
        .handle(serde_json::json!({
            "op": "openUsageForRun",
            "runId": run_id,
        }))
        .unwrap();
    assert!(to_usage.snapshot.usage_open);
    assert_eq!(
        to_usage.snapshot.usage.highlighted_run_id.as_deref(),
        Some(run_id.as_str())
    );
    assert!(to_usage.snapshot.usage.runs[0].highlighted);
    assert_eq!(
        to_usage.snapshot.copy.open_host_usage.contains("用量"),
        true
    );
    assert_eq!(to_usage.snapshot.copy.open_this_run.contains("Run"), true);

    let back = h
        .host
        .handle(serde_json::json!({
            "op": "openRunFromUsage",
            "runId": run_id,
        }))
        .unwrap();
    assert!(!back.snapshot.usage_open);
    assert_eq!(back.snapshot.focused_run_id, run_id);
}

#[test]
fn usage_filters_and_time_windows_do_not_mix_old_samples() {
    let tmp = tempfile::tempdir().unwrap();
    let garden = make_dir(tmp.path(), "work/garden");
    let pond = make_dir(tmp.path(), "work/pond");
    let mut h = harness(tmp.path());
    let garden_id = register(&mut h.host, &garden, "garden", "you/garden");
    let pond_id = register(&mut h.host, &pond, "pond", "you/pond");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON - 2 * DAY }))
        .unwrap();
    let old = start_run(&mut h.host, &garden_id);
    feed(
        &h.agent,
        &old,
        "grok-4.6",
        TelemetryLane::Main,
        tokens(Some(99), Some(99), None, None, None, Some(198)),
        Some(50),
        Some(10),
        NOON - 2 * DAY,
    );
    snapshot(&mut h.host);

    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let fresh = start_run(&mut h.host, &pond_id);
    feed(
        &h.agent,
        &fresh,
        "grok-code",
        TelemetryLane::Main,
        tokens(Some(5), Some(5), None, None, None, Some(10)),
        Some(70),
        Some(20),
        NOON,
    );
    snapshot(&mut h.host);

    let today = open_usage(&mut h.host);
    assert_eq!(today.snapshot.usage.range, UsageRange::Today);
    assert_eq!(today.snapshot.usage.bucket_kind, BucketKind::Hour);
    assert_eq!(today.snapshot.usage.runs.len(), 1);
    assert_eq!(today.snapshot.usage.runs[0].run_id, fresh);
    assert_eq!(today.snapshot.usage.totals.input, Some(5));

    let day24 = h
        .host
        .handle(serde_json::json!({
            "op": "setUsageRange",
            "range": "24-hours",
        }))
        .unwrap();
    assert_eq!(day24.snapshot.usage.range, UsageRange::Last24Hours);
    assert_eq!(day24.snapshot.usage.bucket_kind, BucketKind::Hour);
    assert_eq!(day24.snapshot.usage.runs.len(), 1);

    let week = h
        .host
        .handle(serde_json::json!({
            "op": "setUsageRange",
            "range": "7-days",
        }))
        .unwrap();
    assert_eq!(week.snapshot.usage.bucket_kind, BucketKind::Day);
    assert_eq!(week.snapshot.usage.runs.len(), 2);
    assert_eq!(week.snapshot.usage.runs[0].run_id, fresh);
    assert_eq!(week.snapshot.usage.runs[1].run_id, old);

    let filtered = h
        .host
        .handle(serde_json::json!({
            "op": "setUsageFilter",
            "projectId": pond_id,
            "model": "grok-code",
        }))
        .unwrap();
    assert_eq!(filtered.snapshot.usage.runs.len(), 1);
    assert_eq!(filtered.snapshot.usage.runs[0].run_id, fresh);
}

#[test]
fn each_model_keeps_only_the_last_ten_calls() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let mut h = harness(tmp.path());
    let project_id = register(&mut h.host, &dir, "garden", "you/garden");
    h.host
        .handle(serde_json::json!({ "op": "tick", "nowMs": NOON }))
        .unwrap();
    let run_id = start_run(&mut h.host, &project_id);
    for i in 0..12 {
        feed(
            &h.agent,
            &run_id,
            "grok-4.6",
            TelemetryLane::Main,
            tokens(Some(1), Some(1), None, None, None, Some(2)),
            Some(100 + i),
            Some(50),
            NOON + i * 1_000,
        );
    }
    let out = snapshot(&mut h.host);
    let recent = &out.snapshot.runs[0].telemetry[0].recent;
    assert_eq!(recent.len(), 10);
    assert_eq!(recent[0].ttft_ms, Some(102));
    assert_eq!(recent[9].ttft_ms, Some(111));
}
