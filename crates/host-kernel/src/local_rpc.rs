use std::collections::HashMap;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{CommandOutcome, HostKernel, KernelError, LoopbackKind, ProcessIntent};

pub const LOCAL_RPC_PORT: u16 = 10529;

#[derive(Debug, Clone)]
pub enum LoopbackAssets {
    Builtin,
    Directory(PathBuf),
    DevProxy { origin: String },
}

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn bind_local_rpc(preferred_port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, preferred_port))
        .or_else(|_| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)))?;
    let port = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{port}")))
}

fn bind_strict(port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let bound = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{bound}")))
}

pub struct LoopbackServer {
    protocol_url: String,
    stop: Arc<AtomicBool>,
}

impl LoopbackServer {
    pub fn attach_client_transport(
        kernel: Arc<Mutex<HostKernel>>,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let protocol_url = format!("http://127.0.0.1:{port}");
        spawn_local_rpc_inner(
            listener,
            kernel,
            LoopbackAssets::Builtin,
            Arc::clone(&stop),
            on_outcome,
            false,
        );
        Ok(Self { protocol_url, stop })
    }

    pub fn attach(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        Self::attach_with(kernel, page_port, LoopbackAssets::Builtin, on_outcome)
    }

    pub fn attach_with(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        assets: LoopbackAssets,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let running = kernel
            .lock()
            .map_err(|_| io::Error::other("kernel lock poisoned"))?
            .snapshot()
            .running;
        if !running {
            kernel
                .lock()
                .map_err(|_| io::Error::other("kernel lock poisoned"))?
                .note_loopback_page(
                    LoopbackKind::HostNotRunning,
                    if page_port == 0 {
                        LOCAL_RPC_PORT
                    } else {
                        page_port
                    },
                );
            return Ok(Self {
                protocol_url: String::new(),
                stop,
            });
        }

        match bind_strict(page_port) {
            Ok((listener, protocol_url)) => {
                let port = listener.local_addr()?.port();
                kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?
                    .note_loopback_page(LoopbackKind::Serving, port);
                spawn_local_rpc_inner(listener, kernel, assets, Arc::clone(&stop), on_outcome, true);
                Ok(Self { protocol_url, stop })
            }
            Err(_) => {
                let (listener, protocol_url) = bind_local_rpc(0)?;
                kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?
                    .note_loopback_page(
                        LoopbackKind::Occupied,
                        if page_port == 0 {
                            LOCAL_RPC_PORT
                        } else {
                            page_port
                        },
                    );
                spawn_local_rpc_inner(listener, kernel, assets, Arc::clone(&stop), on_outcome, true);
                Ok(Self { protocol_url, stop })
            }
        }
    }

    pub fn protocol_url(&self) -> &str {
        &self.protocol_url
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(addr) = self
            .protocol_url
            .trim_start_matches("http://")
            .parse::<SocketAddr>()
        {
            let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(50));
        }
    }
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn spawn_local_rpc(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
) {
    spawn_local_rpc_inner(
        listener,
        kernel,
        LoopbackAssets::Builtin,
        Arc::new(AtomicBool::new(false)),
        on_outcome,
        false,
    );
}

fn spawn_local_rpc_inner(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    assets: LoopbackAssets,
    stop: Arc<AtomicBool>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    host_tick: bool,
) {
    let on_outcome = Arc::new(on_outcome);
    if host_tick {
        let tick_kernel = Arc::clone(&kernel);
        let tick_stop = Arc::clone(&stop);
        let tick_on_outcome = Arc::clone(&on_outcome);
        let _ = std::thread::Builder::new()
            .name("host-refresh-tick".into())
            .spawn(move || {
                while !tick_stop.load(Ordering::Relaxed) && kernel_process_alive(&tick_kernel) {
                    std::thread::sleep(Duration::from_millis(1000));
                    if tick_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let outcome = {
                        let Ok(mut host) = tick_kernel.lock() else {
                            break;
                        };
                        host.dispatch(crate::Command::Tick { now_ms: None }).ok()
                    };
                    if let Some(outcome) = outcome {
                        if outcome.process == ProcessIntent::Exit {
                            tick_stop.store(true, Ordering::Relaxed);
                        }
                        tick_on_outcome(outcome);
                    }
                }
            });
    }
    std::thread::Builder::new()
        .name("host-local-rpc".into())
        .spawn(move || {
            let server_port = listener
                .local_addr()
                .map(|addr| addr.port())
                .unwrap_or(LOCAL_RPC_PORT);
            let _ = listener.set_nonblocking(true);
            while !stop.load(Ordering::Relaxed) && kernel_process_alive(&kernel) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let kernel = Arc::clone(&kernel);
                        let on_outcome = Arc::clone(&on_outcome);
                        let stop = Arc::clone(&stop);
                        let assets = assets.clone();
                        let _ = std::thread::Builder::new()
                            .name("host-local-rpc-conn".into())
                            .spawn(move || {
                                match serve_connection(stream, &kernel, &assets, server_port) {
                                    Ok(Some(outcome)) => {
                                        if outcome.process == ProcessIntent::Exit {
                                            stop.store(true, Ordering::Relaxed);
                                        }
                                        on_outcome(outcome);
                                    }
                                    Ok(None) => {}
                                    Err(err) => eprintln!("local rpc: {err}"),
                                }
                            });
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => continue,
                }
            }
        })
        .expect("host local rpc thread");
}

fn kernel_process_alive(kernel: &Mutex<HostKernel>) -> bool {
    kernel
        .lock()
        .map(|host| host.process_alive())
        .unwrap_or(false)
}

pub fn local_client_origin_allowed(origin: Option<&str>) -> bool {
    origin.is_some_and(|origin| {
        origin_is_desktop(origin) || origin_is_loopback_page(origin, LOCAL_RPC_PORT)
    })
}

fn local_client_origin_allowed_for_port(origin: Option<&str>, port: u16) -> bool {
    origin.is_some_and(|origin| origin_is_desktop(origin) || origin_is_loopback_page(origin, port))
}

fn origin_is_desktop(origin: &str) -> bool {
    matches!(
        origin.trim(),
        "http://localhost:1420"
            | "http://127.0.0.1:1420"
            | "tauri://localhost"
            | "http://tauri.localhost"
            | "https://tauri.localhost"
    )
}

fn origin_is_http(origin: &str) -> bool {
    origin
        .trim()
        .split_once("://")
        .is_some_and(|(scheme, _)| matches!(scheme, "http" | "https"))
}

fn origin_is_loopback_page(origin: &str, port: u16) -> bool {
    let origin = origin.trim();
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if scheme != "http" {
        return false;
    }
    let hostport = rest.split('/').next().unwrap_or(rest);
    let Some((host, raw_port)) = hostport.rsplit_once(':') else {
        return false;
    };
    host == "127.0.0.1" && raw_port.parse::<u16>().ok() == Some(port)
}

fn static_client_request(request: &HttpRequest) -> bool {
    matches!(request.method.as_str(), "GET" | "HEAD")
        && !request.path.starts_with("/rpc")
        && parse_run_route(&request.path).is_none()
}

fn browser_same_origin_request(request: &HttpRequest, server_port: u16) -> bool {
    let same_origin = request
        .header("sec-fetch-site")
        .is_some_and(|value| value.eq_ignore_ascii_case("same-origin"));
    let referer = request.header("referer");
    same_origin && referer.is_some_and(|value| origin_is_loopback_page(value, server_port))
}

fn serve_connection(
    mut stream: TcpStream,
    kernel: &Mutex<HostKernel>,
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
        let mut kernel = kernel
            .lock()
            .map_err(|_| io::Error::other("kernel lock poisoned"))?;
        match kernel.handle(value) {
            Ok(outcome) => {
                let body = serde_json::to_string(&outcome.to_json())?;
                write_json(&mut stream, 200, response_origin, &body)?;
                return Ok(Some(outcome));
            }
            Err(err) => {
                let status = match err {
                    KernelError::Protocol(_) | KernelError::Json(_) => 400,
                    KernelError::Denied(_) => 403,
                    KernelError::Io(_) => 500,
                };
                write_json(
                    &mut stream,
                    status,
                    response_origin,
                    &format!(
                        r#"{{"error":{}}}"#,
                        serde_json::to_string(&err.to_string()).unwrap()
                    ),
                )?;
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

fn serve_run_io(
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

fn parse_run_route(path: &str) -> Option<(String, String)> {
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

fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (name, value) = pair.split_once('=')?;
        if name == key {
            return Some(value.to_string());
        }
    }
    None
}

fn pty_input_bytes(body: &[u8]) -> Vec<u8> {
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

fn serve_loopback_get(
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

fn safe_rel_path(path: &str) -> Option<PathBuf> {
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

fn mime_of(path: &Path) -> &'static str {
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

fn proxy_loopback_get(
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
    let _ = upstream.shutdown(Shutdown::Write);
    io::copy(&mut upstream, stream)?;
    Ok(())
}

fn origin_host(origin: &str) -> &str {
    origin
        .split("://")
        .nth(1)
        .unwrap_or(origin)
        .split('/')
        .next()
        .unwrap_or(origin)
}

fn origin_socket_addr(origin: &str) -> Option<SocketAddr> {
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

struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

fn read_request(stream: &mut TcpStream) -> io::Result<HttpRequest> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated http request",
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "http request too large",
            ));
        }
        if let Some(pos) = find_double_crlf(&buf) {
            break pos;
        }
        if buf.len() > MAX_HEADER_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "http headers too large",
            ));
        }
    };

    let header_text = std::str::from_utf8(&buf[..header_end])
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "http headers are not utf-8"))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "http body too large",
        ));
    }

    let body_start = header_end + 4;
    while buf.len() < body_start + content_length {
        let n = stream.read(&mut chunk)?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "truncated http body",
            ));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    let body = buf[body_start..body_start + content_length].to_vec();
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

const BUILTIN_LOOPBACK_PAGE: &str = r#"<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Agent Taskboard</title>
    <script>window.__HOST_PROTOCOL__ = "";</script>
  </head>
  <body>
    <div id="app"></div>
  </body>
</html>
"#;

fn write_empty(stream: &mut TcpStream, status: u16, origin: Option<&str>) -> io::Result<()> {
    let reason = reason_phrase(status);
    let cors = cors_headers(origin);
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{cors}Connection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream.write_all(response.as_bytes())
}

fn write_bytes(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> io::Result<()> {
    let reason = reason_phrase(status);
    let cors = cors_headers(origin);
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\n{cors}Content-Type: {content_type}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    Ok(())
}

fn write_json(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
    body: &str,
) -> io::Result<()> {
    let reason = reason_phrase(status);
    let cors = cors_headers(origin);
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{cors}Content-Type: application/json; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes())
}

fn authorize(
    request: &HttpRequest,
    kernel: &Mutex<HostKernel>,
    peer: Option<SocketAddr>,
    server_port: u16,
) -> io::Result<bool> {
    let origin = request.header("origin");
    let peer_loopback = peer.map(|addr| addr.ip().is_loopback()).unwrap_or(false);
    let local_origin = local_client_origin_allowed_for_port(origin, server_port);
    if peer_loopback && local_origin {
        return Ok(true);
    }
    // A browser navigation has no Origin header. It may load the local shell,
    // but every RPC and PTY route still requires an exact Client origin or token.
    if peer_loopback
        && origin.is_none()
        && (static_client_request(request) || browser_same_origin_request(request, server_port))
    {
        return Ok(true);
    }
    if request.method == "OPTIONS" && origin.is_some_and(origin_is_http) {
        return Ok(true);
    }
    if is_redeem_rpc(request) {
        return Ok(true);
    }
    if let Some(token) = bearer_token(request) {
        let host = kernel
            .lock()
            .map_err(|_| io::Error::other("kernel lock poisoned"))?;
        return Ok(host.pairing_token_valid(token));
    }
    Ok(false)
}

fn is_redeem_rpc(request: &HttpRequest) -> bool {
    if request.method != "POST" || (request.path != "/rpc" && request.path != "/rpc/") {
        return false;
    }
    serde_json::from_slice::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|value| {
            value
                .get("op")
                .and_then(|op| op.as_str())
                .map(|op| op == "redeemPairing")
        })
        .unwrap_or(false)
}

fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn cors_headers(origin: Option<&str>) -> String {
    match origin {
        Some(origin) if origin_is_http(origin) || origin_is_desktop(origin) => {
            format!(
                "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type, authorization\r\nVary: Origin\r\n"
            )
        }
        _ => String::new(),
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        _ => "OK",
    }
}
