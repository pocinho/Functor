<#
.SYNOPSIS
  Reports Phase 1 execution overhead, evidence storage and bounded-resource checks.
.DESCRIPTION
  Uses a release CLI and the deterministic test-only fixture. Reports medians after one warm-up,
  including direct child timings separately from end-to-end timings. No performance threshold is
  enforced; incorrect outcomes, byte counts or resource-limit behavior fail the check.
  All generated repositories stay under tests/scratch and are removed in finally.
.EXAMPLE
  .\scripts\bench-execution.ps1 -Report docs\quality\benchmarks\phase-1.json
#>
param(
    [ValidateRange(1, 20)][int]$Runs = 5,
    [string]$Report
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Report) { $Report = [System.IO.Path]::GetFullPath($Report, (Get-Location).Path) }

cargo build --release -p functor-cli --features test-support | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'release build failed' }
$meta = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$suffix = if ($IsWindows) { '.exe' } else { '' }
$cli = Join-Path $meta.target_directory "release\functor$suffix"
$fixture = Join-Path $meta.target_directory "release\execution-fixture$suffix"
$scratch = Join-Path (Get-Location) 'tests\scratch'
New-Item -ItemType Directory -Force -Path $scratch | Out-Null
$root = Join-Path $scratch ("bench-execution-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $root | Out-Null

function Measure-Process([string]$program, [string[]]$arguments, [int]$expected = 0) {
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $program
    $psi.WorkingDirectory = $root
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.Environment.Remove('FUNCTOR_REPO') | Out-Null
    $psi.Environment.Remove('FUNCTOR_TEST_FIXED_TIME') | Out-Null
    $psi.Environment.Remove('FUNCTOR_TEST_ID_SEED') | Out-Null
    foreach ($argument in $arguments) { $psi.ArgumentList.Add($argument) }
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $process = [Diagnostics.Process]::Start($psi)
    try {
        $process.StandardInput.Close()
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        $peak = 0L
        while (-not $process.HasExited) {
            $process.Refresh()
            $peak = [math]::Max($peak, $process.PeakWorkingSet64)
            if ($clock.Elapsed.TotalSeconds -gt 120) {
                $process.Kill($true)
                throw "benchmark command exceeded 120 seconds: $program $arguments"
            }
            Start-Sleep -Milliseconds 10
        }
        $process.WaitForExit()
        $clock.Stop()
        $text = $stdout.GetAwaiter().GetResult()
        $errorText = $stderr.GetAwaiter().GetResult()
        if ($process.ExitCode -ne $expected) {
            throw "$program $arguments exited $($process.ExitCode), expected ${expected}: $errorText"
        }
        if ($errorText) { throw "unexpected diagnostic from $program ${arguments}: $errorText" }
        [pscustomobject]@{ ms = $clock.Elapsed.TotalMilliseconds; peak_bytes = $peak; stdout = $text }
    }
    finally {
        if (-not $process.HasExited) { $process.Kill($true); $process.WaitForExit() }
        $process.Dispose()
    }
}

function Invoke-Cli([string[]]$arguments, [int]$expected = 0) {
    Measure-Process $cli $arguments $expected
}

function Add-Workflow([string]$name, [string[]]$arguments, [string]$declarations = '') {
    $quoted = ($arguments | ForEach-Object { ConvertTo-Json -InputObject $_ -Compress }) -join ', '
    $text = "schema = `"functor.workflow/1`"`nprovider = `"local-process`"`nprogram = `"tools/fixture$suffix`"`nargs = [$quoted]`n$declarations"
    [IO.File]::WriteAllText((Join-Path $root "$name.toml"), $text)
    $null = Invoke-Cli @('artifact', 'create', $name, '--type', 'workflow', '--content-path', "$name.toml")
}

function Median($samples) {
    [math]::Round(($samples | Sort-Object)[[int][math]::Floor($samples.Count / 2)], 2)
}

try {
    $null = Invoke-Cli @('init', '--name', 'Phase 1 benchmark')
    New-Item -ItemType Directory -Path (Join-Path $root 'tools') | Out-Null
    Copy-Item -LiteralPath $fixture -Destination (Join-Path $root "tools\fixture$suffix")
    $tool = Join-Path $root "tools\fixture$suffix"
    Add-Workflow 'noop' @('exit', '0')
    $direct = @(); $endToEnd = @(); $peaks = @()
    for ($i = 0; $i -le $Runs; $i++) {
        $child = Measure-Process $tool @('exit', '0')
        $run = Invoke-Cli @('run', 'start', 'noop', '--grant-exec', '-o', 'json')
        $doc = $run.stdout | ConvertFrom-Json
        if ($doc.run.state -ne 'succeeded' -or -not $doc.created) { throw 'no-op run was not a fresh success' }
        if ($i -gt 0) { $direct += $child.ms; $endToEnd += $run.ms; $peaks += $run.peak_bytes }
    }

    $inputBytes = 1MB
    [IO.File]::WriteAllBytes((Join-Path $root 'input.bin'), [byte[]]::new($inputBytes))
    $null = Invoke-Cli @('artifact', 'create', 'input', '--content-path', 'input.bin')
    Add-Workflow 'echo' @('echo-input', 'inputs/data', 'outputs/result') "[inputs]`ndata = { required = true }`n[outputs]`nresult = { type = `"dataset`", format = `"binary`" }`n"
    $directEcho = @(); $echo = @()
    for ($i = 0; $i -le $Runs; $i++) {
        $child = Measure-Process $tool @('echo-input', 'input.bin', 'direct-output.bin')
        $run = Invoke-Cli @('run', 'start', 'echo', '--input', 'data=input', '--grant-exec', '-o', 'json')
        $doc = $run.stdout | ConvertFrom-Json
        if ($doc.run.state -ne 'succeeded' -or $doc.run.result.outputs.Count -ne 1) { throw 'echo failed' }
        if ($i -eq 0) {
            $capture = $doc.run.inputs.data
            $workflowCapture = $doc.run.workflow_capture
            $originalOutput = $doc.run.result.outputs[0]
        }
        if ($i -gt 0) { $directEcho += $child.ms; $echo += $run.ms; $peaks += $run.peak_bytes }
    }
    $captureFiles = @(Get-ChildItem -LiteralPath (Join-Path $root 'content\evidence\captures') -File)
    $payloadFiles = @($captureFiles | Where-Object Length -eq $inputBytes)
    if ($payloadFiles.Count -ne 1) { throw 'identical inputs did not deduplicate to one captured payload' }
    [IO.File]::WriteAllBytes((Join-Path $root 'input.bin'), [byte[]]@(1, 2, 3))
    $replay = (Invoke-Cli @('run', 'start', $workflowCapture, '--input', "data=$capture", '--grant-exec', '-o', 'json')).stdout | ConvertFrom-Json
    if ($replay.run.state -ne 'succeeded') { throw 'capture replay failed' }
    $original = (Invoke-Cli @('artifact', 'show', $originalOutput, '-o', 'json')).stdout | ConvertFrom-Json
    $reproduced = (Invoke-Cli @('artifact', 'show', $replay.run.result.outputs[0], '-o', 'json')).stdout | ConvertFrom-Json
    if ($original.artifact.content.path -ne $reproduced.artifact.content.path) { throw 'replay output digest differs' }
    $outputPath = Join-Path $root $original.artifact.content.path
    if ((Get-Item -LiteralPath $outputPath).Length -ne $inputBytes + 4) { throw 'wrong echo byte count' }
    if ((Get-FileHash -LiteralPath $outputPath).Hash -ne (Get-FileHash -LiteralPath (Join-Path $root 'direct-output.bin')).Hash) { throw 'output differs from direct child output' }

    Add-Workflow 'flood' @('flood', '1048577', 'stderr')
    $flood = (Invoke-Cli @('run', 'start', 'flood', '--grant-exec', '-o', 'json')).stdout | ConvertFrom-Json
    if ($flood.run.state -ne 'succeeded' -or $flood.run.streams.stdout.Count -ne 1MB -or
        $flood.run.streams.stderr.Count -ne 1MB -or -not $flood.run.result.truncated.stdout -or
        -not $flood.run.result.truncated.stderr) { throw 'default per-stream cap was not enforced' }
    Add-Workflow 'oversize' @('big-output', 'outputs/result', '16777217') "[outputs]`nresult = { type = `"dataset`", format = `"binary`" }`n"
    $oversize = (Invoke-Cli @('run', 'start', 'oversize', '--grant-exec', '-o', 'json') 8).stdout | ConvertFrom-Json
    if ($oversize.run.state -ne 'failed' -or $oversize.run.result.outputs.Count -ne 0) { throw 'oversized output was published' }
    Add-Workflow 'timeout' @('sleep', '60000')
    $timeout = Invoke-Cli @('run', 'start', 'timeout', '--timeout', '1', '--grant-exec', '-o', 'json') 8
    if (($timeout.stdout | ConvertFrom-Json).run.state -ne 'timed_out') { throw 'timeout was not enforced' }

    $evidence = @(Get-ChildItem -LiteralPath (Join-Path $root 'content\evidence') -File -Recurse)
    $captureFiles = @(Get-ChildItem -LiteralPath (Join-Path $root 'content\evidence\captures') -File)
    $result = [ordered]@{
        measured_utc = [DateTime]::UtcNow.ToString('o')
        os = [Runtime.InteropServices.RuntimeInformation]::OSDescription
        architecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
        powershell = $PSVersionTable.PSVersion.ToString()
        rust = (rustc --version)
        profile = 'release'
        samples_after_warmup = $Runs
        policy = 'informational; direct child duration reported separately; no latency threshold'
        noop = @{
            direct_child_median_ms = Median $direct
            end_to_end_median_ms = Median $endToEnd
            overhead_estimate_ms = [math]::Round((Median $endToEnd) - (Median $direct), 2)
        }
        echo = @{
            input_bytes = $inputBytes
            output_bytes = $inputBytes + 4
            direct_child_median_ms = Median $directEcho
            end_to_end_median_ms = Median $echo
            overhead_estimate_ms = [math]::Round((Median $echo) - (Median $directEcho), 2)
        }
        peak_cli_working_set_bytes = ($peaks | Measure-Object -Maximum).Maximum
        storage = @{
            captured_payload_files = $payloadFiles.Count
            captured_payload_bytes = ($payloadFiles | Measure-Object Length -Sum).Sum
            all_capture_bytes = ($captureFiles | Measure-Object Length -Sum).Sum
            all_evidence_bytes = ($evidence | Measure-Object Length -Sum).Sum
            all_evidence_files = $evidence.Count
            deterministic_replay_and_deduplication = 'passed'
        }
        limits = @{
            streams = '1048577 bytes per stream produced; 1048576 retained, both marked truncated'
            output = '16777217 byte output refused; no outputs published'
            timeout = '60 second child interrupted at a 1 second limit'
            timeout_end_to_end_ms = [math]::Round($timeout.ms, 2)
            boundary_suite = 'cargo test -p functor-runtime --test provider (exact and over item/total caps, timeout, streams, cancellation and tree cleanup)'
        }
    }
    $json = $result | ConvertTo-Json -Depth 6
    Write-Output $json
    if ($Report) {
        [IO.File]::WriteAllText($Report, $json + "`n")
        Write-Output "report written: $Report"
    }
}
finally {
    Remove-Item -LiteralPath $root -Recurse -Force
}
