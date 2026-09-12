mod common;

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{
    boot as boot_base, boot_req, boot_seam as boot_seam_base, make_dir,
    register_project as register, ReadMode, SeamTracker,
};
use host_kernel::{
    BoardEmptyReason, HostEvent, HostKernel, IssueRecord, KernelError, LoopbackServer,
    MemoryTracker, RefreshStatus, DEFAULT_REFRESH_INTERVAL_MS,
};

fn boot(root: &Path, tracker: Arc<MemoryTracker>) -> HostKernel {
    boot_base(root, tracker)
}

fn boot_seam(root: &Path, tracker: Arc<SeamTracker>) -> HostKernel {
    boot_seam_base(root, tracker)
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

fn snapshot_path(host: &HostKernel, project_id: &str) -> std::path::PathBuf {
    host.snapshot()
        .data
        .host_dir
        .join("projects")
        .join(project_id)
        .join("tracker-snapshot")
}

fn post_rpc_response(protocol_url: &str, body: serde_json::Value) -> (u16, serde_json::Value) {
    let address: SocketAddr = protocol_url
        .strip_prefix("http://")
        .expect("loopback protocol")
        .parse()
        .expect("loopback address");
    let body = body.to_string();
    let mut stream = TcpStream::connect(address).unwrap();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {address}\r\nOrigin: tauri://localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    (status, serde_json::from_str(body).unwrap())
}

fn post_rpc(protocol_url: &str, body: serde_json::Value) -> serde_json::Value {
    let (status, body) = post_rpc_response(protocol_url, body);
    assert_eq!(status, 200, "{body}");
    body
}

#[path = "refresh/documents.rs"]
mod documents;
#[path = "refresh/persist.rs"]
mod persist;
#[path = "refresh/polling.rs"]
mod polling;
