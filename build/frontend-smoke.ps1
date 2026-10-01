$ErrorActionPreference = "Stop"
$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Invoke-Dotnet {
    param([string[]] $Arguments)

    & dotnet @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

Push-Location $repositoryRoot
try {
    Invoke-Dotnet @(
        "test",
        "src/Functor.Tests/Functor.Tests.Avalonia/Functor.Tests.Avalonia.fsproj",
        "--no-restore"
    )

    foreach ($project in @(
            "src/Functor.Avalonia.Browser/Functor.Avalonia.Browser.fsproj"
            "src/Functor.Avalonia.Android/Functor.Avalonia.Android.fsproj"
        )) {
        Invoke-Dotnet @("build", $project, "--no-restore")
    }

    $browserArtifact = Join-Path $repositoryRoot "src/Functor.Avalonia.Browser/bin/Debug/net10.0-browser/browser-wasm/Functor.Avalonia.Browser.dll"
    if (-not (Test-Path $browserArtifact)) {
        throw "Browser smoke artifact was not produced: $browserArtifact"
    }

    $androidArtifacts = Get-ChildItem (Join-Path $repositoryRoot "src/Functor.Avalonia.Android/bin/Debug") -Filter "Functor.Avalonia.Android.dll" -Recurse -File
    if ($androidArtifacts.Count -eq 0) {
        throw "Android smoke artifact was not produced."
    }

    Write-Host "Frontend smoke validation completed."
}
finally {
    Pop-Location
}
