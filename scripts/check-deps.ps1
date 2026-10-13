<#
.SYNOPSIS
  Checks the workspace dependency edges against docs/architecture/crate-dependencies.md.
.DESCRIPTION
  Fails when a workspace crate has a normal or build dependency on another workspace crate
  that is not allowed, when functor-testkit is a normal dependency, or when a layer uses an
  external crate it must not use. Update this table and docs/architecture/crate-dependencies.md together.
#>
$ErrorActionPreference = 'Stop'

$allowed = @{
    'functor-model'      = @()
    'functor-core'       = @('functor-model')
    'functor-repository' = @('functor-core', 'functor-model')
    'functor-store'      = @('functor-core', 'functor-model', 'functor-repository')
    'functor-query'      = @('functor-core', 'functor-model')
    'functor-validation' = @('functor-core', 'functor-model')
    'functor-events'     = @('functor-core', 'functor-model', 'functor-repository')
    'functor-testkit'    = @('functor-core', 'functor-model')
    'functor-cli'        = @('functor-model', 'functor-core', 'functor-repository', 'functor-store',
                             'functor-query', 'functor-validation', 'functor-events')
}

$pure = @('serde', 'serde_json', 'toml_edit', 'tokio', 'clap', 'anyhow', 'tracing-subscriber', 'notify', 'fd-lock')
$library = @('clap', 'anyhow', 'tracing-subscriber')
$forbiddenExternal = @{
    'functor-model'      = $pure
    'functor-core'       = $pure
    'functor-query'      = $pure
    'functor-validation' = $pure
    'functor-repository' = $library
    'functor-store'      = $library
    'functor-events'     = $library
    'functor-testkit'    = $library
}

$meta = cargo metadata --format-version 1 --no-deps | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }

$errors = @()
$members = $meta.packages | ForEach-Object { $_.name }

foreach ($name in $members) {
    if (-not $allowed.ContainsKey($name)) {
        $errors += "$name is not in the allowed-edge table; add it here and in docs/architecture/crate-dependencies.md"
    }
}

foreach ($pkg in $meta.packages) {
    $name = $pkg.name
    if (-not $allowed.ContainsKey($name)) { continue }
    foreach ($dep in $pkg.dependencies) {
        $isNormal = ($null -eq $dep.kind) -or ($dep.kind -eq 'build')
        if ($members -contains $dep.name) {
            if ($isNormal -and ($allowed[$name] -notcontains $dep.name)) {
                $errors += "forbidden dependency: $name -> $($dep.name)"
            }
            if ($isNormal -and $dep.name -eq 'functor-testkit') {
                $errors += "functor-testkit must be a dev-dependency only (used by $name)"
            }
        }
        elseif ($isNormal -and $forbiddenExternal.ContainsKey($name) -and ($forbiddenExternal[$name] -contains $dep.name)) {
            $errors += "forbidden external dependency: $name -> $($dep.name)"
        }
    }
}

if ($errors.Count -gt 0) {
    $errors | ForEach-Object { Write-Error $_ -ErrorAction Continue }
    exit 1
}

# Local-first (RFC-0008): nothing that ships may be able to talk to the network.
$network = @('reqwest', 'hyper', 'ureq', 'isahc', 'surf', 'attohttpc', 'curl', 'tonic', 'h2',
             'native-tls', 'openssl', 'rustls')
$shipped = cargo tree --workspace --edges normal --prefix none --format '{p}' 2>$null |
    ForEach-Object { ($_ -split ' ')[0] } | Sort-Object -Unique
if ($LASTEXITCODE -ne 0) { throw 'cargo tree failed' }
$found = @($shipped | Where-Object { $network -contains $_ })
if ($found.Count -gt 0) {
    Write-Error "network-capable crates in the shipped dependency tree: $($found -join ', ')"
    exit 1
}
Write-Output "dependency edges OK ($($members.Count) crates, no network crates)"
