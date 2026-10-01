$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$outputDirectory = Join-Path $repositoryRoot "artifacts"
$outputPath = Join-Path $outputDirectory "architecture-baseline.json"
$solution = Join-Path $repositoryRoot "src\Functor.slnx"

$testProjects = @(
    "src/Functor.Tests/Functor.Tests.Domain/Functor.Tests.Domain.fsproj",
    "src/Functor.Tests/Functor.Tests.Application/Functor.Tests.Application.fsproj",
    "src/Functor.Tests/Functor.Tests.Architecture/Functor.Tests.Architecture.fsproj",
    "src/Functor.Tests/Functor.Tests.Rendering/Functor.Tests.Rendering.fsproj",
    "src/Functor.Tests/Functor.Tests.Workspace/Functor.Tests.Workspace.fsproj",
    "src/Functor.Tests/Functor.Tests.Input/Functor.Tests.Input.fsproj"
)

function Invoke-TimedDotnet {
    param(
        [string[]] $Arguments
    )

    $timer = [Diagnostics.Stopwatch]::StartNew()
    & dotnet @Arguments | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "dotnet $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }

    $timer.Stop()
    return $timer.Elapsed.TotalSeconds
}

Push-Location $repositoryRoot
try {
    $buildSeconds = Invoke-TimedDotnet @("build", $solution, "--configuration", "Release")
    $testResults = @()

    foreach ($project in $testProjects) {
        $testSeconds = Invoke-TimedDotnet @("test", "--project", $project, "--configuration", "Release", "--no-restore")
        $testResults += [ordered]@{
            project = $project
            seconds = [math]::Round($testSeconds, 3)
        }
    }

    $projectGraph = @{}
    Get-ChildItem (Join-Path $repositoryRoot "src") -Filter *.fsproj -Recurse | ForEach-Object {
        [xml] $projectFile = Get-Content $_.FullName
        $name = $_.BaseName
        $references = @(
            $projectFile.Project.ItemGroup.ProjectReference |
            ForEach-Object { [IO.Path]::GetFileNameWithoutExtension($_.Include) } |
            Where-Object { $_ }
        )
        $projectGraph[$name] = @($references | Sort-Object)
    }

    New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
    [ordered]@{
        generatedUtc      = [DateTime]::UtcNow.ToString("O")
        configuration     = "Release"
        buildSeconds      = [math]::Round($buildSeconds, 3)
        tests             = $testResults
        projectReferences = $projectGraph
    } | ConvertTo-Json -Depth 8 | Set-Content $outputPath

    Write-Host "Architecture baseline written to $outputPath"
}
finally {
    Pop-Location
}
