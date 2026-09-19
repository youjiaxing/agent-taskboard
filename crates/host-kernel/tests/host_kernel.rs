use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod common;

use common::boot_req;
use host_kernel::{
    bind_local_rpc, local_client_origin_allowed, spawn_local_rpc, AppearancePreference, Command,
    EmptyAction, HostKernel, HostMode, IssueRecord, Language, LoopbackAssets, LoopbackPage,
    LoopbackServer, MemoryTracker, ProcessIntent, SystemAppearance, LOCAL_RPC_PORT,
};
use std::path::Path;

#[path = "host_kernel/boot.rs"]
mod boot;
#[path = "host_kernel/loopback.rs"]
mod loopback;
#[path = "host_kernel/pairing.rs"]
mod pairing;

fn http_get(addr: SocketAddr, origin: Option<&str>, path: &str) -> (u16, String, String) {
    let origin_header = origin
        .map(|origin| format!("Origin: {origin}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\n{origin_header}Connection: close\r\n\r\n",
        addr.port()
    );
    let (status, headers, body) = http_exchange(addr, &request);
    (status, headers, body)
}

fn http_post(addr: SocketAddr, origin: &str, body: &str) -> (u16, String) {
    http_rpc(
        addr,
        origin,
        &format!("127.0.0.1:{}", addr.port()),
        None,
        body,
    )
}

fn http_post_without_origin(addr: SocketAddr, body: &str) -> (u16, String) {
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        addr.port(),
        body.len()
    );
    let (status, _, body) = http_exchange(addr, &request);
    (status, body)
}

fn http_rpc(
    addr: SocketAddr,
    origin: &str,
    host: &str,
    token: Option<&str>,
    body: &str,
) -> (u16, String) {
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\n{auth}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let (status, _, body) = http_exchange(addr, &request);
    (status, body)
}

fn http_exchange(addr: SocketAddr, request: &str) -> (u16, String, String) {
    let mut last_err = None;
    for _ in 0..50 {
        match TcpStream::connect_timeout(&addr, Duration::from_millis(200)) {
            Ok(mut stream) => {
                stream.write_all(request.as_bytes()).unwrap();
                let _ = stream.shutdown(std::net::Shutdown::Write);
                let mut buf = String::new();
                stream.read_to_string(&mut buf).unwrap();
                let (head, body) = buf.split_once("\r\n\r\n").unwrap_or((buf.as_str(), ""));
                let status = head
                    .lines()
                    .next()
                    .unwrap_or("")
                    .split_whitespace()
                    .nth(1)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0);
                return (status, head.to_string(), body.to_string());
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!("connect {addr} failed: {last_err:?}");
}
