# Project Identity and Status

## Name

The product and its documentation are named **Functor**. The active Rust
package and application directory are both `functor`.

## Default implementation direction

The accepted default architecture is:

- Tauri 2 as the desktop host and native capability boundary.
- A trusted React and TypeScript HTML/CSS application in `functor_web/`,
  loaded in Tauri's WebView and composed with the canonical reusable React
  widget library in `functor_ui/`. This is not a separate browser target.
- Monaco Editor 0.57.0 as the default source-code editor, rather than a
  Functor-built editor widget.
- `functor_core/` as the pure Rust MVU/model crate; `functor/` remains the
  Tauri host, typed bridge, session, and native-effects boundary.
- Rust as the owner of canonical document state, editing invariants, and
  filesystem operations.

All new editor and workspace feature work should target this architecture.
The original F# application informed the initial migration but is not a
permanent feature-parity gate; the Winit/WGPU implementation is a historical
reference only, not a transition baseline or a second long-term UI direction.
See the
[completed migration baseline](../milestone/migration-and-parity.md) and the
[workspace-first product direction](product-direction.md).

## Implementation status

The active Tauri package currently proves a limited multi-document editor
flow: typed command DTOs, revisioned Monaco edit transactions, document
switching, native file dialogs, and Rust-owned file/model state. It now also
has a starter folder-rooted workspace explorer: the host opens a folder,
`functor_core` owns its lazily loaded tree through MVU updates, and files from
the tree open in Monaco. The Windows target supports multiple tabs,
revision-checked dirty-tab close decisions, automatic workspace-wide or
all-open-document literal search with match-case and whole-word options,
cancellation, revision-bound results, match navigation, and Replace All that
updates unopened workspace files without opening tabs while keeping open-buffer
edits unsaved, and startup restoration of saved-file tabs and the previously
active file.
Opening another workspace requires saving dirty documents and closes existing
tabs only after successful folder discovery; cancellation or discovery failure
keeps the current workspace intact. Close Folder also saves dirty documents,
clears the persisted workspace location and its recovery snapshots, and returns
the shell to its welcome panel. Startup and the empty-editor state offer Open
Folder and Open File instead of showing the pristine untitled buffer.
Workspace session state stores paths, while dirty text is autosaved as versioned
local recovery snapshots and restored automatically at startup.
Ordinary buffer edits reach project files
through Save; explicit workspace Replace All updates matching unopened text
files directly through the host. Missing or changed source files are surfaced
during recovery. Versioned settings and user theme files persist in the
application configuration directory; malformed or unsupported settings are
surfaced rather than silently replaced. The initial migration baseline is
complete for the agreed Functor direction; subsequent usability and feature
work is scoped independently. Monaco supplies per-model
lexical tokenization for the complete Monaco 0.57.0 basic-language catalogue;
file extensions select modes automatically, and an accessible language-mode
selector can override them per document. JSON uses a local Monarch tokenizer
without enabling JSON validation services. Monaco exposes diagnostic markers,
but Functor has not yet connected general validation providers. Language
services and servers remain future plugin work. Windows is the current
usability target;
macOS/Linux validation is deferred. The pure model and update logic live in
`functor_core/`; `functor_web/` is the React frontend loaded in Tauri's
WebView and uses the canonical shared widgets from `functor_ui/`. The
application does not yet implement every menu or complete accessibility and
cross-platform validation.

The architectural decision is accepted. Cross-platform release readiness is
separate from the current feature roadmap. Read the
[transition milestone](../milestone/tauri-transition.md), the
[completed migration baseline](../milestone/migration-and-parity.md), and the
[frontend/core decision](../decision/0002-core-and-react-frontend.md) before
treating a capability as shipped.
