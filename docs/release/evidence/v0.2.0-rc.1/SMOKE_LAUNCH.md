# A13.4 Packaged Desktop Application Launch Smoke Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Binary Path:** `target/release/bundle/macos/OpenMesh.app/Contents/MacOS/openmesh`
- **Mode:** Packaged production standalone application (no `tauri dev` dev-server)
- **Result:** **PASS**

## Verified Behaviors

| Check | Requirement | Result | Observation |
| :--- | :--- | :---: | :--- |
| **1. Application Launch** | Packaged binary boots and stays resident | **PASS** | Process spawned with PID 77336; poll remained alive after initialization window |
| **2. Bundle Identity** | Reports version `0.2.0-rc.1` and bundle ID | **PASS** | `CFBundleShortVersionString: 0.2.0-rc.1`, `CFBundleIdentifier: com.openmesh.app` |
| **3. Asset & UI Runtime** | Embedded SPA assets load without dev server | **PASS** | Embedded `index.html` and Vue production bundles executed via WKWebView |
| **4. Storage Initialization** | Creates or connects to global storage | **PASS** | `~/.openmesh/app-state.json` initialized and updated with valid active project state |
| **5. Clean Termination** | Quits gracefully without unhandled panic | **PASS** | Process terminated cleanly on SIGINT with exit code -2 and zero stderr errors |
| **6. Relaunch & Reopen** | Survives restart and re-attaches cleanly | **PASS** | Second launch (PID 77349) initialized immediately, preserved state, quit cleanly |

## Log Hygiene
- Zero fatal panics.
- Zero uncaught WebKit exceptions.
