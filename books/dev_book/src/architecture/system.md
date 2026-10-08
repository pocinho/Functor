# System Architecture

## Target dependency direction

```text
OS window and lifecycle
          |
          v
`functor` Tauri host, command/session composition, and native effects
          |
          v
`functor_web` React/TypeScript frontend in the Tauri WebView, using `functor_ui`
          |
          +---- Monaco model, selection, undo stack
          |
          v
typed, validated command DTOs
          |
          v
`functor` session -> `functor_core` semantic message -> pure update -> Model
                                             |
                                             v
                                  Rust-owned document state
                                             |
                              native open/save effects and replies
```

The whole trusted application is rendered by the WebView. Do not mix HTML
controls with the retired WGPU editor surface as the default design.

## Ownership boundaries

- **`functor` host:** window lifecycle, command registration, session state,
  native dialogs/file operations, and validated application settings/theme
  persistence in the application configuration directory.
- **Frontend:** `functor_web` is the React application loaded in the Tauri
  WebView; it maps the typed bridge to application UI state. `functor_ui` is
  Functor's canonical React widget library and supplies shared, reusable
  components. React owns application layout and presentation; Monaco owns
  editor interaction, per-document command ordering, and incremental
  per-model lexical tokenization.
- **Bridge DTOs:** explicit requests, snapshots, and structured errors.
  Never serialize the internal Rust `Message` enum as a generic IPC API.
- **`functor_core`:** stable document identity, text, dirty state, cursor and
  selection positions, and semantic update behavior. It has no Tauri or DOM
  dependency; effects are represented as core commands and replies.
- **Rust host file boundary:** file loading/saving, workspace discovery, and
  external-modification checks. The WebView does not receive unrestricted
  filesystem access.

Keep Tauri and WebView dependencies out of Rust domain logic. Keep Monaco and
DOM dependencies out of the Rust model and reducer.

## Rendering and editor

The Tauri WebView renders the `functor_web` application using the shared
widgets from `functor_ui`. This package split does not establish a standalone
browser deployment target. Monaco is the editor component; Functor should not
recreate basic text editing, undo, caret, selection, or language-editor
behavior in custom widgets. Monaco 0.57.0 is the currently pinned candidate;
evaluate upgrades through the dependency lockfile and regression tests.

The frontend uses Vite, TypeScript, and React. Rust bridge DTOs are represented
as explicit TypeScript projection/request types; they are not a generic
serialization of Rust domain messages. Keep Tauri/WebView APIs out of
`functor_core` and Monaco/DOM APIs out of the Rust model and reducer.

## Workspace and future notebook direction

The folder-rooted project workspace is Functor's organizing surface around
Monaco. Rust MVU state owns workspace identity and the loaded tree; the Tauri
host mediates folder selection, directory discovery, and file access through
narrow typed requests. Notebooks may later compose stable, MVU-owned cells
with Monaco editor instances. Agent workflows are a future extension and
must use permissioned, revision-aware commands rather than bypassing Rust
state. See the [workspace-first product direction](product-direction.md).

## Transitional implementation

The active workspace contains `functor/` (Tauri host) and `functor_core/`
(pure Rust model/update logic). The web target and shared React widgets live
in `functor_web/` and `functor_ui/`.