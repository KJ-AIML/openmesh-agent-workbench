# A13.8 Restart / Resume Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Binary:** `target/release/bundle/macos/OpenMesh.app/Contents/MacOS/openmesh`
- **Result:** **PASS** (Packaged Desktop Application Lifecycle & Persistence Verified)

---

## 1. Initial Pass Qualification

The initial restart/resume check conducted at 15:16 UTC verified process lifecycle and file descriptor management using a script-serialized session file under `/tmp/openmesh-dogfood-rc1`. While this confirmed that the Tauri process handles signals and file locks cleanly, it did not exercise real session persistence written by the packaged WKWebView application runtime. The section below documents the definitive test using the live packaged application.

---

## 2. Packaged Application Verification Lifecycle

```text
Packaged OpenMesh.app runtime active (PID 84151)
   ↓
Agent Chat session executed with live model turn & patch operations
   ↓
Session persisted to .openmesh/agent/chats/sessions.json by native host
   ↓
Global application state recorded to ~/.openmesh/app-state.json
   ↓
Graceful process termination (SIGTERM PID 84151)
   ↓
Verification: process exited cleanly, zero zombie tasks, zero leaked file locks
   ↓
Relaunch packaged OpenMesh.app (PID 87771)
   ↓
Reconnection to active project (/tmp/openmesh-packaged-dogfood) from app-state.json
   ↓
Session reload: full multi-turn dialogue, tool calls, and patch associations restored
   ↓
Signal integrity check: duplicateSignals == 0, pendingSignals == 3, workEvents == 0
   ↓
Clean termination of second instance (PID 87771)
```

---

## 3. Verified Invariants & Audit Results

| Invariant | Expected Behavior | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. UI Chat Session Persistence** | Multi-turn chat generated in WKWebView survives app restart | Session `chat-1789029543798-7957fb3adebec` reloaded intact with system, user, and assistant turns plus tool calls | **PASS** |
| **2. Tool Call & Patch Metadata** | Recorded tool calls (`read_file`, `propose_patch`) retain status | Both tool calls reloaded with output payloads and `patchId` reference | **PASS** |
| **3. Project Identity Preservation** | Reconnects to previous active project | `currentProjectId: /tmp/openmesh-packaged-dogfood` restored from `~/.openmesh/app-state.json` | **PASS** |
| **4. Provider Configuration Stability** | Settings remain available across restarts | Custom Compatible / OpenRouter provider configuration preserved in user settings | **PASS** |
| **5. Zero Duplicate WorkSignals** | Relaunch / remount does not duplicate events | Signal inbox evaluated via `openmesh-cli state`: `duplicateSignals: 0` before and after relaunch | **PASS** |
| **6. Continuity State Stability** | Pending signals remain actionable after restart | 3 pending signals (`wb-patch-proposed-*`, `wb-patch-applied-*`, `wb-verify-run-*`) remain in pending inbox | **PASS** |
| **7. Clean Process Termination** | Graceful handling of termination signals | Both instances (PID 84151, PID 87771) exited cleanly without orphaned threads or WebKit panics | **PASS** |

---

## 4. State Invariant Verification Output

Evaluation via `openmesh-cli state --project /tmp/openmesh-packaged-dogfood --json` immediately following relaunch:

```json
{
  "sourceCounts": {
    "duplicateSignals": 0,
    "gitSignals": 0,
    "heliSignals": 0,
    "otherProducerSignals": 3,
    "pendingSignals": 3,
    "processedSignals": 0,
    "promotionAuditRecords": 0,
    "quarantineSignals": 0,
    "reporterSignals": 0,
    "unknownProducerSignals": 0,
    "workEvents": 0
  }
}
```

---

## 5. Conclusion

The packaged desktop product `OpenMesh.app` demonstrates complete state continuity across full shutdown and restart cycles without data loss, session corruption, or duplicate signal generation.
