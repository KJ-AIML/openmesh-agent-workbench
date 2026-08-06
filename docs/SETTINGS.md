# Settings

> Route: `/settings` (`?section=`) · Code: `src/pages/SettingsPage.vue`, `src/components/settings/`  
> Related: [CHAT.md](./CHAT.md) · [SESSIONS.md](./SESSIONS.md) · [LIMITATIONS.md](./LIMITATIONS.md)

## Contents

1. [IA](#ia)
2. [Setup](#setup)
3. [Runtime](#runtime)
4. [Project](#project)
5. [App](#app)
6. [Secrets](#secrets)
7. [Legacy redirects](#legacy-redirects)
8. [Dogfood](#dogfood)

---

## IA

Sticky section nav grouped as:

| Group | Sections |
|-------|----------|
| **Setup** | Overview · Provider · Voice |
| **Runtime** | Agents · Extensions · Sessions · Server |
| **Project** | Tools · Paths |
| **App** | Appearance · Data · About |

Section ids: `overview` `provider` `voice` `agents` `extensions` `sessions` `server` `tools` `paths` `appearance` `data` `about`.

---

## Setup

### Overview

Checklist / status hub (project selected, provider ready, etc.). Usage/status content folded here from old standalone pages.

### Provider

- OpenAI-compatible base URL / provider name
- Default + coding model fields
- **Save API key** → user secret file (not project JSON)
- **Test connection** — minimal tool-free probe
- Coding Plan endpoints that can’t do Agent Engine chat fail fast with guidance

### Voice

- STT/TTS preferences (`VoiceSettingsPanel.vue`)
- Works with Voice HUD + chat bridge when enabled

---

## Runtime

### Agents

CLI binary overrides / agent launcher related settings (Codex, Claude, OpenCode, …). PATH defaults; optional absolute overrides.

### Extensions

Skills / hooks / plugins MVP (`SettingsExtensionsPanel.vue`):

- Markdown skill packs
- Lifecycle hooks: `on_chat_start`, `on_before_turn`, `on_after_turn`
- Folder plugins (`openmesh.plugin.json`) enable/disable
- Load from user config + project `.openmesh/` + builtin `catalog/skills/`
- IPC: `extensions_list|catalog|set_enabled|install`
- Settings persisted under `settings.extensions` — **not** the secrets file

### Sessions

Optional session root overrides for Codex / Claude / OpenCode / Cursor / Gemini / Grok. Empty = auto-detect. See [SESSIONS.md](./SESSIONS.md).

### Server

Local server-related settings (legacy Server page content).

---

## Project

### Tools

Project tools panel: terminal prefs / command presets (`SettingsToolsPanel.vue`). Formerly Dev Connector surface.

### Paths

Project path / tooling path presentation.

---

## App

### Appearance

Theme / shell appearance (`SettingsAppearancePanel.vue`, `src/lib/appearance.ts`):

- Theme (dark / light / system), font size, density
- **Top navbar tabs** — toggle which hot tabs show in the titlebar (`Chat` / `Work` / `Docs` / `Sprint`). Persisted with other appearance prefs; at least one stays enabled. Sidebar navigation is unchanged.

### Data

Reset / data management controls (destructive actions confirm in UI).

### About / Updates

- App version from package / Tauri config
- GitHub releases update check (`SettingsUpdatesPanel.vue`, `src/lib/updates/`)
- When a newer release has a matching installer for this OS/arch: primary **Download & install** downloads the asset (Tauri/`reqwest`) and opens it (macOS `.dmg` via `open`, Windows `.exe`/`.msi`, Linux AppImage/deb). Does **not** silently replace the running app.
- If assets are still uploading: Install disabled with “Installers not ready yet…”; **Open release** still works.
- Empty release body: show “Assets ready — see GitHub…” when installers exist (notes do not block install).
- Copy notes that **preview builds are unsigned** (macOS Gatekeeper “damaged” → `xattr -cr` / right-click Open; Windows SmartScreen)
- IPC: `get_host_arch`, `download_and_open_update`; progress event `update-download-progress`

---

## Secrets

| Store | Path / vars |
|-------|-------------|
| File | `{dirs::config_dir}/openmesh/agent-api-key` — macOS ≈ `~/Library/Application Support/openmesh/agent-api-key`; Linux often `~/.config/openmesh/agent-api-key` |
| Env | `OPENMESH_AGENT_API_KEY` → `OPENAI_API_KEY` → `DEEPSEEK_API_KEY` |

Never written into `<project>/.openmesh/` JSON. UI may still show a shorthand `~/.config/...` path — resolve via OS config dir.

IPC: `agent_secret_status|set|clear`.

---

## Legacy redirects

| Old route | Goes to |
|-----------|---------|
| `/models` | `?section=provider` |
| `/dev-connector` | `?section=tools` |
| `/server` | `?section=server` |
| `/status` | `?section=overview` |
| `/usage` | `?section=overview` |

---

## Dogfood

Full fillable checklist: [DOGFOOD_v0.1.28.md](./DOGFOOD_v0.1.28.md) (§5 Appearance, §6 Updates).

1. Overview — confirm checklist items  
2. Provider — save key → Test connection → green/usable result  
3. Extensions — list builtins; toggle one; start a chat turn and look for skill influence  
4. Sessions — leave overrides blank; confirm Sessions page still scans  
5. Appearance — change theme/density; toggle Top navbar tabs (hide Sprint, confirm titlebar updates; last tab stays on); reload  
6. About / Updates — version matches current tag · Check for updates · **Download & install** when available · confirm installer opens · read unsigned-build / Gatekeeper note  

