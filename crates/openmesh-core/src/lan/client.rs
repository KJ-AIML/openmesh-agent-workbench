//! HTTP client for LAN peer send / ask / health.

use crate::lan::contract::{LanAskHttpBody, LanHealthResponse};
use crate::mesh::query::MeshRemoteQueryAnswer;
use crate::relay::contract::RelayPackage;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LanClientError {
    #[error("invalid address: {0}")]
    Address(String),
    #[error("http: {0}")]
    Http(String),
    #[error("peer error ({status}): {body}")]
    Peer { status: u16, body: String },
    #[error("decode: {0}")]
    Decode(String),
}

pub fn parse_host_port(to: &str) -> Result<(String, u16), LanClientError> {
    let t = to.trim();
    if let Some((host, port_s)) = t.rsplit_once(':') {
        let host = host.trim().trim_start_matches('[').trim_end_matches(']');
        if host.is_empty() {
            return Err(LanClientError::Address(to.into()));
        }
        let port: u16 = port_s
            .trim()
            .parse()
            .map_err(|_| LanClientError::Address(to.into()))?;
        if port == 0 {
            return Err(LanClientError::Address(to.into()));
        }
        Ok((host.to_string(), port))
    } else {
        Err(LanClientError::Address(format!(
            "expected host:port, got {to}"
        )))
    }
}

fn blocking_client() -> Result<reqwest::blocking::Client, LanClientError> {
    // Live Agent Engine asks can exceed a short HTTP timeout.
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| LanClientError::Http(e.to_string()))
}

pub fn health_check(host: &str, port: u16) -> Result<LanHealthResponse, LanClientError> {
    let url = format!("http://{host}:{port}/v1/health");
    let client = blocking_client()?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

pub fn send_package_to_peer(
    host: &str,
    port: u16,
    package: &RelayPackage,
) -> Result<serde_json::Value, LanClientError> {
    let url = format!("http://{host}:{port}/v1/relay/package");
    let client = blocking_client()?;
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(package)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

pub fn ask_peer(
    host: &str,
    port: u16,
    question: &str,
    tier: Option<&str>,
) -> Result<MeshRemoteQueryAnswer, LanClientError> {
    let url = format!("http://{host}:{port}/v1/mesh/ask");
    let body = LanAskHttpBody {
        question: question.to_string(),
        tier: tier.map(|s| s.to_string()),
    };
    let client = blocking_client()?;
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_host_port_ok() {
        let (h, p) = parse_host_port("127.0.0.1:41778").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 41778);
    }

    #[test]
    fn parse_host_port_rejects_missing_port() {
        assert!(parse_host_port("127.0.0.1").is_err());
    }
}
