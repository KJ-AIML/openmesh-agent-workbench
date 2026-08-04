//! Workspace path confinement shared by read tools and patch apply.

use crate::storage::safe_child_path;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub fn workspace_root(project_path: &str) -> Result<PathBuf, String> {
    fs::canonicalize(project_path).map_err(|e| format!("workspace root unavailable: {e}"))
}

pub fn normalize_rel(relative: &str) -> Result<String, String> {
    let rel = relative.trim().trim_start_matches("./");
    if rel.is_empty() {
        return Err("path is required".into());
    }
    if Path::new(rel).is_absolute() {
        return Err("absolute paths are not allowed".into());
    }
    Ok(rel.to_string())
}

pub fn deny_sensitive_path(path: &Path) -> Result<(), String> {
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let blocked_name = matches!(
        file_name.as_str(),
        ".env"
            | ".env.local"
            | ".env.production"
            | ".env.development"
            | "credentials.json"
            | "secrets.json"
            | "agent-api-key"
            | "id_rsa"
            | "id_ed25519"
            | "id_ecdsa"
            | "id_dsa"
    ) || file_name.starts_with(".env.")
        || file_name.ends_with(".pem")
        || file_name.ends_with(".key")
        || file_name.ends_with(".p12")
        || file_name.ends_with(".pfx");

    if blocked_name {
        return Err(format!("refusing sensitive path: {file_name}"));
    }

    for component in path.components() {
        if let Component::Normal(name) = component {
            if name.to_string_lossy().eq_ignore_ascii_case(".git") {
                return Err("refusing .git paths".into());
            }
        }
    }
    Ok(())
}

/// Resolve an existing file under the workspace root.
pub fn resolve_file_in_workspace(project_path: &str, relative: &str) -> Result<PathBuf, String> {
    let root = workspace_root(project_path)?;
    let rel = normalize_rel(relative)?;
    let joined = safe_child_path(&root, &rel)?;
    let canon = fs::canonicalize(&joined).map_err(|_| format!("not found: {rel}"))?;
    if !canon.starts_with(&root) {
        return Err("path escapes workspace root".into());
    }
    if !canon.is_file() {
        return Err(format!("not a file: {rel}"));
    }
    deny_sensitive_path(&canon)?;
    Ok(canon)
}

/// Resolve a path for write (file may not exist yet); parent must stay in workspace.
pub fn resolve_write_target(project_path: &str, relative: &str) -> Result<(PathBuf, String), String> {
    let root = workspace_root(project_path)?;
    let rel = normalize_rel(relative)?;
    deny_sensitive_path(Path::new(&rel))?;
    let joined = safe_child_path(&root, &rel)?;
    if let Some(parent) = joined.parent() {
        if parent.exists() {
            let parent_canon =
                fs::canonicalize(parent).map_err(|e| format!("parent unavailable: {e}"))?;
            if !parent_canon.starts_with(&root) {
                return Err("path escapes workspace root".into());
            }
        }
    }
    // If the file already exists, canonicalize and re-check.
    if joined.exists() {
        let canon = fs::canonicalize(&joined).map_err(|e| e.to_string())?;
        if !canon.starts_with(&root) {
            return Err("path escapes workspace root".into());
        }
        deny_sensitive_path(&canon)?;
        return Ok((canon, rel));
    }
    deny_sensitive_path(&joined)?;
    Ok((joined, rel))
}

pub fn resolve_dir_in_workspace(project_path: &str, relative: &str) -> Result<PathBuf, String> {
    let root = workspace_root(project_path)?;
    let trimmed = relative.trim().trim_start_matches("./");
    if trimmed.is_empty() || trimmed == "." {
        return Ok(root);
    }
    if Path::new(trimmed).is_absolute() {
        return Err("absolute paths are not allowed".into());
    }
    let joined = safe_child_path(&root, trimmed)?;
    let canon = fs::canonicalize(&joined).map_err(|_| format!("not found: {trimmed}"))?;
    if !canon.starts_with(&root) {
        return Err("path escapes workspace root".into());
    }
    if !canon.is_dir() {
        return Err(format!("not a directory: {trimmed}"));
    }
    deny_sensitive_path(&canon)?;
    Ok(canon)
}

/// SHA-256 hex (no extra crate dependency).
pub fn sha256_hex(data: &[u8]) -> String {
    sha256(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }
    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_git_dir_and_nested_entries() {
        assert!(deny_sensitive_path(Path::new(".git")).is_err());
        assert!(deny_sensitive_path(Path::new(".git/config")).is_err());
        assert!(deny_sensitive_path(Path::new("src/../.git/HEAD")).is_err());
        assert!(deny_sensitive_path(Path::new(".GIT/config")).is_err());
    }

    #[test]
    fn denies_windows_style_git_paths() {
        // Forward-slash form parses `.git` as a Normal component on all platforms.
        assert!(deny_sensitive_path(Path::new(r"C:/proj/.git/config")).is_err());

        // Backslash form: on Windows, `\` is a separator so `.git` is a component.
        // On Unix it is a single Normal name; still assert via PathBuf push.
        let mut win = PathBuf::from("C:");
        win.push("proj");
        win.push(".git");
        win.push("config");
        assert!(deny_sensitive_path(&win).is_err());

        #[cfg(windows)]
        assert!(deny_sensitive_path(Path::new(r"C:\proj\.git\config")).is_err());
    }

    #[test]
    fn allows_normal_source_paths() {
        assert!(deny_sensitive_path(Path::new("README.md")).is_ok());
        assert!(deny_sensitive_path(Path::new("src/main.rs")).is_ok());
        assert!(deny_sensitive_path(Path::new("crates/openmesh-core/src/lib.rs")).is_ok());
    }

    #[test]
    fn denies_sensitive_basenames() {
        assert!(deny_sensitive_path(Path::new(".env")).is_err());
        assert!(deny_sensitive_path(Path::new(".env.local")).is_err());
        assert!(deny_sensitive_path(Path::new("config/.env.staging")).is_err());
        assert!(deny_sensitive_path(Path::new("credentials.json")).is_err());
        assert!(deny_sensitive_path(Path::new("secrets.json")).is_err());
        assert!(deny_sensitive_path(Path::new("agent-api-key")).is_err());
        assert!(deny_sensitive_path(Path::new("id_rsa")).is_err());
        assert!(deny_sensitive_path(Path::new("id_ed25519")).is_err());
        assert!(deny_sensitive_path(Path::new("certs/server.pem")).is_err());
        assert!(deny_sensitive_path(Path::new("keys/app.key")).is_err());
        assert!(deny_sensitive_path(Path::new("store.p12")).is_err());
        assert!(deny_sensitive_path(Path::new("store.pfx")).is_err());
    }

    #[test]
    fn does_not_false_positive_git_prefixed_names() {
        assert!(deny_sensitive_path(Path::new("gitignore")).is_ok());
        assert!(deny_sensitive_path(Path::new("src/git_helpers.rs")).is_ok());
        assert!(deny_sensitive_path(Path::new(".github/workflows/ci.yml")).is_ok());
    }
}
