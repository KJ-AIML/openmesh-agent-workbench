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
export PROJ="$PWD"   # or another OpenMesh project root

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
