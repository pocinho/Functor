# Development Requirements

## Core Rust workspace

Install Git and the current stable Rust toolchain with `rustfmt` and `clippy`.
The project uses Rust edition 2024 and does not declare a minimum Rust
version. The currently verified Windows toolchain is Rust/Cargo 1.99.0 with
the `x86_64-pc-windows-msvc` target. Cargo dependency versions are recorded
in the workspace manifests and `Cargo.lock`.

The native application needs the platform compiler/runtime and a WGPU-capable
graphics driver:

| Platform | Requirements |
| --- | --- |
| Windows | MSVC Rust target, Visual Studio C++ Build Tools and Windows SDK, Direct3D 12-capable driver. |
| macOS | Xcode Command Line Tools and Metal-capable hardware. |
| Linux | C/C++ toolchain, Winit/Skia development libraries, X11 or Wayland session, Vulkan loader and driver. Package names depend on distribution. |

The Tauri transition and this setup are currently verified on Windows only.
The other desktop targets are documented architecture targets, not a claim of
successful current builds.

Cargo build artifacts default to the repository's `out/cargo/` directory.
The Vite frontend build defaults to `out/functor_web/`; both locations are
ignored generated output.

## Tauri frontend

Node.js and npm are needed for the `src/functor_web` frontend and
`src/functor` Tauri host. `src/functor_web/package.json` requires Node.js
`>=22.12.0`; npm
comes with the official Node.js installer. Use the system installation and
restart VS Code or its terminals after installing so the updated `PATH` is
visible. Do not download temporary Node runtimes for repository validation.

As of 2026-10-05, Node.js 24.21.0 is the latest LTS and Node.js 26.10.0 is the
latest Current release. Prefer the latest LTS for routine development. The
system installation used for validation is Node.js 24.21.0. Check the official
[Node.js release schedule](https://nodejs.org/en/about/previous-releases) for
updated versions.

Tauri's platform requirements also apply:

| Platform | WebView/native requirement |
| --- | --- |
| Windows | Microsoft Edge WebView2 Runtime. |
| macOS | Xcode Command Line Tools; system WebKit framework. |
| Linux | WebKitGTK 4.1 and GTK/native development packages for the distribution. |

See the official [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)
for current platform-specific packages. This transition build uses the
installed WebView and does not generate an installer.

## Common commands

From the repository root:

```text
rustup update stable
rustup component add rustfmt clippy
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
```

Install the locked frontend and Tauri CLI dependencies, then check/build the
React frontend or launch/build the default Tauri implementation:

```text
npm ci --prefix src/functor_web
npm ci --prefix src/functor
npm --prefix src/functor_web run check
npm --prefix src/functor_web run test
npm --prefix src/functor_web run build
npm --prefix src/functor run tauri -- dev
npm --prefix src/functor run tauri -- build
```

The Cargo workspace manifest and lockfile are at the repository root; the
active Rust packages are under `src/functor/` and `src/functor_core/`.
Changes to window behavior, input, or visible geometry require a live desktop
smoke test.

On Windows, the repository also provides PowerShell helpers:

```powershell
.\build\build-release-x64.ps1
.\build\clean-local.ps1 -WhatIf
.\build\clean-local.ps1
```

The release script targets `x86_64-pc-windows-msvc`, uses the system Rust and
Node.js installations, and builds the executable without an installer under
`out/cargo/`. It checks that the Rust target is installed and runs `npm ci`
separately for `src/functor/` and `src/functor_web/` only when their local
Tauri/Vite commands are missing. The cleanup script removes only known local
build outputs, including obsolete pre-move output paths; use `-WhatIf` to
preview its removals.
