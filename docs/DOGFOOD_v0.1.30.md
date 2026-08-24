# Dogfood checklist — OpenMesh Desktop v0.1.30

**Build / tag:** `v0.1.30`
**Purpose:** Fillable pass/fail checklist for a real installed (or `tauri:dev`) session.
**Supersedes:** [DOGFOOD_v0.1.28.md](./DOGFOOD_v0.1.28.md) for current dogfood (keep 0.1.28 for historical ticks).
**Related:** [PRODUCT_GUIDE.md](./PRODUCT_GUIDE.md) · [CHAT.md](./CHAT.md) · [TERMINAL.md](./TERMINAL.md) · [SESSIONS.md](./SESSIONS.md) · [CONTINUITY_MESH.md](./CONTINUITY_MESH.md) · [SETTINGS.md](./SETTINGS.md) · [RELEASE_SMOKE.md](./RELEASE_SMOKE.md) · [LIMITATIONS.md](./LIMITATIONS.md)

> Agents cannot physically click the GUI for you. Tick each box yourself.
> Note any code-level issues found during release work at the bottom.

**How to mark:** `[x]` pass · `[ ]` fail / not run · write a one-line note after the item when something is off.

---

## 0. Download & install

- [ ] Downloaded the correct macOS DMG for this machine (`*_aarch64.dmg` Apple Silicon / `*_x64.dmg` Intel) from the [GitHub Release](https://github.com/kjct0s/openmesh-agent-workbench/releases/tag/v0.1.30) — or ran `npm run tauri:dev`
- [ ] Cleared quarantine if needed (`xattr -cr /Applications/OpenMesh.app` or `./scripts/macos-unquarantine.sh`) — see §8
- [ ] App opens after Gatekeeper remediation
- [ ] Project selected (or Add Project works)
- [ ] Settings → Provider: key + model set; **Test connection** succeeds
- [ ] Settings → About / Updates: **Check for updates** reports current build (or offers newer)
- [ ] **Download & install** (when a newer release exists) completes or shows honest unsigned-install guidance — no fake silent update

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

## 4. Continuity — Peers → Team → Trust → LAN

Route: `/continuity` · see [CONTINUITY_MESH.md](./CONTINUITY_MESH.md)

Guided strip should read **Peers → Team → Trust → LAN** (trusted-LAN alpha; no WAN).

### 4a. Peers

- [ ] Continuity → Mesh → **Peers** shows the guided step strip
- [ ] **Add peer** with label + optional `host:port` works; peer appears in list
- [ ] Empty state tells you to add a peer, then continue to Team
- [ ] Peer with LAN address shows presence dot (or honest unknown/offline)

### 4b. Team

- [ ] Team group → **Team** tab: empty state → **Initialize team** with a display name
- [ ] Roster shows owner after init
- [ ] **Add member** with peer dropdown (link to registered mesh peer)
- [ ] Linked member shows LAN address / presence when peer has `lanAddress`
- [ ] **Remove** works for non-sole-owner members
- [ ] Cloud sync is **not** presented as a finished desktop feature (CLI scaffold only)

### 4c. Trust

- [ ] Without team: clear CTA **Go to Team → Init**
- [ ] **Initialize trust policy** after team exists
- [ ] Toggle **remote query** on/off; audit row appears
- [ ] Set mode **allowlist-only**; active mode is visually indicated
- [ ] **Trust this peer** from peer dropdown; entry appears with optional presence dot
- [ ] **Remove** allowlist entry works; audit shows remove
- [ ] Copy stays honest: policy gating, not finished-product E2E crypto

### 4d. LAN + Ask + Chat

- [ ] **Start listener** / Stop / Refresh discover work
- [ ] Trusted peer shows a **green presence dot** when online (or honest offline state)
- [ ] LAN Ask to a ready peer returns a live Agent Engine answer (markdown)
- [ ] Missing API key on peer surfaces a clear error (not a fake success)
- [ ] Chat tab: send a short message both ways when both listeners are up

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

## 6. Check for updates / Download & install

Settings → Updates (or About / Updates panel)

- [ ] **Check for updates** runs against GitHub Releases
- [ ] Current build reports as up to date for `0.1.30` (or correctly offers a newer tag if one exists)
- [ ] **Download & install** path is usable when an update is available (or guidance for unsigned installs is honest)
- [ ] No fake auto-update install success when Gatekeeper / unsigned policy blocks it

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
| Mesh WAN / relay cloud | Still out of scope — LAN alpha only. | n/a |
| Notarization / signed DMG | Still expected Gatekeeper friction on macOS. | release |
| Team/Trust desktop | Init, members add/remove, trust init, remote-query, modes, allowlist add/remove, audit — desktop-complete for LAN dogfood. | UX |

Fill rows above if you find more while testing.
