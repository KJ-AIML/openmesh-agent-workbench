# A13.5 Real Project Dogfood Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Target Fixture:** Disposable project `/tmp/openmesh-packaged-dogfood`
- **Result:** **PASS** (Full Packaged UI Desktop Workflow & Dual Verify Semantics Verified)

---

## 1. Initial Pass Methodology Qualification (Historical Context)

The initial smoke pass conducted at 15:15 UTC simulated several primary workflow steps using direct CLI commands and script-based file generation:
- Patch proposal JSON was staged directly under `.openmesh/agent/patches/`.
- Host patch application was invoked via manual filesystem copy and run ledger update.
- Continuity evidence was injected using `openmesh-cli signal milestone` instead of exercising the automatic A6 Workbench Continuity Bridge.
- Session persistence was exercised via direct JSON file serialization.

While that initial pass verified file data contracts, schemas, and CLI subcommands, it did **not** exercise the packaged desktop application's WKWebView UI runtime. The section below documents the definitive, unsimulated packaged application validation.

---

## 2. Definitive Packaged Desktop Application Workflow Execution

- **Application Binary:** `target/release/bundle/macos/OpenMesh.app` (macOS arm64, standalone release build)
- **Runtime PIDs:** `84151` (initial run & patch lifecycle) → `87771` (relaunch persistence) → `91795` (Verify milestone closure)
- **Live Upstream Provider:** OpenRouter (`deepseek/deepseek-v4-flash-0731` via Custom Compatible adapter)
- **UI Evidence Artifacts:**
  - `media_1789029585452.png`: Confirms running `OpenMesh.app` window, live chat, applied patch card, and initial verify execution (missing `package.json` producing exit code 254).
  - `media_1789032002520.png`: Confirms running `OpenMesh.app` window after configuring project recipe `package.json`, showing successful UI verify execution with exit code 0 (`ALL TESTS PASSED: 3 * 4 == 12`).

### Verified Packaged Workflow Lifecycle

```text
Launch OpenMesh.app (PID 84151)
   ↓
Select project (/tmp/openmesh-packaged-dogfood, "Test")
   ↓
Open Chat page (Test / Chat)
   ↓
Prompt Agent Engine ("Read src/math_service.py and propose a patch so that multiply(a, b) returns a * b.")
   ↓
Live Model Turn (Calls read_file, proposes patch-18d3e8e751d1d798)
   ↓
Verify disk unmutated before host action (src/math_service.py retains return a + b)
   ↓
Pending Patch card rendered in UI (1 file, diff preview, Approve & apply button)
   ↓
User clicks "Approve & apply" in WKWebView UI
   ↓
Host IPC executes agent_patch_apply (creates atomic backup, updates src/math_service.py)
   ↓
Automatic A6 Signal emitted (wb-patch-applied-patch-18d3e8e751d1d798)
   ↓
UI updates patch card status to "Status: applied · 1 file"
   ↓
[Verify Path 1 — Failure Semantics]:
User clicks "Verify" (npm-typecheck without package.json)
→ Recipe exits 254; UI displays error logs
→ Automatic A6 Blocker signal emitted: wb-verify-run-18d3e8ea439feae0
   ↓
[Project Recipe Configuration]:
Add package.json defining "typecheck": "python3 test_math.py"
   ↓
[Verify Path 2 — Success Semantics]:
User clicks "Verify" in packaged OpenMesh.app UI (PID 91795)
→ Recipe npm run typecheck executes python3 test_math.py
→ Recipe exits 0; UI streams "ALL TESTS PASSED: 3 * 4 == 12"
→ Automatic A6 Milestone signal emitted: wb-verify-run-18d3eb1dae4d3bd8
   ↓
Quit application (SIGTERM PID 84151 / 91795)
   ↓
Relaunch OpenMesh.app → Session, project state, and signals reloaded intact (0 duplicate signals)
```

---

## 3. Observable Checkpoints & Invariant Verification Matrix

| Checkpoint | Expected Behavior | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. Window Rendered** | Standalone Tauri WKWebView renders application chrome | Native macOS window active, project sidebar and tab navigation rendered | **PASS** |
| **2. Project Selected** | Project selection routes to target folder | Selected project "Test" (`/tmp/openmesh-packaged-dogfood`), marker recognized | **PASS** |
| **3. Chat Visible** | Chat interface renders message list and composer | Breadcrumb `Test / Chat`, session controls, prompt composer visible | **PASS** |
| **4. Live Model Response** | Agent Engine calls upstream model via configured credentials | Live `deepseek/deepseek-v4-flash-0731` responded with code explanation across 3 tool rounds | **PASS** |
| **5. Tool Execution** | Agent Engine invokes local tools safely | Executed `read_file` (diagnosed `return a + b`) and `propose_patch` | **PASS** |
| **6. Source Immutability Pre-Approval** | Source files on disk are untouched until explicit host approval | Verified `src/math_service.py` contained buggy code until user approved patch | **PASS** |
| **7. Patch Proposal Card** | Pending patch card displayed with actions | UI rendered `Pending patch patch-18d3e8e751d1d798`, summary, and action buttons | **PASS** |
| **8. Host Apply via UI** | Clicking "Approve & apply" invokes host IPC | Host IPC executed `agent_patch_apply`; UI transitioned button state to applied | **PASS** |
| **9. Atomic Backup Created** | Pre-patch file saved to backup directory | Backup stored at `.openmesh/agent/backups/patch-18d3e8e751d1d798/src__math_service.py` | **PASS** |
| **10. File Mutation on Disk** | Source file updated with patch content | `src/math_service.py` updated on disk to `return a * b` | **PASS** |
| **11. Verify Failure → Blocker** | Unconfigured/failing recipe produces Blocker signal | `npm-typecheck` exited 254; UI logged error; emitted `wb-verify-run-...` with `kind: blocker` | **PASS** |
| **12. Verify Success → Milestone** | Configured/passing recipe produces Milestone signal | `npm-typecheck` exited 0; UI logged `ALL TESTS PASSED`; emitted `wb-verify-run-...` with `kind: milestone` | **PASS** |
| **13. Automatic A6 Signals** | Workbench Continuity Bridge fires without manual CLI injection | 4 signals automatically recorded: proposal, apply, verify blocker, verify milestone | **PASS** |
| **14. Session Persistence** | Multi-turn chat and tool calls saved to disk | Persisted in `.openmesh/agent/chats/sessions.json` (`chat-1789029543798-7957fb3adebec`) | **PASS** |
| **15. Relaunch Integrity** | State survives app termination and restart | App relaunched; `duplicateSignals: 0`, project and chat intact | **PASS** |

---

## 4. Automatic A6 Continuity Bridge Evidence

The following signals were emitted automatically by the native runtime during the packaged UI workflow:

1. **Patch Proposed Signal:**
   - `signalId`: `wb-patch-proposed-patch-18d3e8e751d1d798`
   - `kind`: `review-required`
   - `actor`: `agent-engine` (proxy)
   - `summary`: `Agent proposed patch patch-18d3e8e751d1d798 (1 file): Fix multiply() to return a * b`
   - `evidenceRef`: `.openmesh/agent/patches/patch-18d3e8e751d1d798.json`

2. **Patch Applied Signal:**
   - `signalId`: `wb-patch-applied-patch-18d3e8e751d1d798`
   - `kind`: `progress`
   - `actor`: `openmesh-desktop` (device)
   - `summary`: `Host applied patch patch-18d3e8e751d1d798 (1 file): Fix multiply() to return a * b`
   - `producer-signal`: `wb-patch-proposed-patch-18d3e8e751d1d798`

3. **Verify Completed Signal (Failure Path):**
   - `signalId`: `wb-verify-run-18d3e8ea439feae0`
   - `kind`: `blocker`
   - `actor`: `openmesh-desktop` (device)
   - `summary`: `Verification recipe npm-typecheck failed exit=254 patch=patch-18d3e8e751d1d798.`
   - `producer-signal`: `wb-patch-applied-patch-18d3e8e751d1d798`

4. **Verify Completed Signal (Success Path):**
   - `signalId`: `wb-verify-run-18d3eb1dae4d3bd8`
   - `kind`: `milestone`
   - `actor`: `openmesh-desktop` (device)
   - `summary`: `Verification recipe npm-typecheck succeeded exit=0 patch=patch-18d3e8e751d1d798.`
   - `producer-signal`: `wb-patch-applied-patch-18d3e8e751d1d798`

`openmesh-cli state --project /tmp/openmesh-packaged-dogfood --json` evaluation:
- `pendingSignals`: 4
- `otherProducerSignals`: 4
- `duplicateSignals`: 0
- `quarantineSignals`: 0
- `workEvents`: 0 (ledger remains unbypassed)

---

## 5. Conclusion

The packaged desktop product `OpenMesh.app` has been definitively validated through both operational branches of the primary workbench workflow: native UI launch, real LLM interaction, tool-driven patch proposal, host-gated approval and atomic application, and both failure (`Blocker`) and success (`Milestone`) verification paths with automatic continuity signal generation.
