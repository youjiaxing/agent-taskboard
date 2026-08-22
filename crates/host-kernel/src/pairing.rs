use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{KernelError, LOCAL_RPC_PORT};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingOffer {
    pub address: String,
    pub code: String,
    pub text: String,
    pub qr_text: String,
    pub qr_svg: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairedClient {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IssuedPairing {
    pub token: String,
    pub host_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IssuedClient {
    pub id: String,
    pub name: String,
    pub token: String,
}

impl IssuedClient {
    pub(crate) fn summary(&self) -> PairedClient {
        PairedClient {
            id: self.id.clone(),
            name: self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveOffer {
    pub address: String,
    pub code: String,
}

const CODE_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

impl ActiveOffer {
    pub(crate) fn new(address: String) -> Self {
        Self {
            address,
            code: generate_code(),
        }
    }

    pub(crate) fn to_offer(&self) -> PairingOffer {
        let text = format!("{}\n{}", self.address, self.code);
        PairingOffer {
            address: self.address.clone(),
            code: self.code.clone(),
            text: text.clone(),
            qr_text: text.clone(),
            qr_svg: qr_svg(&text),
        }
    }
}

fn generate_code() -> String {
    let mut bytes = [0u8; 8];
    fill_random(&mut bytes);
    let chars: String = bytes
        .iter()
        .map(|byte| CODE_ALPHABET[(*byte as usize) % CODE_ALPHABET.len()] as char)
        .collect();
    format!("{}-{}", &chars[..4], &chars[4..])
}

pub(crate) fn generate_token() -> String {
    random_hex(32)
}

pub(crate) fn random_id() -> String {
    random_hex(16)
}

pub(crate) fn codes_match(expected: &str, provided: &str) -> bool {
    normalize_code(expected) == normalize_code(provided)
}

fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}

fn random_hex(n: usize) -> String {
    let mut bytes = vec![0u8; n];
    fill_random(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fill_random(buf: &mut [u8]) {
    getrandom::getrandom(buf).expect("rng");
}

fn qr_svg(text: &str) -> String {
    let Ok(code) = qrcode::QrCode::new(text.as_bytes()) else {
        return String::new();
    };
    code.render::<qrcode::render::svg::Color>()
        .min_dimensions(160, 160)
        .build()
}

pub(crate) fn parse_http_url(address: &str) -> Result<String, String> {
    let address = address.trim();
    if address.is_empty() {
        return Err("missing address".into());
    }
    let (scheme, rest) = address
        .split_once("://")
        .ok_or_else(|| "address must be an http URL".to_string())?;
    if !matches!(scheme, "http" | "https") {
        return Err("address must be an http URL".into());
    }
    if rest.is_empty() || rest.starts_with('/') {
        return Err("address must be an http URL".into());
    }
    Ok(address.trim_end_matches('/').to_string())
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteHost {
    pub id: String,
    pub display_name: String,
    pub address: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SavedRemoteHost {
    pub id: String,
    pub display_name: String,
    pub address: String,
}

impl RemoteHost {
    pub(crate) fn to_saved(&self) -> SavedRemoteHost {
        SavedRemoteHost {
            id: self.id.clone(),
            display_name: self.display_name.clone(),
            address: self.address.clone(),
        }
    }
}

pub(crate) fn post_rpc(
    address: &str,
    token: Option<&str>,
    body: &serde_json::Value,
) -> Result<serde_json::Value, KernelError> {
    let address = parse_http_url(address).map_err(KernelError::Protocol)?;
    let (socket, host_header) = rpc_target(&address)?;
    let payload = body.to_string();
    let auth = token
        .map(|token| format!("Authorization: Bearer {token}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /rpc HTTP/1.1\r\nHost: {host_header}\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
        payload.len()
    );
    let mut stream = TcpStream::connect_timeout(&socket, Duration::from_secs(5))?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(request.as_bytes())?;
    let _ = stream.shutdown(Shutdown::Write);
    let mut buf = String::new();
    stream.read_to_string(&mut buf)?;
    let (head, body) = buf.split_once("\r\n\r\n").unwrap_or((buf.as_str(), ""));
    let status = head
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    if status == 403 {
        return Err(KernelError::Denied("invalid pairing code".into()));
    }
    if status != 200 {
        return Err(KernelError::Protocol(format!("pairing failed ({status})")));
    }
    Ok(serde_json::from_str(body)?)
}

fn rpc_target(address: &str) -> Result<(std::net::SocketAddr, String), KernelError> {
    let rest = address
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(address);
    let hostport = rest.split('/').next().unwrap_or(rest);
    let host_header = if hostport.contains(':') {
        hostport.to_string()
    } else {
        format!("{hostport}:{LOCAL_RPC_PORT}")
    };
    let addr = host_header
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| KernelError::Protocol("address is not reachable".into()))?;
    Ok((addr, host_header))
}
