# Desktop IPC contract (A8)

**Status:** implemented for v0.2 A8 (incremental typed client; not a 181-command rewrite)
**Does not:** replace A3/A4 authorization, generate all Rust/TS bindings, or migrate Canvas/legacy Continuity reads

## Approach

Option B/C hybrid:

- Rust `generate_handler!` remains the registered-command identity source.
- A frontend **typed catalog** (`src/lib/ipc/catalog.ts`) is the migrated-command source of truth.
- All Tauri `invoke` goes through `src/lib/ipc` (`invokeTyped` or `legacyInvoke`).
- `scripts/check-ipc-contract.mjs` enforces identity, allowlisted transport, no typed/legacy overlap, and prints coverage.

Payload TypeScript types live at existing feature clients (`agentEngineClient`, `oauthClient`, …). The IPC layer is transport, not a second domain service.

## Layout (source)

| Group | Registered commands (approx.) | A8 status |
|-------|-------------------------------|-----------|
| Agent Engine / Chat / patch / recipes | `agent_*` | **Typed** |
| PTY / process | `pty_*`, `open_terminal`, `open_agent_cli`, `run_command_preset` | **Typed** |
| Secrets / OAuth | `agent_secret_*`, used `oauth_*` | **Typed** |
| LAN / pairing | `lan_*` | **Typed** |
| Provider runtime | `proxy_runtime_*`, `proxy_management_*` | **Typed** |
| Project / settings / reset | `get_settings`, `*_project*`, `reset_all_data_cmd`, lists | **Typed** |
| Sessions scan | `scan_*`, `read_foreign_session_transcript` | **Typed** |
| Path helpers | `validate_path`, `open_folder`, `get_host_os` | **Typed** |
| Continuity reads / mesh / team / trust UI | `continuity_*`, `mesh_*`, `team_*`, `connector_*`, `org_*`, `pilot_*`, `rc_*` | Legacy (`legacyInvoke`) |
| Docs / notes / sprint / presets | `list_docs*`, `*_note*`, `*_sprint*`, … | Legacy |
| Canvas | `canvas_*` | Legacy (out of A8 vanity 100%) |
| Voice / usage / extensions / updates | `voice_*`, `usage_*`, `extensions_*`, `download_and_open_update` | Legacy |
| Unused registered | `greet`, `detect_agent_session_roots`, some OAuth/runtime/usage ghosts | Uninvoked; reported, not typed |

`npm run check:ipc` prints live coverage. At A8 implementation:

```text
registered=181 typed=80 legacy=54 unused=10 unknown=0 duplicates=0
```

Unused remain desktop-only / local-only OAuth setters (not invoked from `src/`). Canvas, docs/notes, Continuity reads stay on `legacyInvoke`.

## Mutating vs read-only (migrated)

Mutating (host/domain still authoritative): patch apply/reject/rollback, recipe run/cancel, secret set/clear, OAuth connect/cancel/config writes, LAN start/stop/send/pair, proxy start/stop/management update, project add/remove/init/save/delete/reset, PTY write/kill, process spawn.

Read-only: status/list/get/scan/transcript/provider test (test still needs a key in-process).

## Security

A4 path/project/process guards remain in Rust. Typed IPC does not log secrets. OAuth management-secret setters stay local-only on the frontend (no IPC). Provider errors keep A5 client-side redaction in `builtinProxyClient`.

## Compatibility

```text
feature client
    → invokeTyped(catalog command)     // migrated
    → legacyInvoke(registered name)    // explicit leftover
    → Tauri invoke (only ipc/client.ts)
```

A command must not be both typed and legacy. New `@tauri-apps/api/core` `invoke` imports outside `src/lib/ipc/client.ts` fail CI.
