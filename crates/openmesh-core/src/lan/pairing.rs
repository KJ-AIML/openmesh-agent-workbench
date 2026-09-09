//! LAN pairing, authentication, budget, and audit (v0.2 A4.1).
//!
//! Trust is `pairing → peer identity → capability token → request auth`.
//! Display labels are never used for authorization. Tokens are hashed at rest
//! and stored outside project JSON.

use chrono::Utc;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub const LAN_PAIRING_PROTOCOL: &str = "openmesh-lan-pair/1";
pub const DEFAULT_ASK_BUDGET: u32 = 8;
pub const DEFAULT_ASK_WINDOW_SECS: u64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanCapability {
    LiveAsk,
    Chat,
    Relay,
}

impl LanCapability {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LiveAsk => "live-ask",
            Self::Chat => "chat",
            Self::Relay => "relay",
        }
    }

    pub fn parse_list(raw: &str) -> Result<Vec<Self>, LanAuthError> {
        let mut out = Vec::new();
        for part in raw.split(',') {
            let p = part.trim();
            if p.is_empty() {
                continue;
            }
            out.push(match p {
                "live-ask" | "ask" => Self::LiveAsk,
                "chat" => Self::Chat,
                "relay" => Self::Relay,
                other => {
                    return Err(LanAuthError::InvalidCapability(other.to_string()));
                }
            });
        }
        if out.is_empty() {
            return Err(LanAuthError::InvalidCapability("empty".into()));
        }
        out.sort_by_key(|c| c.as_str());
        out.dedup();
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPeerRecord {
    pub protocol: String,
    pub peer_id: String,
    pub label: String,
    pub capabilities: Vec<LanCapability>,
    pub token_sha256: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
}

impl LanPeerRecord {
    pub fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    pub fn has_capability(&self, cap: LanCapability) -> bool {
        self.capabilities.contains(&cap)
    }

    /// Public listing never includes the token hash (still not a secret, but
    /// no need to leak pairing material into UI/CLI status).
    pub fn redacted(&self) -> LanPeerPublic {
        LanPeerPublic {
            peer_id: self.peer_id.clone(),
            label: self.label.clone(),
            capabilities: self.capabilities.clone(),
            created_at: self.created_at.clone(),
            revoked: self.is_revoked(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPeerPublic {
    pub peer_id: String,
    pub label: String,
    pub capabilities: Vec<LanCapability>,
    pub created_at: String,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanPairIssued {
    pub peer: LanPeerPublic,
    /// Shown once at issuance. Never persisted in plaintext.
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedLanPeer {
    pub peer_id: String,
    pub capabilities: Vec<LanCapability>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LanAuthError {
    #[error("missing lan credential")]
    MissingCredential,
    #[error("malformed lan credential")]
    MalformedCredential,
    #[error("unknown lan peer")]
    UnknownPeer,
    #[error("revoked lan peer")]
    RevokedPeer,
    #[error("lan capability not granted")]
    CapabilityDenied,
    #[error("invalid lan capability: {0}")]
    InvalidCapability(String),
    #[error("lan ask budget exceeded")]
    BudgetExceeded,
    #[error("io: {0}")]
    Io(String),
}

impl LanAuthError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingCredential => "missing_credential",
            Self::MalformedCredential => "malformed_credential",
            Self::UnknownPeer => "unknown_peer",
            Self::RevokedPeer => "revoked_peer",
            Self::CapabilityDenied => "capability_denied",
            Self::InvalidCapability(_) => "invalid_capability",
            Self::BudgetExceeded => "budget_exceeded",
            Self::Io(_) => "io",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            Self::BudgetExceeded => 429,
            Self::CapabilityDenied => 403,
            Self::Io(_) => 500,
            _ => 401,
        }
    }

    pub fn to_json_body(&self) -> String {
        serde_json::json!({
            "error": self.to_string(),
            "code": self.code(),
        })
        .to_string()
    }
}

pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.trim().as_bytes());
    hex_encode(&hasher.finalize())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn ct_eq_hex(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut d = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        d |= x ^ y;
    }
    d == 0
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex_encode(&bytes)
}

fn generate_peer_id() -> String {
    let mut bytes = [0u8; 8];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("lan-peer-{}", hex_encode(&bytes))
}

fn parse_bearer(authorization: Option<&str>) -> Result<String, LanAuthError> {
    let Some(raw) = authorization.map(str::trim).filter(|s| !s.is_empty()) else {
        return Err(LanAuthError::MissingCredential);
    };
    let Some(token) = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(str::trim)
    else {
        return Err(LanAuthError::MalformedCredential);
    };
    if token.is_empty() || token.len() < 16 || token.len() > 256 {
        return Err(LanAuthError::MalformedCredential);
    }
    if !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(LanAuthError::MalformedCredential);
    }
    Ok(token.to_string())
}

#[derive(Clone, Default)]
pub struct MemoryLanRegistry {
    inner: Arc<Mutex<Vec<LanPeerRecord>>>,
}

impl MemoryLanRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn issue(
        &self,
        label: &str,
        capabilities: Vec<LanCapability>,
    ) -> Result<LanPairIssued, LanAuthError> {
        let label = label.trim();
        if label.is_empty() {
            return Err(LanAuthError::Io("peer label is required".into()));
        }
        if capabilities.is_empty() {
            return Err(LanAuthError::InvalidCapability("empty".into()));
        }
        let token = generate_token();
        let record = LanPeerRecord {
            protocol: LAN_PAIRING_PROTOCOL.into(),
            peer_id: generate_peer_id(),
            label: label.to_string(),
            capabilities,
            token_sha256: hash_token(&token),
            created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            revoked_at: None,
        };
        self.inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?
            .push(record.clone());
        Ok(LanPairIssued {
            peer: record.redacted(),
            token,
        })
    }

    pub fn list(&self) -> Result<Vec<LanPeerPublic>, LanAuthError> {
        Ok(self
            .inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?
            .iter()
            .map(LanPeerRecord::redacted)
            .collect())
    }

    pub fn revoke(&self, peer_id: &str) -> Result<LanPeerPublic, LanAuthError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?;
        let rec = guard
            .iter_mut()
            .find(|p| p.peer_id == peer_id)
            .ok_or(LanAuthError::UnknownPeer)?;
        rec.revoked_at = Some(Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true));
        Ok(rec.redacted())
    }

    pub fn authenticate(
        &self,
        authorization: Option<&str>,
        needed: LanCapability,
    ) -> Result<AuthenticatedLanPeer, LanAuthError> {
        let token = parse_bearer(authorization)?;
        let presented = hash_token(&token);
        let guard = self
            .inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?;
        let mut matched: Option<LanPeerRecord> = None;
        for rec in guard.iter() {
            if ct_eq_hex(&rec.token_sha256, &presented) {
                matched = Some(rec.clone());
                break;
            }
        }
        let rec = matched.ok_or(LanAuthError::UnknownPeer)?;
        if rec.is_revoked() {
            return Err(LanAuthError::RevokedPeer);
        }
        if !rec.has_capability(needed) {
            return Err(LanAuthError::CapabilityDenied);
        }
        Ok(AuthenticatedLanPeer {
            peer_id: rec.peer_id,
            capabilities: rec.capabilities,
        })
    }
}

#[derive(Clone)]
pub struct FileLanRegistry {
    path: PathBuf,
}

impl FileLanRegistry {
    pub fn user_default() -> Self {
        let root = std::env::var("OPENMESH_LAN_HOME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                dirs::config_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("openmesh")
                    .join("lan")
            });
        Self {
            path: root.join("peers.json"),
        }
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn load(&self) -> Result<Vec<LanPeerRecord>, LanAuthError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let raw = fs::read_to_string(&self.path).map_err(|e| LanAuthError::Io(e.to_string()))?;
        serde_json::from_str(&raw).map_err(|e| LanAuthError::Io(e.to_string()))
    }

    fn save(&self, records: &[LanPeerRecord]) -> Result<(), LanAuthError> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| LanAuthError::Io(e.to_string()))?;
        }
        let json =
            serde_json::to_string_pretty(records).map_err(|e| LanAuthError::Io(e.to_string()))?;
        let tmp = self.path.with_extension("tmp");
        {
            let mut f = fs::File::create(&tmp).map_err(|e| LanAuthError::Io(e.to_string()))?;
            f.write_all(json.as_bytes())
                .map_err(|e| LanAuthError::Io(e.to_string()))?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600));
            }
        }
        fs::rename(&tmp, &self.path).map_err(|e| LanAuthError::Io(e.to_string()))?;
        Ok(())
    }

    pub fn issue(
        &self,
        label: &str,
        capabilities: Vec<LanCapability>,
    ) -> Result<LanPairIssued, LanAuthError> {
        let mem = MemoryLanRegistry::new();
        {
            let mut g = mem
                .inner
                .lock()
                .map_err(|e| LanAuthError::Io(e.to_string()))?;
            *g = self.load()?;
        }
        let issued = mem.issue(label, capabilities)?;
        let records = mem
            .inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?
            .clone();
        self.save(&records)?;
        Ok(issued)
    }

    pub fn list(&self) -> Result<Vec<LanPeerPublic>, LanAuthError> {
        Ok(self.load()?.iter().map(LanPeerRecord::redacted).collect())
    }

    pub fn revoke(&self, peer_id: &str) -> Result<LanPeerPublic, LanAuthError> {
        let mem = MemoryLanRegistry::new();
        {
            let mut g = mem
                .inner
                .lock()
                .map_err(|e| LanAuthError::Io(e.to_string()))?;
            *g = self.load()?;
        }
        let public = mem.revoke(peer_id)?;
        let records = mem
            .inner
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?
            .clone();
        self.save(&records)?;
        Ok(public)
    }

    pub fn authenticate(
        &self,
        authorization: Option<&str>,
        needed: LanCapability,
    ) -> Result<AuthenticatedLanPeer, LanAuthError> {
        let mem = MemoryLanRegistry::new();
        {
            let mut g = mem
                .inner
                .lock()
                .map_err(|e| LanAuthError::Io(e.to_string()))?;
            *g = self.load()?;
        }
        mem.authenticate(authorization, needed)
    }
}

#[derive(Clone)]
pub enum LanRegistry {
    Memory(MemoryLanRegistry),
    File(FileLanRegistry),
}

impl LanRegistry {
    pub fn memory(store: MemoryLanRegistry) -> Self {
        Self::Memory(store)
    }

    pub fn user_default() -> Self {
        Self::File(FileLanRegistry::user_default())
    }

    pub fn issue(
        &self,
        label: &str,
        capabilities: Vec<LanCapability>,
    ) -> Result<LanPairIssued, LanAuthError> {
        match self {
            Self::Memory(m) => m.issue(label, capabilities),
            Self::File(f) => f.issue(label, capabilities),
        }
    }

    pub fn list(&self) -> Result<Vec<LanPeerPublic>, LanAuthError> {
        match self {
            Self::Memory(m) => m.list(),
            Self::File(f) => f.list(),
        }
    }

    pub fn revoke(&self, peer_id: &str) -> Result<LanPeerPublic, LanAuthError> {
        match self {
            Self::Memory(m) => m.revoke(peer_id),
            Self::File(f) => f.revoke(peer_id),
        }
    }

    pub fn authenticate(
        &self,
        authorization: Option<&str>,
        needed: LanCapability,
    ) -> Result<AuthenticatedLanPeer, LanAuthError> {
        match self {
            Self::Memory(m) => m.authenticate(authorization, needed),
            Self::File(f) => f.authenticate(authorization, needed),
        }
    }
}

#[derive(Debug)]
pub struct LanAskBudget {
    max_asks: u32,
    window_secs: u64,
    hits: Mutex<HashMap<String, Vec<u64>>>,
}

impl LanAskBudget {
    pub fn new(max_asks: u32, window_secs: u64) -> Self {
        Self {
            max_asks,
            window_secs,
            hits: Mutex::new(HashMap::new()),
        }
    }

    pub fn standard() -> Self {
        Self::new(DEFAULT_ASK_BUDGET, DEFAULT_ASK_WINDOW_SECS)
    }

    pub fn allow(&self, peer_id: &str, now_unix: u64) -> Result<(), LanAuthError> {
        let mut g = self
            .hits
            .lock()
            .map_err(|e| LanAuthError::Io(e.to_string()))?;
        let window_start = now_unix.saturating_sub(self.window_secs);
        let entry = g.entry(peer_id.to_string()).or_default();
        entry.retain(|t| *t >= window_start);
        if entry.len() as u32 >= self.max_asks {
            return Err(LanAuthError::BudgetExceeded);
        }
        entry.push(now_unix);
        Ok(())
    }
}

impl Default for LanAskBudget {
    fn default() -> Self {
        Self::standard()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanAuditEvent {
    pub at: String,
    pub peer_id: Option<String>,
    pub operation: String,
    pub allowed: bool,
    pub code: String,
}

#[derive(Clone, Default)]
pub struct LanAuditLog {
    events: Arc<Mutex<Vec<LanAuditEvent>>>,
    path: Option<PathBuf>,
}

impl LanAuditLog {
    pub fn memory() -> Self {
        Self::default()
    }

    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
            path: Some(path.into()),
        }
    }

    pub fn user_default() -> Self {
        let path = FileLanRegistry::user_default()
            .path
            .parent()
            .unwrap_or(Path::new("."))
            .join("audit.jsonl");
        Self::file(path)
    }

    pub fn record(&self, peer_id: Option<&str>, operation: &str, allowed: bool, code: &str) {
        let event = LanAuditEvent {
            at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            peer_id: peer_id.map(|s| s.to_string()),
            operation: operation.to_string(),
            allowed,
            code: code.to_string(),
        };
        if let Ok(mut g) = self.events.lock() {
            g.push(event.clone());
        }
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(path) {
                if let Ok(line) = serde_json::to_string(&event) {
                    let _ = writeln!(f, "{line}");
                }
            }
        }
    }

    pub fn events(&self) -> Vec<LanAuditEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_authenticate_revoke() {
        let reg = MemoryLanRegistry::new();
        let issued = reg.issue("alice", vec![LanCapability::LiveAsk]).unwrap();
        assert!(!issued.token.is_empty());
        let auth = reg
            .authenticate(
                Some(&format!("Bearer {}", issued.token)),
                LanCapability::LiveAsk,
            )
            .unwrap();
        assert_eq!(auth.peer_id, issued.peer.peer_id);
        assert!(reg
            .authenticate(
                Some(&format!("Bearer {}", issued.token)),
                LanCapability::Chat
            )
            .is_err());
        reg.revoke(&issued.peer.peer_id).unwrap();
        let err = reg
            .authenticate(
                Some(&format!("Bearer {}", issued.token)),
                LanCapability::LiveAsk,
            )
            .unwrap_err();
        assert_eq!(err, LanAuthError::RevokedPeer);
    }

    #[test]
    fn unknown_and_missing_tokens_fail_closed() {
        let reg = MemoryLanRegistry::new();
        assert_eq!(
            reg.authenticate(None, LanCapability::LiveAsk).unwrap_err(),
            LanAuthError::MissingCredential
        );
        assert_eq!(
            reg.authenticate(Some("Bearer not-hex"), LanCapability::LiveAsk)
                .unwrap_err(),
            LanAuthError::MalformedCredential
        );
        let err = reg
            .authenticate(
                Some("Bearer 0123456789abcdef0123456789abcdef"),
                LanCapability::LiveAsk,
            )
            .unwrap_err();
        assert_eq!(err, LanAuthError::UnknownPeer);
    }

    #[test]
    fn label_is_not_an_auth_factor() {
        let reg = MemoryLanRegistry::new();
        let issued = reg.issue("alice", vec![LanCapability::LiveAsk]).unwrap();
        assert!(reg
            .authenticate(Some("Bearer alice"), LanCapability::LiveAsk)
            .is_err());
        assert!(reg
            .authenticate(
                Some(&format!("Bearer {}", issued.token)),
                LanCapability::LiveAsk
            )
            .is_ok());
    }

    #[test]
    fn budget_rejects_after_limit() {
        let budget = LanAskBudget::new(2, 60);
        budget.allow("p1", 1000).unwrap();
        budget.allow("p1", 1001).unwrap();
        assert_eq!(
            budget.allow("p1", 1002).unwrap_err(),
            LanAuthError::BudgetExceeded
        );
        budget.allow("p2", 1002).unwrap();
    }

    #[test]
    fn file_registry_does_not_store_plaintext_token() {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-lan-pair-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("peers.json");
        let store = FileLanRegistry::new(&path);
        let issued = store
            .issue("bob", vec![LanCapability::Chat, LanCapability::Relay])
            .unwrap();
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains(&issued.token));
        assert!(raw.contains(&issued.peer.peer_id));
        let _ = fs::remove_dir_all(&dir);
    }
}
