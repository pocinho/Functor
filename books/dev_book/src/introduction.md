# Functor Developer Book

This book is the canonical engineering reference for **Functor**. It records
the accepted architecture, its implementation status, development
requirements, and the work needed to complete the transition.

The user-facing guide is maintained separately in `books/user_book/`.

The accepted direction is a Tauri-hosted desktop application with a trusted
HTML/CSS/JavaScript interface and Monaco as its default code editor. Rust
continues to own document state, invariants, and native effects. The current
Winit/WGPU application and several old package/window identifiers remain in
the repository during migration; they are not the forward architecture.

Start with [Project identity and status](architecture/project.md), then read
the [architecture decision](decision/0001-tauri-monaco-default.md).
