# A13.11 Security Smoke Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Result:** **PASS**

## Security Boundary Verification Matrix

| Security Boundary | Mechanism / Guardrail | Verification Evidence | Status |
| :--- | :--- | :--- | :---: |
| **1. Unregistered Project Path** | Mutations require valid registered workspace directory | `workspace_root` validates canonical project path | **PASS** |
| **2. Path Traversal & Sensitive Files** | `deny_sensitive_path` blocks `.env*`, secrets, SSH keys, `.git` | 5/5 path safety tests passed; absolute & `..` paths rejected | **PASS** |
| **3. Invalid LAN Bearer** | LAN HTTP endpoints require authenticated peer token | `lan_trust::invalid_and_unknown_tokens_are_rejected` passed | **PASS** |
| **4. Revoked Peer Bearer** | Immediate denial upon revocation | `lan_trust::revoked_peer_is_rejected` passed | **PASS** |
| **5. Credential Sanitization in Logs** | Redacts bearer tokens and API keys | Verified in A13.6 and A13.7: 0 secret keys leaked in error strings or stdout | **PASS** |
| **6. Packaged App CSP** | Strict Content Security Policy in `index.html` & `tauri.conf.json` | `npm run check:tauri-security` verified CSP set, `withGlobalTauri: false`, `plugin-fs` absent | **PASS** |
| **7. PTY CWD Confinement** | PTY shell spawns only in validated existing directory | `pty_desktop::resolve_cwd` rejects nonexistent or non-directory paths | **PASS** |
| **8. Host-Gated Patch Apply** | Agent Engine cannot auto-apply source patches | No apply tool exposed to model; only human-invoked host IPC can trigger apply | **PASS** |
