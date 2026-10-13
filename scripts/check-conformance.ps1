<#
.SYNOPSIS
  Checks docs/rfc-conformance.md against the code: every row has a valid status, every
  "Implemented" or "Subset" row names at least one test, and every named test exists.
.DESCRIPTION
  A test reference is `prefix:path` in backticks, where the prefix selects the crate(s) to look in:
    model:  repo:  store:  conf:  query:  val:  events:  cli:  acc:   (see the table in the document)
    script: a file in scripts/ (a check that is not a cargo test)
  and the path is `function`, `module::function`, `module::prefix_*` (a wildcard), `module::*`
  (the module exists) or a bare test file name (`cli:validate` is tests/validate.rs).
.EXAMPLE
  pwsh scripts/check-conformance.ps1
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$doc = Join-Path $root 'docs/rfc-conformance.md'

$scopes = @{
    'model'  = @('src/crates/functor-model')
    'repo'   = @('src/crates/functor-repository')
    'store'  = @('src/crates/functor-store')
    'conf'   = @('src/crates/functor-testkit', 'src/crates/functor-store')
    'query'  = @('src/crates/functor-query')
    'val'    = @('src/crates/functor-validation')
    'events' = @('src/crates/functor-events')
    'cli'    = @('src/app/cli')
    'acc'    = @('src/app/cli/tests')
}
$statuses = @('Implemented', 'Subset', 'Deferred', 'Deviation')

$files = @{}
function Get-Sources([string]$scope) {
    if (-not $files.ContainsKey($scope)) {
        $files[$scope] = @(Get-ChildItem (Join-Path $root $scope) -Recurse -Filter *.rs -File |
            Where-Object { $_.FullName -notmatch '[\\/](target|out)[\\/]' } |
            ForEach-Object { [pscustomobject]@{ Path = $_.FullName; Name = $_.BaseName; Text = [System.IO.File]::ReadAllText($_.FullName) } })
    }
    return $files[$scope]
}

function Test-Reference([string]$token) {
    if ($token -match '^script:(.+)$') { return Test-Path (Join-Path $root "scripts/$($Matches[1])") }
    if ($token -notmatch '^([a-z]+):(.+)$') { return $false }
    $prefix = $Matches[1]; $path = $Matches[2]
    if (-not $scopes.ContainsKey($prefix)) { return $false }
    $sources = @($scopes[$prefix] | ForEach-Object { Get-Sources $_ })
    $segments = $path -split '::'
    $name = $segments[-1]
    $module = if ($segments.Count -gt 1) { $segments[-2] } else { $null }

    if ($name -eq '*') {
        if (-not $module) { return $sources.Count -gt 0 }
        return [bool]($sources | Where-Object { $_.Name -eq $module -or $_.Text -match "mod\s+$([regex]::Escape($module))\b" })
    }
    $pattern = '(?m)\bfn\s+' + ([regex]::Escape($name) -replace '\\\*', '\w*') + '\b'
    if ($module) {
        $candidates = @($sources | Where-Object { $_.Name -eq $module })
        if ($candidates.Count -eq 0) { $candidates = @($sources | Where-Object { $_.Text -match "mod\s+$([regex]::Escape($module))\b" }) }
        return [bool]($candidates | Where-Object { $_.Text -match $pattern })
    }
    if ($sources | Where-Object { $_.Text -match $pattern }) { return $true }
    return [bool]($sources | Where-Object { $_.Name -eq $name })   # a whole test file
}

$problems = @()
$rows = 0
foreach ($line in Get-Content $doc) {
    if ($line -notmatch '^\| (C-\d{4}-\d+) \|') { continue }
    $cells = ($line.Trim().Trim('|') -split '(?<!\\)\|') | ForEach-Object { $_.Trim() }
    $id = $cells[0]; $status = $cells[2]; $tests = $cells[-1]
    $rows++
    if ($cells.Count -ne 5) { $problems += "$id : expected 5 columns, found $($cells.Count)"; continue }
    if ($status -notin $statuses) { $problems += "$id : unknown status '$status'"; continue }
    $tokens = @([regex]::Matches($tests, '`([^`]+)`') | ForEach-Object { $_.Groups[1].Value } | Where-Object { $_ -match '^[a-z]+:' })
    if ($status -in 'Implemented', 'Subset' -and $tokens.Count -eq 0) { $problems += "$id : $status row names no test" }
    foreach ($token in $tokens) {
        if (-not (Test-Reference $token)) { $problems += "$id : '$token' does not exist" }
    }
}

if ($rows -eq 0) { throw "no conformance rows found in $doc" }

# Test references in the evidence tables of the other documents must exist too.
foreach ($match in [regex]::Matches(((@('nonfunctional-requirements.md', 'mvp-scope.md') | ForEach-Object { [System.IO.File]::ReadAllText((Join-Path $root "docs/$_")) }) -join "`n"), '`((?:model|repo|store|conf|query|val|events|cli|acc|script):[^`\s]+)`')) {
    if (-not (Test-Reference $match.Groups[1].Value)) { $problems += "evidence tables : '$($match.Groups[1].Value)' does not exist" }
}
if ($problems.Count -gt 0) {
    $problems | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    Write-Error "$($problems.Count) problem(s) in docs/rfc-conformance.md"
    exit 1
}
Write-Output "docs/rfc-conformance.md: $rows rows, every status valid and every named test exists"
