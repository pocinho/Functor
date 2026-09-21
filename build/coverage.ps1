$ErrorActionPreference = "Stop"

$coverageDirectory = Join-Path $PSScriptRoot "..\coverage"
$reportDirectory = Join-Path $coverageDirectory "report"

if (Test-Path $coverageDirectory) {
    Remove-Item $coverageDirectory -Recurse -Force
}

New-Item -ItemType Directory -Path $coverageDirectory | Out-Null

$testProjects = @(
    "src/Functor.Tests/Functor.Tests.Domain/Functor.Tests.Domain.fsproj",
    "src/Functor.Tests/Functor.Tests.Application/Functor.Tests.Application.fsproj",
    "src/Functor.Tests/Functor.Tests.Rendering/Functor.Tests.Rendering.fsproj",
    "src/Functor.Tests/Functor.Tests.Workspace/Functor.Tests.Workspace.fsproj",
    "src/Functor.Tests/Functor.Tests.Input/Functor.Tests.Input.fsproj"
)

foreach ($project in $testProjects) {
    $projectName = [IO.Path]::GetFileNameWithoutExtension($project)
    $outputPath = Join-Path $coverageDirectory "$projectName.cobertura.xml"

    dotnet test --project $project --coverage --coverage-output $outputPath --coverage-output-format cobertura
    if ($LASTEXITCODE -ne 0) {
        throw "Coverage test run failed for $project"
    }
}

dotnet tool restore
dotnet tool run reportgenerator `
    "-reports:$coverageDirectory\*.cobertura.xml" `
    "-targetdir:$reportDirectory" `
    "-reporttypes:Html"

if ($LASTEXITCODE -ne 0) {
    throw "ReportGenerator failed"
}

Write-Host "Coverage report: $reportDirectory\index.html"
Write-Host "Avalonia headless tests remain a separate behavior suite; run them with:"
Write-Host "dotnet test --project src/Functor.Tests/Functor.Tests.Avalonia/Functor.Tests.Avalonia.fsproj"
