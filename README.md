# Functor

**MVU-based workspace to organize notebooks and agent-assisted workflows.**

Functor is a Windows-focused desktop application built around a folder-based
project workspace. The current implementation provides a file explorer,
multiple document tabs, Monaco editing, workspace search, and local recovery
for unsaved changes. Notebooks and agent-assisted workflows are the product
direction, not yet shipped features.

## Architecture and status

The active application uses Tauri 2 for its desktop host, a React interface in
the Tauri WebView, and Monaco as its editor. Rust and `functor_core` own
canonical document and workspace state; native dialogs and filesystem
operations stay behind the host.

Settings and theme persistence, full feature parity, accessibility, packaging,
and release validation remain in progress. Windows is the current usability
target; macOS and Linux validation are deferred.

## Documentation

- [Developer Book](books/dev_book/src/SUMMARY.md) — architecture, decisions,
  development requirements, milestones, and migration work.
- [User Book](books/user_book/src/SUMMARY.md) — current workflows and
  limitations.

## Development

Use the system Node.js installation and follow the
[development requirements](books/dev_book/src/architecture/development-requirements.md).
The default UI can be started from the repository root with:

```text
npm ci --prefix src/functor
npm --prefix src/functor run tauri -- dev
```

## Local build scripts

See [build/README.md](build/README.md) for the Windows PowerShell build and
cleanup scripts. Build the release x64 executable with:

```powershell
.\build\build-release-x64.ps1
```

## License

BSD-2-Clause
