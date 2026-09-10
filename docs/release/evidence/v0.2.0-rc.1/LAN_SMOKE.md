# A13.10 LAN Pair / Ask / Revoke Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Environment:** Single physical host (Darwin arm64); Loopback/local interface networking
- **Result:** **PASS (SINGLE HOST LOOPBACK)**

## Scope & Limitation Disclosure
> **Notice:** Testing was conducted on a single physical developer workstation using local/loopback interfaces (`127.0.0.1` and OS subnet sockets). Multi-device physical hardware mesh routing was NOT simulated across separate physical machines.

## Verified Invariants & Lifecycle

```text
default launch → no LAN exposure (wildcard bind denied)
       ↓
explicit enable LAN → listener exposed on loopback
       ↓
pair peer → authenticated Ask works
       ↓
attempt mutation → denied (LanPeer policy restricts tools to read-only ask tools)
       ↓
unauthenticated request → rejected before engine (zero LLM token consumption)
       ↓
revoke peer → subsequent requests denied (HTTP 401/403)
```

## Evidence Matrix

| Gate / Security Invariant | Test Target / Command | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. Default No Exposure** | `lan_trust::default_serve_does_not_bind_wildcard` | Listener binds loopback only; fails if unauthenticated wildcard attempted | **PASS** |
| **2. Explicit Exposure Flag** | `lan_trust::lan_exposure_requires_explicit_flag` | LAN networking is disabled until explicitly enabled by host | **PASS** |
| **3. Unauthenticated Rejection** | `lan_trust::unauthenticated_live_ask_is_rejected_before_engine` | Request denied before dispatching to Agent Engine; 0 model tokens consumed | **PASS** |
| **4. Token Validation** | `lan_trust::invalid_and_unknown_tokens_are_rejected` | Invalid or unrecognized bearer credentials rejected | **PASS** |
| **5. Mutation Denial** | `lan_trust::valid_peer_reaches_lan_peer_policy_without_act_tools` | `LanPeer` origin policy explicitly forbids mutating tools (`propose_patch`, `update_task`, `link_session`) | **PASS** |
| **6. Revocation** | `lan_trust::revoked_peer_is_rejected` | Revoking paired peer identity immediately denies subsequent requests | **PASS** |
| **7. End-to-End Workflow** | `openmesh-cli::lan_workflow::lan_serve_send_ask_status_loopback` | Live peer serve, send, ask, status cycle passed end-to-end on loopback | **PASS** |
