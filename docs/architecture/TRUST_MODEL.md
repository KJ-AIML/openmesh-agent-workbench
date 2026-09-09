# OpenMesh v0.2 trust model

**Status:** implemented as of A4 on `feat/v0.2.0-unified-workbench`
**Not:** a 1.0 security claim, cloud identity, or E2E mesh product

This document describes **code that exists**, not the desired end state.
Agent turns go through `authorize_agent_turn` (`crates/openmesh-core/src/agent_engine/policy.rs`).
LAN callers become `LanPeer` only after pairing + bearer authentication (A4.1).
The webview is a privilege boundary (A4.2). Process spawn and mutating IPC
use `program + argv + cwd` and a registered-project path pipeline (A4.3).

## Origins

| Origin | Who | How identity is established |
|--------|-----|-----------------------------|
| `LocalChat` | Desktop Chat (Ask / Plan / Act) | Local Tauri webview; project path must be a registered workspace for engine turns |
| `LocalDelegate` | Desktop Chat delegate mode | Same local webview; policy uses Ask tools |
| `LocalCLI` | `openmesh-cli agent` | Local process; Ask tools |
| `ContinuityQuery` | Continuity live-ask on this host | Local/system; Ask tools; no pairing |
| `LanPeer` | Remote LAN HTTP caller | Explicit `--expose-lan` / `exposeLan`, then pairing token (Bearer) with capability `live-ask` |

Successful LAN authentication still cannot mint LocalChat/Act. Remote origins stay Ask.

## Trust matrix (implemented)

| Origin | Identity | Read | Model | Shell | Patch proposal | Patch apply | Secret access |
|--------|----------|------|-------|-------|----------------|-------------|---------------|
| LocalChat Ask | local webview | workspace-bounded Ask tools | yes | desktop PTY/terminal for a registered project cwd; not an engine tool | no | explicit host IPC `agent_patch_apply` | `LocalConfigured` — engine may use the configured key; tools deny sensitive paths |
| LocalChat Act | local webview | workspace-bounded Act tools | yes | same as Ask | yes (`propose_patch`) | explicit host IPC; model cannot apply | `LocalConfigured` |
| LocalChat Plan | local webview | workspace-bounded Plan tools | yes | same as Ask | yes (`propose_patch`) | explicit host IPC | `LocalConfigured` |
| LocalDelegate | local webview | Ask tools (same allowlist as Ask) | yes | same as Ask | no | explicit host IPC | `LocalConfigured` |
| LocalCLI | local CLI process | Ask tools | yes | CLI/host process, not engine | no | not via engine; host/CLI only | `LocalConfigured` |
| ContinuityQuery | local/system | Ask tools, read-only | yes | no | no | no | `RemoteNoExport` — key used to call the model, not exported |
| LanPeer | authenticated paired peer | Ask tools, restricted | yes, budgeted (8 asks / 60s per peer) | no | no | no | `RemoteNoExport` |

Notes that match code, not the A4 brief's aspirational cells:

- `LocalDelegate` does **not** currently get Act/`propose_patch`. `delegate_tool_names()` is Ask.
- `LocalCLI` is Ask tools, not a separate CLI act policy.
- Patch **apply** is never an engine tool. It is `agent_patch_apply` on the desktop, gated on a registered project path.
- Shell in the matrix means the desktop PTY / visible-terminal launchers (`pty_create`, `open_terminal`, `open_agent_cli`, `run_command_preset`). Agent Engine has no shell tool.

## LAN trust boundary (A4.1)

Replaced `network reachability == trust` with:

`explicit LAN enablement + authenticated peer identity + bounded peer authority`

- Default HTTP bind: `127.0.0.1`. Wildcard bind requires `--expose-lan` / `exposeLan`.
- Ordinary Chat/Continuity start does not advertise LAN.
- Protected routes (`/v1/mesh/ask`, `/v1/chat/message`, `/v1/relay/package`) require Bearer + capability **before** Agent Engine. `/v1/health` stays open.
- Pairing tokens are hashed at rest under the user LAN home (`OPENMESH_LAN_HOME` or `openmesh/lan/peers.json`, mode `0600`). Not project JSON. Display labels are not authorization.
- Missing / malformed / unknown / revoked / wrong-capability credentials fail closed (401/403). Auth failure does not call the provider.
- Live-ask budget: 8 requests / 60 seconds per authenticated peer (in memory).
- Audit records peer id, time, operation class, allow/deny. Credentials and provider secrets are not logged.

## Webview boundary (A4.2)

- Production CSP is set (`script-src 'self'`, no `unsafe-eval`). `devCsp` is separate so Vite HMR is not granted in production.
- `withGlobalTauri: false`. Runtime detect uses `__TAURI_INTERNALS__` / `window.isTauri`.
- `tauri-plugin-fs` is not a desktop dependency. Frontend filesystem mutation is custom IPC, not plugin-fs. Folder picker remains `tauri-plugin-dialog`.
- OAuth authorization pages open in the **system browser** (`oauth_open_url` → `open::that`). Provider origins are not in the webview CSP.
- Static check: `npm run check:tauri-security`.

## Process and path authority (A4.3)

Sensitive mutating IPC:

1. caller path
2. canonicalize existing directory
3. resolve against `projects.json`
4. execute

Applied to engine turns, patch apply/rollback, PTY/terminal launch, command presets, LAN serve start, docs/notes writes, project JSON writes, recipes.

Visible terminals keep `program + argv + cwd` internally. macOS Terminal.app and Windows `cmd /k` are shell interfaces; quoting is isolated in `src-tauri/src/process_launch.rs`. Substring "dangerous command" lists are not the authorization control.

PTY architecture is unchanged (`portable-pty` + `CommandBuilder`). Empty cwd is no longer HOME; cwd must be a registered project (or a directory inside one).

Recipe execution was already argv-based. Recipe `cwd_rel` is confined with `path_safety::resolve_dir_in_workspace`.

## Remaining limitations

See [LIMITATIONS.md](../LIMITATIONS.md). In particular:

- Read-only IPC and some canvas/continuity commands still accept a caller `project_path` without the registered-project guard.
- Once a PTY is spawned in a registered cwd, it is a real interactive shell.
- Recipes may run any argv under the project cwd; there is no product-level program allowlist.
- LAN pairing is a local bearer, not E2E crypto or an IdP.
- The built-in LLM proxy bind policy is separate from LAN Agent Engine (loopback-safe; not redesigned in A4).
- Playwright e2e still reflects the v0.1.40 CLIProxyAPI sidecar contract and is not an A4 freeze gate.
