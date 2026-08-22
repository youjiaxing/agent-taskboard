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
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, preferred_port))
        .or_else(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)))?;
    let port = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{port}")))
}

fn bind_strict(port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    let bound = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{bound}")))
}

pub struct LoopbackServer {
    protocol_url: String,
    stop: Arc<AtomicBool>,
}

impl LoopbackServer {
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
                spawn_local_rpc_inner(listener, kernel, assets, Arc::clone(&stop), on_outcome);
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
                spawn_local_rpc_inner(listener, kernel, assets, Arc::clone(&stop), on_outcome);
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
    );
}

fn spawn_local_rpc_inner(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    assets: LoopbackAssets,
    stop: Arc<AtomicBool>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
) {
    let on_outcome = Arc::new(on_outcome);
    std::thread::Builder::new()
        .name("host-local-rpc".into())
        .spawn(move || {
            let _ = listener.set_nonblocking(true);
            while !stop.load(Ordering::Relaxed) && kernel_running(&kernel) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let kernel = Arc::clone(&kernel);
                        let on_outcome = Arc::clone(&on_outcome);
                        let stop = Arc::clone(&stop);
                        let assets = assets.clone();
                        let _ = std::thread::Builder::new()
                            .name("host-local-rpc-conn".into())
                            .spawn(move || match serve_connection(stream, &kernel, &assets) {
                                Ok(Some(outcome)) => {
                                    if outcome.process == ProcessIntent::Exit {
                                        stop.store(true, Ordering::Relaxed);
                                    }
                                    on_outcome(outcome);
                                }
                                Ok(None) => {}
                                Err(err) => eprintln!("local rpc: {err}"),
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

fn kernel_running(kernel: &Mutex<HostKernel>) -> bool {
    kernel
        .lock()
        .map(|host| host.snapshot().running)
        .unwrap_or(false)
}

pub fn local_client_origin_allowed(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(origin) => origin_is_desktop_or_loopback(origin),
    }
}

fn origin_is_desktop_or_loopback(origin: &str) -> bool {
    let origin = origin.trim();
    if origin.eq_ignore_ascii_case("null") || origin.eq_ignore_ascii_case("tauri://localhost") {
        return true;
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return false;
    };
    if !matches!(scheme, "http" | "https" | "tauri") {
        return false;
    }
    let hostport = rest.split('/').next().unwrap_or(rest);
    matches!(
        hostname_of(hostport),
        "127.0.0.1" | "localhost" | "::1" | "tauri.localhost"
    )
}

fn hostname_of(hostport: &str) -> &str {
    if let Some(rest) = hostport.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    match hostport.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => hostport,
    }
}

fn host_header_is_loopback(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    matches!(hostname_of(host.trim()), "127.0.0.1" | "localhost" | "::1")
}

fn serve_connection(
    mut stream: TcpStream,
    kernel: &Mutex<HostKernel>,
    assets: &LoopbackAssets,
) -> io::Result<Option<CommandOutcome>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = read_request(&mut stream)?;
    let origin = request.header("origin");
    let allowed_origin = local_client_origin_allowed(origin);
    if !allowed_origin || !host_header_is_loopback(request.header("host")) {
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
        write_json(
            &mut stream,
            403,
            origin.filter(|_| allowed_origin),
            &body.to_string(),
        )?;
        return Ok(None);
    }

    if request.method == "OPTIONS" {
        write_empty(&mut stream, 204, origin)?;
        return Ok(None);
    }

    if request.method == "GET" || request.method == "HEAD" {
        serve_loopback_get(&mut stream, &request, origin, assets)?;
        return Ok(None);
    }

    if request.method == "POST" && (request.path == "/rpc" || request.path == "/rpc/") {
        let value: serde_json::Value = match serde_json::from_slice(&request.body) {
            Ok(value) => value,
            Err(err) => {
                write_json(
                    &mut stream,
                    400,
                    origin,
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
                write_json(&mut stream, 200, origin, &body)?;
                return Ok(Some(outcome));
            }
            Err(err) => {
                let status = match err {
                    KernelError::Protocol(_) | KernelError::Json(_) => 400,
                    KernelError::Io(_) => 500,
                };
                write_json(
                    &mut stream,
                    status,
                    origin,
                    &format!(
                        r#"{{"error":{}}}"#,
                        serde_json::to_string(&err.to_string()).unwrap()
                    ),
                )?;
                return Ok(None);
            }
        }
    }

    write_json(&mut stream, 404, origin, r#"{"error":"not found"}"#)?;
    Ok(None)
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

fn cors_headers(origin: Option<&str>) -> String {
    match origin {
        Some(origin) if local_client_origin_allowed(Some(origin)) => format!(
            "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type\r\nVary: Origin\r\n"
        ),
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
