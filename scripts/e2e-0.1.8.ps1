# OpenMesh 0.1.8 — local E2E dogfood (Handoff Note Engine)
# Run from repo root: repos/openmesh-agent-workbench

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $MyInvocation.MyCommand.Path | Split-Path -Parent
Set-Location $repoRoot

$tmp = Join-Path $env:TEMP "openmesh-e2e-0.1.8-$(Get-Random)"
New-Item -ItemType Directory -Path $tmp | Out-Null
Write-Host "temp project: $tmp"

try {
    cargo run -q -p openmesh-cli -- init --project $tmp
    if ($LASTEXITCODE -ne 0) { throw "init failed" }

    cargo run -q -p openmesh-cli -- signal progress --summary "E2E seed progress" --project $tmp
    if ($LASTEXITCODE -ne 0) { throw "signal seed failed" }

    $create = cargo run -q -p openmesh-cli -- handoff create --recipient "E2E Recipient" --role teammate --json --project $tmp
    if ($LASTEXITCODE -ne 0) { throw "handoff create failed" }
    $parsed = $create | ConvertFrom-Json
    $id = $parsed.handoffId
    Write-Host "created handoff: $id"

    cargo run -q -p openmesh-cli -- handoff show --id $id --project $tmp
    if ($LASTEXITCODE -ne 0) { throw "handoff show failed" }

    cargo run -q -p openmesh-cli -- handoff approve --id $id --link-event --project $tmp
    if ($LASTEXITCODE -ne 0) { throw "handoff approve failed" }

    $md = cargo run -q -p openmesh-cli -- handoff export --id $id --project $tmp 2>&1
    if ($LASTEXITCODE -ne 0) { throw "handoff export failed" }
    # PowerShell may capture multi-line stdout as Object[]; join before matching.
    $mdText = if ($md -is [System.Array]) { $md -join "`n" } else { [string]$md }
    if ($mdText -notmatch [regex]::Escape("# Handoff Note")) {
        throw "export missing markdown header"
    }

    Write-Host "E2E 0.1.8 handoff chain: OK"
}
finally {
    if (Test-Path $tmp) { Remove-Item -Recurse -Force $tmp }
}
