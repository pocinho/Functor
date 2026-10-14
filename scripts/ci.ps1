<#
.SYNOPSIS
  Runs the same checks as the blocking CI jobs, locally.
#>
$ErrorActionPreference = 'Stop'
function Step($name, [scriptblock]$cmd) {
    Write-Output "==> $name"
    & $cmd
    if ($LASTEXITCODE -ne 0) { throw "$name failed" }
}

Step 'cargo fmt'     { cargo fmt --all --check }
Step 'cargo clippy'  { cargo clippy --workspace --all-targets --all-features -- -D warnings }
Step 'dependency edges' { & "$PSScriptRoot\check-deps.ps1" }
Step 'rfc conformance' { & "$PSScriptRoot\check-conformance.ps1" }
Step 'cargo test'    { cargo test --workspace --all-features }
Step 'cargo test (release feature set)' { cargo test -p functor-cli }
# Tests that start real processes keep their repositories under tests/scratch (git-ignored) and
# remove them themselves; this clears what a killed run could leave behind.
$scratch = Join-Path (Split-Path $PSScriptRoot) 'tests'
if (Test-Path $scratch) { Remove-Item -Recurse -Force $scratch }
Step 'markdown links' { & "$PSScriptRoot\check-docs-links.ps1" }
Write-Output 'All checks passed.'
