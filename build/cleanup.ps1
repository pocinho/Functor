### cleanup.ps1 ###

# Root path:
$rootProjectPath = "..\src\"

# File and directory name patterns that are safe to delete:
$filenamePattern = @("bin", "obj", ".vs", "*.user", "*.bak", "*.old")

# Clean project built artifacts and stale files:
Get-ChildItem $rootProjectPath -Recurse -Force -Include $filenamePattern | Remove-Item -Recurse -Force -WhatIf

### cleanup.ps1 ###