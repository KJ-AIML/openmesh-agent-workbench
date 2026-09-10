# A13.5 Real Project Dogfood Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Target Fixture:** Disposable project `/tmp/openmesh-dogfood-rc1`
- **Result:** **PASS**

## Flow Execution & Invariant Verification

```text
Open project (/tmp/openmesh-dogfood-rc1)
   ↓
Chat (Agent Engine live session)
   ↓
Ask (Read src/calculator.py, diagnose subtraction bug)
   ↓
Plan (Analyze diff: a - b → a + b)
   ↓
Act / patch proposal (patch-rc1-001 created in proposed state)
   ↓
review diff (Base SHA-256 verified)
   ↓
Apply (Host-gated application with atomic backup)
   ↓
Verify (Automated test executed: 2 + 3 == 5)
   ↓
Continuity evidence (WorkSignal recorded; CurrentState projected)
```

## Verification Details

| Stage | Action / Check | Expected Behavior | Observed Result | Status |
| :--- | :--- | :--- | :--- | :---: |
| **1. Project Confinement** | Initialize marker and inspect workspace boundary | Confined to project root | Marker created at `.openmesh/`; out-of-boundary paths rejected | **PASS** |
| **2. Live Agent Engine** | Ask real upstream model to diagnose bug | Uses tool `read_file`, diagnoses bug | Live model `google/gemini-2.5-flash` called tool and diagnosed subtraction bug | **PASS** |
| **3. Patch Proposal Isolation** | Stage patch proposal `patch-rc1-001` | No auto-apply to disk | File on disk verified to contain `return a - b` prior to host action | **PASS** |
| **4. Host Apply** | Host triggers application of approved patch | Backup created, file updated | Backup saved to `.openmesh/backups/patch-rc1-001/src__calculator.py`; file updated to `return a + b` | **PASS** |
| **5. Verification** | Run Python assertion on updated module | Exit code 0, `2 + 3 == 5` | Verification test executed and passed (`ADD VERIFIED: 2 + 3 == 5`) | **PASS** |
| **6. Continuity Signal** | Record `milestone` WorkSignal into inbox | Signal appended with unique ID | `sig-20260910-18d3e79bb6e8b8d8-12fac` recorded to project Signal Inbox | **PASS** |
| **7. WorkEvent Integrity** | Rebuild current state projection | No direct WorkEvent bypass | Current state rebuilt with `workEvents=0`, zero unvetted event promotion | **PASS** |
| **8. Session Persistence** | Persist session and reload from disk | Session history intact | Session serialized to disk and reloaded with full turn and patch reference | **PASS** |

## Conclusion
The full end-to-end v0.2 workflow operates strictly within architectural boundaries: no silent file modification, explicit host approval gate enforced, and full continuity evidence recorded.
