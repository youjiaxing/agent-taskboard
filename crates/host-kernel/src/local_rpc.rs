use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{CommandOutcome, HostKernel, KernelError};

pub const LOCAL_RPC_PORT: u16 = 10529;

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn bind_local_rpc(preferred_port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, preferred_port))
        .or_else(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)))?;
    let port = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{port}")))
}

pub fn spawn_local_rpc(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
) {
    let on_outcome = Arc::new(on_outcome);
    std::thread::Builder::new()
        .name("host-local-rpc".into())
        .spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else {
                    continue;
                };
                let kernel = Arc::clone(&kernel);
                let on_outcome = Arc::clone(&on_outcome);
                let _ = std::thread::Builder::new()
                    .name("host-local-rpc-conn".into())
                    .spawn(move || match serve_connection(stream, &kernel) {
                        Ok(Some(outcome)) => on_outcome(outcome),
                        Ok(None) => {}
                        Err(err) => eprintln!("local rpc: {err}"),
                    });
            }
        })
        .expect("host local rpc thread");
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
) -> io::Result<Option<CommandOutcome>> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    let request = read_request(&mut stream)?;
    let origin = request.header("origin");
    let allowed_origin = local_client_origin_allowed(origin);
    if !allowed_origin || !host_header_is_loopback(request.header("host")) {
        write_json(
            &mut stream,
            403,
            origin.filter(|_| allowed_origin),
            r#"{"error":"origin not allowed"}"#,
        )?;
        return Ok(None);
    }

    if request.method == "OPTIONS" {
        write_empty(&mut stream, 204, origin)?;
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

fn write_empty(stream: &mut TcpStream, status: u16, origin: Option<&str>) -> io::Result<()> {
    let reason = reason_phrase(status);
    let cors = cors_headers(origin);
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{cors}Connection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream.write_all(response.as_bytes())
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
            "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Methods: POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type\r\nVary: Origin\r\n"
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
        _ => "OK",
    }
}
