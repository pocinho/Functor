[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$appRoot = Join-Path $repoRoot "src\functor"
$webRoot = Join-Path $repoRoot "src\functor_web"
$target = "x86_64-pc-windows-msvc"

foreach ($commandName in @("cargo", "rustup")) {
    if (-not (Get-Command $commandName -ErrorAction SilentlyContinue)) {
        throw "Required command '$commandName' was not found on PATH."
    }
}

$npmCommand = Get-Command npm -ErrorAction SilentlyContinue
if ($npmCommand) {
    $npmPath = $npmCommand.Source
}
else {
    $npmPath = Join-Path $env:ProgramFiles "nodejs\npm.cmd"
}
if (-not $npmPath -or -not (Test-Path -LiteralPath $npmPath -PathType Leaf)) {
    throw "npm was not found on PATH or in the system Node.js installation."
}
$env:Path = "$(Split-Path -Parent $npmPath);$env:Path"

$installedTargets = & rustup target list --installed
if ($LASTEXITCODE -ne 0) {
    throw "Could not query installed Rust targets."
}
if ($installedTargets -notcontains $target) {
    throw "Rust target '$target' is not installed. Install it with: rustup target add $target"
}

$tauriCli = Join-Path $appRoot "node_modules\.bin\tauri.cmd"
if (-not (Test-Path -LiteralPath $tauriCli)) {
    Write-Host "Installing locked Tauri dependencies with npm ci..."
    & $npmPath --prefix $appRoot ci
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri dependency installation failed with exit code $LASTEXITCODE."
    }
}

$viteCli = Join-Path $webRoot "node_modules\.bin\vite.cmd"
if (-not (Test-Path -LiteralPath $viteCli)) {
    Write-Host "Installing locked web dependencies with npm ci..."
    & $npmPath --prefix $webRoot ci
    if ($LASTEXITCODE -ne 0) {
        throw "Web dependency installation failed with exit code $LASTEXITCODE."
    }
}

Push-Location $repoRoot
try {
    $metadataJson = & cargo metadata --no-deps --format-version 1 --manifest-path (Join-Path $repoRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "Could not read Cargo workspace metadata."
    }
    $cargoMetadata = $metadataJson | ConvertFrom-Json -ErrorAction Stop
    $targetDirectory = $cargoMetadata.target_directory
    if (-not $targetDirectory) {
        throw "Cargo metadata did not report a target directory."
    }

    & $npmPath --prefix $appRoot run tauri -- build --target $target --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "The Tauri release build failed with exit code $LASTEXITCODE."
    }

    $executable = Join-Path $targetDirectory "$target\release\functor.exe"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Tauri reported success, but the expected executable was not found: $executable"
    }

    Write-Output "Built release executable: $executable"
}
finally {
    Pop-Location
}
