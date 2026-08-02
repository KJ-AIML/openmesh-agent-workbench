# OpenMesh Handoff — Post v0.1.21 (1.0 RC Program)

**Date:** 2026-08-03  
**Repo:** `openmesh-agent-workbench` (`main`)  
**Latest release:** [v0.1.21](https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.1.21)  
**Audience:** Next human or agent session continuing OpenMesh shipping / dogfood / 1.0.0  
**Status:** Sequential matrix **0.1.15 → 0.1.21 RELEASED**. Open PRs: **none**. Next track: **1.0.0 gate only**.

---

## 1. TL;DR for the next agent

1. Work only in **target repo**:  
   `repos/openmesh-agent-workbench` (Heli parent: `openmesh-ws`).
2. **Do not invent features** under RC freeze unless fixing P0/P1 or amending Product Bible.
3. Recommended first action: **dogfood** `pilot check` + `rc check` on a real project.
4. Only after RC holds at real-team scale: implement **1.0.0** gate verification package and cut release.
5. macOS builds: prefer CommandLineTools env (see §7) if Xcode license blocks `cc`.

---

## 2. Current product state

### Released tracks (this ship wave)

| Tag | Mission | Key PR |
|-----|---------|--------|
| v0.1.15 | Team Workspace Foundation | #7 |
| v0.1.16 | Team Cloud Beta + desktop chrome polish | #8 |
| v0.1.17 | Trust, Privacy & Admin Beta | #9 |
| v0.1.18 | Connector Layer (evidence producers only) | #10 |
| v0.1.19 | Org Graph Preview + desktop IA | #11 |
| v0.1.20 | Enterprise Pilot Readiness | #12 |
| **v0.1.21** | **1.0 RC Program** | **#13** |

Unlock matrix: `docs/development/unlock-matrix-all.md`  
Ledger: `docs/development/execution-ledger.md`  
Changelog: `CHANGELOG.md`

### Global invariants (never waive)

1. Remote teammate query **read-only by default**.
2. Selective sync stays **selective** (no full-repo upload).
3. Authority ladder: AI never more certain than evidence.
4. Secret / sensitivity **fail-closed**.
5. Desktop and CLI are **peers over `openmesh-core`** (not CLI-only shells for core logic).
6. Claims need tests, dogfood, or explicit “not performed”.

---

## 3. Architecture map (where code lives)

| Concern | Core module | CLI | Desktop / Tauri |
|---------|-------------|-----|-----------------|
| Team registry | `team` | `team init\|show\|member\|query` | Continuity → Team |
| Team cloud (local-sim) | `team_cloud` | `team cloud init\|show\|sync-scaffold` | (via pilot/rc) |
| Trust / privacy | `trust_admin` | `trust-admin *` | Continuity → Trust |
| Connectors | `connectors` | `connector register\|list\|show\|collect` | Continuity → Connectors |
| Org graph | `org_graph` | `org graph show` | Continuity → Org |
| Pilot pack | `pilot` | `pilot check\|show\|runbook\|threats` | Continuity → Pilot |
| RC pack | `rc` | `rc check\|show\|matrix\|freeze-policy` | Continuity → RC |
| Mesh / proxy / relay | `mesh`, `online_proxy`, `relay`, … | existing mesh/online-proxy/relay | Continuity tabs |
| Sprint / chrome | Vue only | — | Titlebar, Sidebar, Sprint, Home |

**Storage root per project:** `<project>/.openmesh/`  
Notable new dirs: `team/`, `team-cloud/`, `trust-admin/`, `connectors/`, `pilot/`, `rc/`.

**Desktop chrome (macOS):**  
`src-tauri/tauri.macos.conf.json` — Overlay titlebar, `trafficLightPosition` `{x:16,y:20}`.  
Project name chip on **titlebar right**; sidebar under traffic lights is empty drag rail.  
Dock icon regenerated with safe padding (`src-tauri/icons/`).

**Sidebar IA:** Work · Team/Mesh · Agents · System (collapsed by default).

---

## 4. Dogfood checklist (do this before 1.0.0)

Use a real project path (example: this monorepo parent or a lab clone).

```bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"

cd repos/openmesh-agent-workbench
# Build CLI once
cargo build -p openmesh-cli

PROJ="/path/to/your/project"   # must be an OpenMesh-initialized project root
```

### Minimum path to `rc_ready`

```bash
# If not already an OpenMesh project:
# openmesh-cli init --project "$PROJ"   # or create via Desktop

openmesh-cli profile init --owner-label You --role-label Owner --project "$PROJ"
openmesh-cli team init --name "Lab Team" --owner-label You --project "$PROJ"
openmesh-cli trust-admin init --project "$PROJ"
# optional but good:
# openmesh-cli team cloud init --mode local-sim --project "$PROJ"
# openmesh-cli connector register --id gh-lab --kind github-stub --ref org/repo --project "$PROJ"

openmesh-cli pilot check --project "$PROJ"   # exit 0 ready, 2 not ready
openmesh-cli rc check --project "$PROJ"      # exit 0 rc_ready, 2 not ready
openmesh-cli rc matrix --project "$PROJ"
openmesh-cli org graph show --project "$PROJ"
```

### GUI smoke

```bash
npm run tauri:dev
```

- [ ] macOS traffic lights not glued to top; dock icon not oversized  
- [ ] Project name on **nav right** (not under lights)  
- [ ] Sidebar groups readable; System collapsed  
- [ ] Sprint: empty board, no mock seed tasks; add + drag columns  
- [ ] Continuity: Team / Trust / Connectors / Org / Pilot / RC tabs load without panic  
- [ ] Agent Sessions: no permanent “Mock” badge; Scan empty state works  

### Verify suite (pre-release always)

```bash
cargo test --workspace
npm run typecheck
# optional: npm run verify
```

---

## 5. What “done” looks like for 1.0.0

Plan: `docs/development/openmesh-1.0.0-execution-plan.md`

**Mission:** Ship 1.0 when product gates hold at real team scale.  
**Non-goals:** Scope creep without Product Bible amendment.

Suggested 1.0.0 package (implementation still open):

1. **Gate verification pack** — formal evidence that RC PASS holds (pilot + rc + regression matrix + dogfood notes).  
2. **Version bump** to `1.0.0` across package.json / Cargo crates / tauri.conf.  
3. **CHANGELOG + ledger + unlock matrix** → RELEASED.  
4. **Tag `v1.0.0` + GitHub release.**  
5. Optional: thin Desktop “1.0 status” surface if useful; not required for gate.

Do **not** treat 1.0.0 as another feature dump — it is a **gate**.

---

## 6. Known risks / footguns

| Risk | Mitigation |
|------|------------|
| Xcode license / `cc` exit 69 on macOS | `DEVELOPER_DIR` + `SDKROOT` → CommandLineTools |
| Tauri `#[tauri::command]` count hard-coded to 53 in many tests | Update all if adding lib.rs-level commands |
| Continuity has many tabs | Prefer IA cleanup later; don’t expand without need |
| Agent sessions “Mock” history | UI cleaned; old stored sessions may still exist as “Saved” |
| `team query` without allowlist | With trust-admin `allowlist-only`, link peers + allowlist first |
| Claiming multi-region / SLA | Explicit non-goal in pilot/rc limitations |

---

## 7. Dev environment notes

```bash
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"
```

- Node ≥ 20, `npm` scripts: `typecheck`, `test`, `verify`, `tauri:dev`  
- Rust workspace: `openmesh-core`, `openmesh-cli`, Tauri crate `openmesh`  
- Parent Heli workspace: `openmesh-ws` — follow `Agents.md` → harness adapters when required  

---

## 8. Key file index for next edits

```
crates/openmesh-core/src/{team,team_cloud,trust_admin,connectors,org_graph,pilot,rc}/
crates/openmesh-cli/src/{team,trust_admin,connector,org,pilot,rc}.rs
src-tauri/src/continuity_desktop.rs    # Tauri IPC surface
src/pages/ContinuityPage.vue
src/components/{Sidebar,Titlebar}.vue
src/pages/SprintPage.vue
docs/development/unlock-matrix-all.md
docs/development/execution-ledger.md
CHANGELOG.md
```

---

## 9. Session history (for context)

Ship wave after **Unlock all** (0.1.15–1.0.0 authorized):

- Desktop: macOS chrome, project name on nav, sprint without mock seeds, dock icon padding  
- Domain: team → cloud → trust → connectors → org → pilot → rc  
- All cut as tags `v0.1.15` … `v0.1.21` with PRs #7–#13  

**Open work intentionally left:** real-project RC dogfood evidence; **1.0.0** gate package.

---

## 10. Handoff acceptance (next session)

Mark complete when:

- [ ] Read this doc + unlock matrix + latest ledger entry  
- [ ] `main` at / past `v0.1.21`; `git status` clean or understood  
- [ ] Ran or explicitly skipped dogfood with reason  
- [ ] Either fixed P0/P1 from dogfood **or** started 1.0.0 gate work **or** closed session with status  

**Recommended first command after pull:**

```bash
git checkout main && git pull
git describe --tags --abbrev=0   # expect v0.1.21 or later
```
