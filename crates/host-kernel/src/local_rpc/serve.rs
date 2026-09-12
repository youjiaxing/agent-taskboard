use std::fs;
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{CommandOutcome, HostKernel, KernelError};

use super::http::*;
use super::*;

fn kernel_error_response(error: &KernelError) -> (u16, String) {
    match error {
        KernelError::Conflict(conflict) => (
            409,
            serde_json::json!({
                "error": "issue-conflict",
                "issueId": conflict.issue_id,
                "fields": conflict.fields,
                "latest": conflict.latest,
            })
            .to_string(),
        ),
        KernelError::Protocol(_) | KernelError::Json(_) => (
            400,
            serde_json::json!({ "error": error.to_string() }).to_string(),
        ),
        KernelError::Denied(_) => (
            403,
            serde_json::json!({ "error": error.to_string() }).to_string(),
        ),
        KernelError::Io(_) => (
            500,
            serde_json::json!({ "error": error.to_string() }).to_string(),
        ),
    }
}

fn spawn_background_refreshes(
    kernel: Arc<Mutex<HostKernel>>,
    refreshes: Vec<crate::kernel::refresh::PreparedRefresh>,
) {
    let pending = Arc::new(Mutex::new(Some(refreshes)));
    let worker_pending = Arc::clone(&pending);
    let worker_kernel = Arc::clone(&kernel);
    let spawned = std::thread::Builder::new()
        .name("host-project-refresh".into())
        .spawn(move || {
            let refreshes = worker_pending
                .lock()
                .ok()
                .and_then(|mut pending| pending.take())
                .unwrap_or_default();
            for refresh in refreshes {
                let completed = HostKernel::execute_prepared_refresh(refresh);
                let Ok(mut host) = worker_kernel.lock() else {
                    return;
                };
                host.finish_prepared_refresh(completed);
            }
        });
    if spawned.is_err() {
        let refreshes = pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.take())
            .unwrap_or_default();
        if let Ok(mut host) = kernel.lock() {
            host.cancel_prepared_refreshes(&refreshes);
        }
    }
}

pub(super) fn serve_connection(
    mut stream: TcpStream,
    kernel: &Arc<Mutex<HostKernel>>,
    assets: &LoopbackAssets,
    server_port: u16,
) -> io::Result<Option<CommandOutcome>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let peer = stream.peer_addr().ok();
    let request = read_request(&mut stream)?;
    let origin = request.header("origin");
    let local_origin = local_client_origin_allowed_for_port(origin, server_port);
    let allowed = authorize(&request, kernel, peer, server_port)?;
    let response_origin = origin.filter(|_| {
        local_origin
            || (request.method == "OPTIONS" && origin.is_some_and(origin_is_http))
            || (allowed
                && (is_redeem_rpc(&request)
                    || bearer_token(&request).is_some_and(|token| {
                        kernel
                            .lock()
                            .map(|host| host.pairing_token_valid(token))
                            .unwrap_or(false)
                    })))
    });
    if !allowed {
        let message = kernel
            .lock()
            .ok()
            .map(|host| host.snapshot().copy.pairing_required)
            .unwrap_or_else(|| {
                "Access via Tailscale, LAN, or another site needs a long-term token.".into()
            });
        let body = serde_json::json!({
            "error": "pairing required",
            "message": message,
        });
        write_json(&mut stream, 403, response_origin, &body.to_string())?;
        return Ok(None);
    }

    if request.method == "OPTIONS" {
        write_empty(&mut stream, 204, response_origin)?;
        return Ok(None);
    }

    if let Some(outcome) = serve_run_io(&mut stream, &request, response_origin, kernel)? {
        return Ok(outcome);
    }

    if request.method == "GET" || request.method == "HEAD" {
        let host_running = kernel
            .lock()
            .map(|host| host.snapshot().running)
            .unwrap_or(false);
        if !host_running {
            write_json(
                &mut stream,
                404,
                response_origin,
                r#"{"error":"not found"}"#,
            )?;
        } else {
            serve_loopback_get(&mut stream, &request, response_origin, assets)?;
        }
        return Ok(None);
    }

    if request.method == "POST" && (request.path == "/rpc" || request.path == "/rpc/") {
        let value: serde_json::Value = match serde_json::from_slice(&request.body) {
            Ok(value) => value,
            Err(err) => {
                write_json(
                    &mut stream,
                    400,
                    response_origin,
                    &format!(
                        r#"{{"error":{}}}"#,
                        serde_json::to_string(&err.to_string()).unwrap()
                    ),
                )?;
                return Ok(None);
            }
        };
        let background_refreshes = request_backgrounds_refreshes(&value);
        let defer_refreshes = request_defers_refreshes(&value);
        let defer_issue_document = request_defers_issue_document(&value);
        let defer_issue_write = request_defers_issue_write(&value);
        let (initial, refreshes, issue_documents, issue_writes, remote_calls) = {
            let mut host = kernel
                .lock()
                .map_err(|_| io::Error::other("kernel lock poisoned"))?;
            host.begin_deferred_remote_calls(&value);
            if defer_refreshes {
                host.begin_deferred_refreshes();
            }
            if defer_issue_document {
                host.begin_deferred_issue_documents();
            }
            if defer_issue_write {
                host.begin_deferred_issue_writes();
            }
            let result = host.handle(value.clone());
            let remote_calls = host.take_deferred_remote_calls();
            let refreshes = if defer_refreshes {
                host.take_deferred_refreshes()
            } else {
                Vec::new()
            };
            let issue_documents = if defer_issue_document {
                host.take_deferred_issue_documents()
            } else {
                Vec::new()
            };
            let issue_writes = if defer_issue_write {
                host.take_deferred_issue_writes()
            } else {
                Vec::new()
            };
            if result.is_err() {
                host.cancel_prepared_refreshes(&refreshes);
                host.cancel_prepared_issue_documents(&issue_documents);
            }
            (
                result,
                refreshes,
                issue_documents,
                issue_writes,
                remote_calls,
            )
        };
        match initial {
            Ok(mut outcome) => {
                for call in remote_calls {
                    let mut events = std::mem::take(&mut outcome.events);
                    let (prepared, response) = call.execute();
                    let response = match response {
                        Ok(response) => response,
                        Err(error) => {
                            let (status, body) = kernel_error_response(&error);
                            write_json(&mut stream, status, response_origin, &body)?;
                            return Ok(None);
                        }
                    };
                    let mut host = kernel
                        .lock()
                        .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                    let applied = host
                        .finish_remote_call(prepared, &response)
                        .map_err(|error| io::Error::other(error.to_string()))?;
                    outcome = host.handle(serde_json::json!({
                        "op": "snapshot", "clientInstanceId": value.get("clientInstanceId").and_then(|value| value.as_str()).unwrap_or_default(),
                    })).map_err(|error| io::Error::other(error.to_string()))?;
                    events.append(&mut outcome.events);
                    outcome.events = events;
                    if applied {
                        if let Some(inference) = response.get("inference").cloned() {
                            outcome.inference = serde_json::from_value(inference).ok();
                        }
                        if let Some(view) = response.get("viewChanges").cloned() {
                            outcome.view_changes = serde_json::from_value(view).ok();
                        }
                    }
                }
                if background_refreshes && !refreshes.is_empty() {
                    let body = serde_json::to_string(&outcome.to_json())?;
                    write_json(&mut stream, 200, response_origin, &body)?;
                    spawn_background_refreshes(Arc::clone(kernel), refreshes);
                    return Ok(Some(outcome));
                }
                if !refreshes.is_empty() {
                    let mut events = std::mem::take(&mut outcome.events);
                    let completed: Vec<_> = refreshes
                        .into_iter()
                        .map(HostKernel::execute_prepared_refresh)
                        .collect();
                    let snapshot_request = serde_json::json!({
                        "op": "snapshot",
                        "clientInstanceId": value
                            .get("clientInstanceId")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                    });
                    let mut host = kernel
                        .lock()
                        .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                    for refresh in completed {
                        host.finish_prepared_refresh(refresh);
                    }
                    let mut refreshed = host
                        .handle(snapshot_request)
                        .map_err(|err| io::Error::other(err.to_string()))?;
                    events.append(&mut refreshed.events);
                    refreshed.events = events;
                    outcome = refreshed;
                }
                if !issue_documents.is_empty() {
                    let mut events = std::mem::take(&mut outcome.events);
                    let completed: Vec<_> = issue_documents
                        .into_iter()
                        .map(HostKernel::execute_prepared_issue_document)
                        .collect();
                    let mut host = kernel
                        .lock()
                        .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                    for document in completed {
                        host.finish_prepared_issue_document(document);
                    }
                    let snapshot_request = serde_json::json!({
                        "op": "snapshot",
                        "clientInstanceId": value
                            .get("clientInstanceId")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                    });
                    let mut refreshed = host
                        .handle(snapshot_request)
                        .map_err(|err| io::Error::other(err.to_string()))?;
                    events.append(&mut refreshed.events);
                    refreshed.events = events;
                    outcome = refreshed;
                }
                if !issue_writes.is_empty() {
                    let completed: Vec<_> = issue_writes
                        .into_iter()
                        .map(HostKernel::execute_prepared_issue_write)
                        .collect();
                    let mut host = kernel
                        .lock()
                        .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                    for write in completed {
                        if let Err(error) = host.finish_prepared_issue_write(write) {
                            let (status, body) = kernel_error_response(&error);
                            write_json(&mut stream, status, response_origin, &body)?;
                            return Ok(None);
                        }
                    }
                    let mut events = std::mem::take(&mut outcome.events);
                    let snapshot_request = serde_json::json!({
                        "op": "snapshot",
                        "clientInstanceId": value
                            .get("clientInstanceId")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                    });
                    let mut refreshed = host
                        .handle(snapshot_request)
                        .map_err(|err| io::Error::other(err.to_string()))?;
                    events.append(&mut refreshed.events);
                    refreshed.events = events;
                    outcome = refreshed;
                }
                let body = serde_json::to_string(&outcome.to_json())?;
                write_json(&mut stream, 200, response_origin, &body)?;
                return Ok(Some(outcome));
            }
            Err(err) => {
                let (status, body) = kernel_error_response(&err);
                write_json(&mut stream, status, response_origin, &body)?;
                return Ok(None);
            }
        }
    }

    write_json(
        &mut stream,
        404,
        response_origin,
        r#"{"error":"not found"}"#,
    )?;
    Ok(None)
}

pub(super) fn serve_run_io(
    stream: &mut TcpStream,
    request: &HttpRequest,
    origin: Option<&str>,
    kernel: &Mutex<HostKernel>,
) -> io::Result<Option<Option<CommandOutcome>>> {
    let Some((run_id, action)) = parse_run_route(&request.path) else {
        return Ok(None);
    };
    match (request.method.as_str(), action.as_str()) {
        ("GET", "output") => {
            let after = query_param(&request.path, "after")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(0);
            let session = {
                let host = kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                host.pty_session(&run_id)
            };
            let session = match session {
                Ok(session) => session,
                Err(_) => {
                    write_json(stream, 404, origin, r#"{"error":"unknown run"}"#)?;
                    return Ok(Some(None));
                }
            };
            let chunk = session.read_after(after, Duration::from_secs(8));
            if let Some(code) = chunk.exit_code {
                if let Ok(mut host) = kernel.lock() {
                    host.note_run_exit(&run_id, code);
                }
            }
            let body = serde_json::json!({
                "offset": chunk.offset,
                "data": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &chunk.data),
                "recentOutput": session.recent_output(),
                "exited": chunk.exit_code,
            });
            write_json(stream, 200, origin, &body.to_string())?;
            Ok(Some(None))
        }
        ("POST", "input") => {
            let data = pty_input_bytes(&request.body);
            let result = {
                let host = kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                host.write_pty(&run_id, &data)
            };
            match result {
                Ok(()) => {
                    write_json(stream, 200, origin, r#"{"ok":true}"#)?;
                    Ok(Some(None))
                }
                Err(KernelError::Protocol(_)) => {
                    write_json(stream, 404, origin, r#"{"error":"unknown run"}"#)?;
                    Ok(Some(None))
                }
                Err(err) => {
                    write_json(
                        stream,
                        500,
                        origin,
                        &format!(
                            r#"{{"error":{}}}"#,
                            serde_json::to_string(&err.to_string()).unwrap()
                        ),
                    )?;
                    Ok(Some(None))
                }
            }
        }
        ("POST", "resize") => {
            let cols = serde_json::from_slice::<serde_json::Value>(&request.body)
                .ok()
                .and_then(|value| value.get("cols").and_then(|v| v.as_u64()))
                .unwrap_or(80) as u16;
            let rows = serde_json::from_slice::<serde_json::Value>(&request.body)
                .ok()
                .and_then(|value| value.get("rows").and_then(|v| v.as_u64()))
                .unwrap_or(24) as u16;
            let result = {
                let host = kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?;
                host.resize_pty(&run_id, cols, rows)
            };
            match result {
                Ok(()) => {
                    write_json(stream, 200, origin, r#"{"ok":true}"#)?;
                    Ok(Some(None))
                }
                Err(_) => {
                    write_json(stream, 404, origin, r#"{"error":"unknown run"}"#)?;
                    Ok(Some(None))
                }
            }
        }
        _ => Ok(None),
    }
}

pub(super) fn parse_run_route(path: &str) -> Option<(String, String)> {
    let path = path.split('?').next().unwrap_or(path);
    let mut parts = path.trim_start_matches('/').split('/');
    if parts.next()? != "runs" {
        return None;
    }
    let id = parts.next()?.to_string();
    let action = parts.next()?.to_string();
    if id.is_empty() || action.is_empty() {
        return None;
    }
    Some((id, action))
}

pub(super) fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(value.to_string());
        }
    }
    None
}

pub(super) fn pty_input_bytes(body: &[u8]) -> Vec<u8> {
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(body) {
        if let Some(data) = value.get("data").and_then(|value| value.as_str()) {
            return data.as_bytes().to_vec();
        }
        if let Some(text) = value.get("text").and_then(|value| value.as_str()) {
            return text.as_bytes().to_vec();
        }
    }
    body.to_vec()
}

pub(super) fn serve_loopback_get(
    stream: &mut TcpStream,
    request: &HttpRequest,
    origin: Option<&str>,
    assets: &LoopbackAssets,
) -> io::Result<()> {
    let head_only = request.method == "HEAD";
    match assets {
        LoopbackAssets::Builtin => {
            let path = request.path.split('?').next().unwrap_or("/");
            if path == "/" || path == "/index.html" {
                write_bytes(
                    stream,
                    200,
                    origin,
                    "text/html; charset=utf-8",
                    BUILTIN_LOOPBACK_PAGE.as_bytes(),
                    head_only,
                )
            } else {
                write_json(stream, 404, origin, r#"{"error":"not found"}"#)
            }
        }
        LoopbackAssets::Directory(root) => {
            let Some(rel) = safe_rel_path(&request.path) else {
                return write_json(stream, 404, origin, r#"{"error":"not found"}"#);
            };
            let candidate = root.join(&rel);
            let file = if candidate.is_file() {
                candidate
            } else if rel.as_os_str() == "index.html" || rel.extension().is_none() {
                root.join("index.html")
            } else {
                return write_json(stream, 404, origin, r#"{"error":"not found"}"#);
            };
            if !file.is_file() {
                return write_json(stream, 404, origin, r#"{"error":"not found"}"#);
            }
            let body = fs::read(&file)?;
            write_bytes(stream, 200, origin, mime_of(&file), &body, head_only)
        }
        LoopbackAssets::DevProxy { origin: upstream } => {
            proxy_loopback_get(stream, request, upstream)
        }
    }
}

pub(super) fn safe_rel_path(path: &str) -> Option<PathBuf> {
    let path = path.split('?').next().unwrap_or(path);
    let path = path.split('#').next().unwrap_or(path);
    let path = path.trim_start_matches('/');
    if path.is_empty() {
        return Some(PathBuf::from("index.html"));
    }
    let mut out = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            _ => return None,
        }
    }
    Some(out)
}

pub(super) fn mime_of(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" | "ts" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) fn proxy_loopback_get(
    stream: &mut TcpStream,
    request: &HttpRequest,
    upstream_origin: &str,
) -> io::Result<()> {
    let Some(addr) = origin_socket_addr(upstream_origin) else {
        return write_json(stream, 502, None, r#"{"error":"dev proxy origin invalid"}"#);
    };
    let host = origin_host(upstream_origin);
    let mut upstream = match TcpStream::connect_timeout(&addr, Duration::from_secs(2)) {
        Ok(stream) => stream,
        Err(_) => {
            return write_json(stream, 502, None, r#"{"error":"dev proxy unavailable"}"#);
        }
    };
    upstream.set_read_timeout(Some(Duration::from_secs(5)))?;
    upstream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let mut forwarded = format!(
        "{} {} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n",
        request.method, request.path
    );
    for (name, value) in &request.headers {
        if name == "host" || name == "connection" {
            continue;
        }
        forwarded.push_str(name);
        forwarded.push_str(": ");
        forwarded.push_str(value);
        forwarded.push_str("\r\n");
    }
    forwarded.push_str("\r\n");
    upstream.write_all(forwarded.as_bytes())?;
    if !request.body.is_empty() {
        upstream.write_all(&request.body)?;
    }
    io::copy(&mut upstream, stream)?;
    Ok(())
}

pub(super) fn origin_host(origin: &str) -> &str {
    origin
        .split("://")
        .nth(1)
        .unwrap_or(origin)
        .split('/')
        .next()
        .unwrap_or(origin)
}

pub(super) fn origin_socket_addr(origin: &str) -> Option<SocketAddr> {
    let hostport = origin_host(origin);
    if let Ok(addr) = hostport.parse() {
        return Some(addr);
    }
    let hostport = if hostport.contains(':') {
        hostport.to_string()
    } else {
        format!("{hostport}:80")
    };
    hostport.parse().ok()
}
