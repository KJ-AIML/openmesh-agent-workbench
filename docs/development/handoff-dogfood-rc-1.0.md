# Handoff runbook — RC dogfood → 1.0.0 gate

Companion to `handoff-post-0.1.21.md`. Short operational script.

## A. Pull & build

```bash
cd repos/openmesh-agent-workbench
git checkout main && git pull
export DEVELOPER_DIR=/Library/Developer/CommandLineTools
export SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk
export PATH="/opt/homebrew/bin:$HOME/.cargo/bin:$PATH"
cargo test --workspace
npm run typecheck
```

## B. CLI readiness path

```bash
# Fresh lab root (or an existing OpenMesh project):
export PROJ="$(mktemp -d /tmp/openmesh-rc-dogfood-XXXXXX)"   # or "$PWD"

# Required first — profile/team need an initialized marker:
cargo run -p openmesh-cli -- init --project "$PROJ"

cargo run -p openmesh-cli -- profile init --owner-label You --role-label Owner --project "$PROJ"
cargo run -p openmesh-cli -- team init --name "RC Lab" --owner-label You --project "$PROJ"
cargo run -p openmesh-cli -- trust-admin init --project "$PROJ"
cargo run -p openmesh-cli -- pilot check --project "$PROJ"
cargo run -p openmesh-cli -- rc check --project "$PROJ"
```

Expect:

| Command | Ready | Not ready |
|---------|-------|-----------|
| `pilot check` | exit **0** | exit **2** |
| `rc check` | exit **0** | exit **2** |

Artifacts:

- `$PROJ/.openmesh/pilot/pack.json`
- `$PROJ/.openmesh/rc/pack.json`

## C. Optional depth

```bash
cargo run -p openmesh-cli -- team cloud init --mode local-sim --project "$PROJ"
cargo run -p openmesh-cli -- team cloud sync-scaffold --project "$PROJ"
cargo run -p openmesh-cli -- connector register --id gh-lab --kind github-stub --ref acme/lab --project "$PROJ"
cargo run -p openmesh-cli -- connector collect --id gh-lab --project "$PROJ"
cargo run -p openmesh-cli -- org graph show --project "$PROJ"
cargo run -p openmesh-cli -- rc matrix --project "$PROJ"
cargo run -p openmesh-cli -- rc freeze-policy --project "$PROJ"
```

## D. GUI

```bash
npm run tauri:dev
```

Check Continuity tabs: **Team · Trust · Connectors · Org · Pilot · RC**.

## E. If RC not ready

1. Read fail rows from `rc check --json` / Continuity RC tab.  
2. Fix only **P0/P1** (RC freeze forbids feature expansion).  
3. Re-run `pilot check` then `rc check`.  
4. Record evidence in ledger when claiming PASS.

## F. When ready for 1.0.0

1. Branch `feat/openmesh-1.0.0`  
2. Follow `docs/development/openmesh-1.0.0-execution-plan.md`  
3. Gate verification package + version `1.0.0` + CHANGELOG/ledger/matrix  
4. Tag `v1.0.0` only with evidence of RC PASS on real team usage  

## Freeze reminder (from RC pack)

**Allowed:** bugfix P0/P1, docs, tests, re-evaluate packs  
**Forbidden:** new domain features, scope creep, breaking protocols without migration  

## G. Session evidence (2026-08-03)

| Gate | Result | Notes |
|------|--------|-------|
| `git describe` | `v0.1.21` | `main` @ handoff commit era |
| `cargo test --workspace` | **PASS** | 1895 passed, 0 failed, 1 ignored (after flake fix) |
| `npm run typecheck` | **PASS** | exit 0 |
| CLI min path (`init`→profile→team→trust→pilot→rc) | **PASS** | `pilot_ready=true` exit 0; `rc_ready=true` p0=0 p1=0 exit 0 |
| CLI optional depth (cloud+connector+org+matrix+freeze) | **PASS** | pilot pass=6; rc still ready; matrix warn only on optional online-proxy |
| GUI `npm run tauri:dev` | **not performed** | still required before claiming Desktop RC smoke |
| Real multi-person team project | **not performed** | temp lab only (`/tmp/openmesh-rc-dogfood-*`); 1.0.0 still needs real-team evidence |

Flake fixed (tests, RC-allowed): `context_pack_and_continuity_views_coexist_without_semantic_coupling` compared full JSON including wall-clock `generatedAt` (1s race). Asserts now ignore `generatedAt`.
