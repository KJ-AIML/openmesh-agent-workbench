# Dogfood checklist — OpenMesh Desktop v0.1.28

**Build / tag:** `v0.1.28`  
**Purpose:** Fillable pass/fail checklist for a real installed (or `tauri:dev`) session.  
**Related:** [PRODUCT_GUIDE.md](./PRODUCT_GUIDE.md) · [CHAT.md](./CHAT.md) · [TERMINAL.md](./TERMINAL.md) · [SESSIONS.md](./SESSIONS.md) · [CONTINUITY_MESH.md](./CONTINUITY_MESH.md) · [SETTINGS.md](./SETTINGS.md) · [RELEASE_SMOKE.md](./RELEASE_SMOKE.md) · [LIMITATIONS.md](./LIMITATIONS.md)

> Agents cannot physically click the GUI for you. Tick each box yourself.  
> Note any code-level issues found during release work at the bottom.

**How to mark:** `[x]` pass · `[ ]` fail / not run · write a one-line note after the item when something is off.

---

## 0. Install / launch

- [ ] Downloaded the correct macOS DMG for this machine (`*_aarch64.dmg` Apple Silicon / `*_x64.dmg` Intel) from the [GitHub Release](https://github.com/kjct0s/openmesh-agent-workbench/releases/tag/v0.1.28) — or ran `npm run tauri:dev`
- [ ] App opens after Gatekeeper remediation (see §7)
- [ ] Project selected (or Add Project works)
- [ ] Settings → Provider: key + model set; **Test connection** succeeds

---

## 1. Chat composer (`/` `@`, mode dropdown, one shell)

Route: `/agent-chat` · see [CHAT.md](./CHAT.md)

- [ ] Composer shows a **single** shell (not a dense dual toolbar)
- [ ] Mode dropdown cycles Ask / Plan / Act / Delegate without layout jump
- [ ] Typing `/` opens slash menu; `/tools` or `/help` lists tools
- [ ] Typing `@` opens mentions (project / file / doc / note / terminal / canvas as available)
- [ ] Quiet status icons (Working / Terminal / Canvas) sit in the composer chrome without clutter
- [ ] Send a short Ask message; Stop cancels a long turn if tried
- [ ] Mid-turn: Working chip appears; Terminal chip / session-run rows update for verify / shell-like tools (not only after the reply lands)

**Notes:**

---

## 2. Terminal right sidebar (PTY, +, resize, chat scroll)

See [TERMINAL.md](./TERMINAL.md)

- [ ] Terminal icon opens a **right** sidebar (default dock)
- [ ] `+` creates a new PTY tab; switching tabs works
- [ ] Drag resize (width) works; chat transcript still scrolls independently
- [ ] Dock toggle right ↔ bottom persists across reopen
- [ ] Session runs list shows verify / long tool rows with elapsed time
- [ ] Expandable output on a finished session run (when output was captured)
- [ ] Closing the panel does not kill the chat; reopening restores dock preference

**Notes:**

---

## 3. Continue in Chat imports (roles You / Assistant)

Route: `/agent-sessions` · see [SESSIONS.md](./SESSIONS.md)

- [ ] **Cursor** session appears in scan (or import path works with a known Cursor transcript)
- [ ] Continue in Chat opens Agent Chat with imported turns
- [ ] Imported human turns render as **You** (not raw `user`)
- [ ] Imported model turns render as **Assistant**
- [ ] **Grok** / xAI-family session import (or Continuity path) also maps You / Assistant correctly
- [ ] Imported thread survives app restart (disk under `<project>/.openmesh/agent/chats/`)

**Notes:**

---

## 4. Continuity — LAN + presence + Ask markdown

Route: `/continuity` · see [CONTINUITY_MESH.md](./CONTINUITY_MESH.md)

- [ ] LAN discovery / last-known peer list loads without crash
- [ ] Trusted peer shows a **green presence dot** when online (or honest offline state)
- [ ] LAN Ask to a ready peer returns a live Agent Engine answer
- [ ] Ask answer renders **markdown** (headings / lists / code) safely
- [ ] Missing API key on peer surfaces a clear error (not a fake success)

**Notes:**

---

## 5. Appearance — theme / density + Top navbar tabs

Settings → Appearance · see [SETTINGS.md](./SETTINGS.md)

- [ ] Theme switch (light / dark / system as offered) applies immediately
- [ ] Density / spacing control changes chrome density without breaking composer
- [ ] **Top navbar tabs** toggles show/hide Chat / Work / Docs / Sprint in the titlebar
- [ ] Toggles persist after quit + relaunch

**Notes:**

---

## 6. Check for updates

Settings → Updates (or About / Updates panel)

- [ ] **Check for updates** runs against GitHub Releases
- [ ] Current build reports as up to date for `0.1.28` (or correctly offers a newer tag if one exists)
- [ ] Guidance for unsigned installs is honest (no fake auto-update install)

**Notes:**

---

## 7. macOS Gatekeeper / `xattr -cr`

See [LIMITATIONS.md — Gatekeeper](./LIMITATIONS.md#macos-gatekeeper-damaged--wont-open)

- [ ] Fresh download from GitHub shows quarantine / “damaged” (expected for unsigned)
- [ ] `xattr -cr /Applications/OpenMesh.app` (or `./scripts/macos-unquarantine.sh`) clears it
- [ ] App then opens with **Open** / **Open Anyway**
- [ ] Release notes mention aarch64 vs x64 DMG pick

**Notes:**

---

## Agent notes (code-level; not a substitute for GUI ticks)

| Area | Observation | Severity |
|------|-------------|----------|
| Release CI | Empty `APPLE_*` secret env mapping must stay **out** of `release.yml` (fixed in 0.1.28; guarded in docs + CI script). | release |
| Chat streaming | Mid-turn Agent Engine tool progress should feed Working / Terminal session runs (shipped with this dogfood cycle when present in Unreleased / 0.1.29). | UX |
| Mesh WAN / Team redesign | Out of scope for this dogfood pass. | n/a |

Fill rows above if you find more while testing.
