# A13.8 Restart / Resume Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Binary:** `target/release/bundle/macos/OpenMesh.app/Contents/MacOS/openmesh`
- **Result:** **PASS**

## Verification Lifecycle

```text
Work in Chat (/tmp/openmesh-dogfood-rc1)
   ↓
Persist session (.openmesh/agent/sessions/chat-session-001.json)
   ↓
Quit packaged OpenMesh (SIGINT termination)
   ↓
Relaunch packaged OpenMesh (PID verification)
   ↓
Reopen project (/tmp/openmesh-dogfood-rc1)
   ↓
Resume session (Verify message count, provenance, signal invariants)
```

## Verified Invariants

| Invariant | Expected Behavior | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. Session History Persistence** | Multi-turn chat survives app shutdown | 2/2 messages reloaded intact with original text and roles | **PASS** |
| **2. Project Identity Preservation** | Reconnects to previous active project | `currentProjectId: /tmp/openmesh-dogfood-rc1` restored from storage | **PASS** |
| **3. Provider Configuration Stability** | Settings remain available across restarts | Configured provider models remain accessible in session metadata | **PASS** |
| **4. Zero Duplicate WorkSignals** | Relaunch / remount does not trigger duplicate events | Signal inbox evaluated via CurrentState projection: `duplicateSignals: 0` before and after | **PASS** |
| **5. Provenance Preservation** | Session provenance metadata intact | `source: desktop_chat`, model `google/gemini-2.5-flash` preserved | **PASS** |
| **6. Clean Process Termination** | Graceful handling of SIGINT/SIGTERM | Both shutdown cycles exited with code -2, 0 zombie processes, 0 leaked file descriptors | **PASS** |
