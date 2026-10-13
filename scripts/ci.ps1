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
Step 'cargo test'    { cargo test --workspace }
Step 'markdown links' { & "$PSScriptRoot\check-docs-links.ps1" }
Write-Output 'All checks passed.'
