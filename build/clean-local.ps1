[CmdletBinding(SupportsShouldProcess = $true)]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$generatedPaths = @(
    "out",
    "target",
    "src\target",
    "src\out",
    "src\functor\node_modules",
    "src\functor_web\node_modules",
    "src\functor_ui\node_modules",
    "src\functor\frontend\dist",
    "src\functor_web\dist",
    "books\dev_book\book",
    "books\user_book\book"
)

foreach ($relativePath in $generatedPaths) {
    $path = Join-Path $repoRoot $relativePath
    if ((Test-Path -LiteralPath $path) -and $PSCmdlet.ShouldProcess($path, "Remove generated local build files")) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}
