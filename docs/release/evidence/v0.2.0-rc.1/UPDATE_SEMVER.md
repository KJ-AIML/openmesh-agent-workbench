# A13.12 Update and Version Comparison Behavior Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Module:** `src/lib/updates/semver.ts`
- **Result:** **PASS**

## Packaged Version Identity Verification
- Packaged macOS bundle Info.plist reports: `CFBundleShortVersionString: 0.2.0-rc.1`
- Core/CLI binary string reports: `0.2.0-rc.1`
- `npm run check:version-consistency` reports 5/5 declaration manifests and 2/2 lockfiles synchronized to `0.2.0-rc.1`.

## Prerelease Semver Comparison Verification

| Current Version | Remote Release | Comparison Result | Update Available? | Interpretation | Status |
| :--- | :--- | :---: | :---: | :--- | :---: |
| `0.2.0-rc.1` | `0.2.0` | `-1` | Yes | Stable release takes precedence over RC | **PASS** |
| `0.2.0-rc.1` | `0.2.0-rc.2` | `-1` | Yes | Subsequent RC increments recognized | **PASS** |
| `0.1.40` | `0.2.0-rc.1` | `-1` | Yes | Legacy v0.1 recognizes v0.2 candidate | **PASS** |
| `0.2.0-rc.1` | `0.2.0-rc.1` | `0` | No | Identical candidate version ignored | **PASS** |
| `0.2.0-rc.1` | `0.1.40` | `1` | No | Downgrade offers suppressed | **PASS** |

## Prerelease Feed Discrimination
- In `src/lib/updates/githubReleases.ts`, `isPrereleaseVersion("0.2.0-rc.1")` returns `true`, correctly directing queries to `RELEASES_LIST_URL` to evaluate both stable and prerelease candidates without publishing an unapproved release.
