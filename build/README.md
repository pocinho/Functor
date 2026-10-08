# Local Build Scripts

From the repository root, run these scripts in PowerShell:

```powershell
.\build\build-release-x64.ps1
.\build\clean-local.ps1 -WhatIf
.\build\clean-local.ps1
```

`build-release-x64.ps1` builds the Tauri app in `src/functor/` for
`x86_64-pc-windows-msvc`. It uses the system Rust and Node.js installations,
installs the locked Tauri and web npm dependencies separately with `npm ci`
when their local CLIs are missing, and produces the release executable without
an installer. Cargo artifacts and the Vite frontend are written under the
repository's `out/` directory. The Rust target must already be installed.

`clean-local.ps1` removes only known generated build outputs: `out/`,
Cargo target directories, local npm dependencies, obsolete frontend output,
and generated mdBook output. Use `-WhatIf` to preview. It does not remove
source files, lockfiles, manifests, or user documents.
