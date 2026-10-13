<#
.SYNOPSIS
  Measures the performance budgets of docs/quality/nonfunctional-requirements.md with a release build.
.DESCRIPTION
  Generates a synthetic repository (artifacts, a DAG of depends_on relationships) directly in the
  canonical file format, then times each budgeted command, taking the median of several runs, and
  samples the peak working set. It reports; it fails only when a median exceeds `-Factor` times
  its stored baseline (default 3), or when `-Enforce` is given and a budget is exceeded.
.EXAMPLE
  pwsh scripts/bench.ps1 -Artifacts 10000
  pwsh scripts/bench.ps1 -Artifacts 10000 -Baseline docs/quality/benchmarks/baseline.json
  pwsh scripts/bench.ps1 -Artifacts 10000 -Baseline docs/quality/benchmarks/baseline.json -UpdateBaseline
#>
param(
    [int]$Artifacts = 10000,
    [int]$Runs = 5,
    [double]$Factor = 3,
    [string]$Baseline,
    [switch]$UpdateBaseline,
    [switch]$Enforce
)
$ErrorActionPreference = 'Stop'
Set-Location (Split-Path -Parent $PSScriptRoot)
if ($Baseline) { $Baseline = [System.IO.Path]::GetFullPath($Baseline, (Get-Location).Path) }

cargo build --release -p functor-cli | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'release build failed' }
$meta = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
$exe = Join-Path $meta.target_directory 'release/functor.exe'
if (-not (Test-Path $exe)) { $exe = Join-Path $meta.target_directory 'release/functor' }

$alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ'
function New-Id([string]$prefix, [long]$n) {
    $chars = New-Object char[] 26
    for ($i = 25; $i -ge 0; $i--) { $chars[$i] = $alphabet[[int]($n % 32)]; $n = [math]::Floor($n / 32) }
    return $prefix + (-join $chars)
}

$root = Join-Path ([System.IO.Path]::GetTempPath()) ("functor-bench-" + [guid]::NewGuid().ToString('N').Substring(0, 8))
New-Item -ItemType Directory -Path $root | Out-Null
Push-Location $root
try {
    & $exe init --name bench | Out-Null
    $stamp = '2026-10-12T18:00:00Z'
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    $t = [Diagnostics.Stopwatch]::StartNew()
    for ($i = 0; $i -lt $Artifacts; $i++) {
        $id = New-Id 'art_' (1000 + $i)
        $text = "id = `"$id`"`nslug = `"item-$i`"`ntype = `"document`"`ntitle = `"Item $i`"`nrevision = 1`ncreated = $stamp`nupdated = $stamp`ntags = [`"bench`"]`n"
        [System.IO.File]::WriteAllText("$root/artifacts/$id.toml", $text, $utf8)
    }
    $rel = 0
    function Add-Rel([int]$from, [int]$to) {
        $script:rel++
        $rid = New-Id 'rel_' (1000 + $script:rel)
        $text = "id = `"$rid`"`ntype = `"depends_on`"`nsource = `"$(New-Id 'art_' (1000 + $from))`"`ntarget = `"$(New-Id 'art_' (1000 + $to))`"`ncreated = $stamp`n"
        [System.IO.File]::WriteAllText("$root/relationships/$rid.toml", $text, $utf8)
    }
    for ($i = 1; $i -lt $Artifacts; $i++) {
        $a = [math]::Floor($i / 2)
        Add-Rel $i $a
        $b = [math]::Floor($i / 3)
        if ($i -ge 3 -and $b -ne $a) { Add-Rel $i $b }
    }
    Write-Output ("generated {0} artifacts and {1} relationships in {2:N1}s" -f $Artifacts, $rel, $t.Elapsed.TotalSeconds)

    $last = $Artifacts - 1
    $mid = [int][math]::Floor($last / 2)
    $scenarios = @(
        @{ name = 'artifact show';          budget = $(if ($Artifacts -le 1000) { 300 } else { 750 }); args = @('artifact', 'show', "item-$mid") ; applies = 10000 },
        @{ name = 'query dependencies';     budget = $(if ($Artifacts -le 1000) { 200 } else { 2000 }); args = @('query', 'dependencies', "item-$last", '--depth', 'all'); applies = 0 },
        @{ name = 'validate';               budget = 3000; args = @('validate'); applies = 10000 }
    )

    function Measure-Command-Peak([string[]]$cmdArgs) {
        $psi = New-Object Diagnostics.ProcessStartInfo
        $psi.FileName = $exe
        foreach ($a in $cmdArgs) { $psi.ArgumentList.Add($a) }
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $psi.WorkingDirectory = $root
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $p = [Diagnostics.Process]::Start($psi)
        $out = $p.StandardOutput.ReadToEndAsync()
        $err = $p.StandardError.ReadToEndAsync()
        $peak = 0L
        while (-not $p.HasExited) {
            try { $p.Refresh(); $peak = [math]::Max($peak, $p.PeakWorkingSet64) } catch { }
            Start-Sleep -Milliseconds 20
        }
        $p.WaitForExit()
        $sw.Stop()
        return [pscustomobject]@{ ms = $sw.Elapsed.TotalMilliseconds; peak = $peak; code = $p.ExitCode }
    }

    # `relation add` changes the repository, so each run uses a fresh pair.
    $results = [ordered]@{}
    $report = @()
    $next = 0
    $all = @($scenarios) + @(@{ name = 'relation add'; budget = $(if ($Artifacts -le 1000) { 300 } else { 1500 }); args = $null; applies = 10000 })
    foreach ($s in $all) {
        $samples = @(); $peaks = @()
        for ($r = 0; $r -lt ($Runs + 1); $r++) {
            $cmdArgs = $s.args
            if ($null -eq $cmdArgs) {
                $next++
                $cmdArgs = @('relation', 'add', "item-$($next)", 'references', "item-$($last - $next)")
            }
            $m = Measure-Command-Peak $cmdArgs
            if ($m.code -ne 0 -and $s.name -ne 'validate') { throw "$($s.name) failed with exit code $($m.code)" }
            if ($r -gt 0) { $samples += $m.ms; $peaks += $m.peak }   # the first run warms the file cache
        }
        $median = ($samples | Sort-Object)[[int][math]::Floor($samples.Count / 2)]
        $peakMb = ($peaks | Measure-Object -Maximum).Maximum / 1MB
        $results[$s.name] = [math]::Round($median, 1)
        $status = if ($median -le $s.budget) { 'within budget' } else { 'OVER BUDGET' }
        $report += [pscustomobject]@{ scenario = $s.name; median_ms = [math]::Round($median); budget_ms = $s.budget; peak_mb = [math]::Round($peakMb); status = $status }
    }
    $report | Format-Table -AutoSize | Out-String | Write-Output

    $failed = $false
    if ($Baseline -and (Test-Path $Baseline) -and -not $UpdateBaseline) {
        $base = Get-Content $Baseline -Raw | ConvertFrom-Json
        $key = "n$Artifacts"
        foreach ($name in $results.Keys) {
            $was = $base.$key.$name
            if ($null -ne $was -and $results[$name] -gt $Factor * $was) {
                Write-Error "$name regressed: $($results[$name]) ms vs baseline $was ms (more than ${Factor}x)" -ErrorAction Continue
                $failed = $true
            }
        }
    }
    if ($UpdateBaseline -and $Baseline) {
        $doc = if (Test-Path $Baseline) { Get-Content $Baseline -Raw | ConvertFrom-Json -AsHashtable } else { @{} }
        $doc["n$Artifacts"] = $results
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Baseline) | Out-Null
        ($doc | ConvertTo-Json -Depth 5) | Set-Content $Baseline -Encoding utf8
        Write-Output "baseline updated: $Baseline"
    }
    if ($Enforce -and ($report | Where-Object status -eq 'OVER BUDGET')) { $failed = $true }
    if ($failed) { exit 1 }
}
finally {
    Pop-Location
    Remove-Item -Recurse -Force $root -ErrorAction SilentlyContinue
}
