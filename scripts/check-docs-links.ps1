<#
.SYNOPSIS
  Checks that relative markdown links in docs/ and README.md point to existing files.
#>
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$files = @(Get-ChildItem (Join-Path $root 'docs') -Recurse -Filter *.md) + (Get-Item (Join-Path $root 'README.md'))
$bad = @()
foreach ($f in $files) {
    $text = Get-Content -LiteralPath $f.FullName -Raw
    foreach ($m in [regex]::Matches($text, '\]\(([^)#\s]+)(#[^)]*)?\)')) {
        $link = $m.Groups[1].Value
        if ($link -match '^(https?:|mailto:)') { continue }
        if (-not (Test-Path -LiteralPath (Join-Path $f.DirectoryName $link))) {
            $bad += "$($f.FullName): $link"
        }
    }
}
if ($bad.Count -gt 0) {
    $bad | Sort-Object -Unique | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    exit 1
}
Write-Output "markdown links OK ($($files.Count) files)"
