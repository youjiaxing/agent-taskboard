use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Mutex;

use crate::HostKernel;

use super::*;

pub(super) fn kernel_process_alive(kernel: &Mutex<HostKernel>) -> bool {
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

pub(super) fn local_client_origin_allowed_for_port(origin: Option<&str>, port: u16) -> bool {
    origin.is_some_and(|origin| origin_is_desktop(origin) || origin_is_loopback_page(origin, port))
}

pub(super) fn origin_is_desktop(origin: &str) -> bool {
    matches!(
        origin.trim(),
        "http://localhost:1420"
            | "http://127.0.0.1:1420"
            | "tauri://localhost"
            | "http://tauri.localhost"
            | "https://tauri.localhost"
    )
}

pub(super) fn origin_is_http(origin: &str) -> bool {
    origin
        .trim()
        .split_once("://")
        .is_some_and(|(scheme, _)| matches!(scheme, "http" | "https"))
}

pub(super) fn origin_is_loopback_page(origin: &str, port: u16) -> bool {
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

pub(super) fn static_client_request(request: &HttpRequest) -> bool {
    matches!(request.method.as_str(), "GET" | "HEAD")
        && !request.path.starts_with("/rpc")
        && parse_run_route(&request.path).is_none()
}

pub(super) fn browser_same_origin_request(request: &HttpRequest, server_port: u16) -> bool {
    let same_origin = request
        .header("sec-fetch-site")
        .is_some_and(|value| value.eq_ignore_ascii_case("same-origin"));
    let referer = request.header("referer");
    same_origin && referer.is_some_and(|value| origin_is_loopback_page(value, server_port))
}

pub(super) struct HttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: HashMap<String, String>,
    pub(super) body: Vec<u8>,
}

impl HttpRequest {
    pub(super) fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

pub(super) fn read_request(stream: &mut TcpStream) -> io::Result<HttpRequest> {
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

pub(super) fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|window| window == b"\r\n\r\n")
}

pub(super) const BUILTIN_LOOPBACK_PAGE: &str = r#"<!doctype html>
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

pub(super) fn write_empty(
    stream: &mut TcpStream,
    status: u16,
    origin: Option<&str>,
) -> io::Result<()> {
    let reason = reason_phrase(status);
    let cors = cors_headers(origin);
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\n{cors}Connection: close\r\nContent-Length: 0\r\n\r\n"
    );
    stream.write_all(response.as_bytes())
}

pub(super) fn request_backgrounds_refreshes(request: &serde_json::Value) -> bool {
    // Client ticks share the frontend request queue. A slow Tracker read must
    // not hold later navigation or form actions behind the background refresh.
    matches!(
        request.get("op").and_then(|value| value.as_str()),
        Some("focusProject" | "setClientView" | "tick")
    )
}

pub(super) fn request_defers_refreshes(request: &serde_json::Value) -> bool {
    matches!(
        request.get("op").and_then(|value| value.as_str()),
        Some("refresh" | "tick" | "setClientView" | "showWindow" | "focusProject" | "noteRunEnded")
    )
}

pub(super) fn request_defers_issue_document(request: &serde_json::Value) -> bool {
    request.get("op").and_then(|value| value.as_str()) == Some("loadIssueDocument")
}

pub(super) fn request_defers_issue_write(request: &serde_json::Value) -> bool {
    matches!(
        request.get("op").and_then(|value| value.as_str()),
        Some(
            "createIssue"
                | "updateIssue"
                | "setIssueOpen"
                | "addIssueComment"
                | "claimIssue"
                | "releaseIssue"
                | "setIssueParent"
                | "setIssueBlockedBy"
        )
    )
}

pub(super) fn write_bytes(
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

pub(super) fn write_json(
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

pub(super) fn authorize(
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

pub(super) fn is_redeem_rpc(request: &HttpRequest) -> bool {
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

pub(super) fn bearer_token(request: &HttpRequest) -> Option<&str> {
    request
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

pub(super) fn cors_headers(origin: Option<&str>) -> String {
    match origin {
        Some(origin) if origin_is_http(origin) || origin_is_desktop(origin) => {
            format!(
                "Access-Control-Allow-Origin: {origin}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: content-type, authorization\r\nVary: Origin\r\n"
            )
        }
        _ => String::new(),
    }
}

pub(super) fn reason_phrase(status: u16) -> &'static str {
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
