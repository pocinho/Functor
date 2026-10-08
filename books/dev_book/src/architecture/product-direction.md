# Product Direction: Workspace, Notebooks, and Agents

## Product focus

Functor should build on Monaco rather than recreate editor fundamentals.
Monaco provides editing, selection, undo/redo, keyboard editing, and
editor-local find/replace. Functor's distinctive value is the MVU-based
workspace around those editors: a friendly, folder-rooted place to organize
project files and, later, compose them into notebooks and agent-assisted
workflows.

The original F# application informed the initial migration, archived in the
[completed migration baseline](../milestone/migration-and-parity.md), but it
does not define an ongoing feature-parity gate. Functor's current product
direction and independently scoped work are authoritative. The current
usability target is Windows desktop; macOS and Linux support are deferred and
must not be inferred from the shared Tauri architecture.

## Ownership and state flow

- Rust and `functor_core` own stable workspace/document identity, validated
  state transitions, and canonical document state.
- The Tauri host owns native folder/file dialogs and filesystem discovery.
- `functor_web` is the React application loaded in Tauri's WebView. It owns
  application-level workspace/notebook presentation and composes Monaco
  editor instances.
- `functor_ui` is Functor's canonical React widget library, providing shared
  reusable components to the frontend.
- Typed bridge requests expose narrow workspace operations; the WebView never
  receives unrestricted filesystem access.

Workspace interactions follow the MVU direction: UI intent maps to a typed
semantic message, the pure update changes model state and declares effects,
and the host performs those effects before validated results return through
the update path. See the [system architecture](system.md) and
[document workflow](document-workflow.md).

## Folder-based workspace

Treat an opened folder as a project root, familiar to users of VS Code. The
starter workspace provides:

1. Opening a folder through a native Tauri dialog.
2. Showing the selected root and its file/folder tree.
3. Loading nested directories on demand and opening files from the tree in
   Monaco.
4. Keeping workspace data and loaded tree entries in Rust-owned MVU state,
   with typed bridge operations for listing a directory and opening a file.
5. Clear loading, empty, cancellation, and error feedback.

The initial explorer omits symbolic links and loads nested directories lazily.
This bounds traversal and avoids symlink cycles or escapes; explicit ignore
rules and larger-project performance work remain future work.

The Windows implementation includes native folder and file dialogs, the lazy
tree, open/save flows, multiple document tabs, and a Save/Discard/Cancel
decision when closing a dirty tab. Versioned settings and user themes are
persisted. Startup restores the last workspace and
its saved-file tabs in order, then reopens those files from disk and selects
the previously active file. Dirty buffers are autosaved as versioned recovery
snapshots under the user's Functor application-data directory, separate from
project files. Startup restores those snapshots automatically after the saved
workspace tabs; recovery never silently writes into the source file. Snapshots are
limited to 16 MiB of text per document, 100 documents, and 256 MiB total; they
are written after 500 ms idle or at most every 2 seconds during continuous
typing. Errors are surfaced. A process failure before a pending write
completes can still lose edits since the previous snapshot. Missing or
inaccessible workspace files are reported rather than silently omitted.
Opening a different workspace requires saving all dirty documents; cancelling
the save decision leaves the current workspace open. After successful folder
discovery, current document tabs and recent-close history are cleared so files
from the previous workspace are not mixed into the new one. Cancelling folder
selection or failing discovery leaves the current workspace and tabs intact.
Close Folder saves dirty documents before clearing the workspace and its
persisted location. The startup and empty-editor state presents Open Folder
and Open File actions rather than an empty untitled document.
The in-session recent-close list retains up to ten clean saved files and
reopens from current disk contents. Incomplete F# workspace and diagnostics
work is not a Functor migration blocker.

## Editor behavior boundary

Do not build a second editor for features Monaco already owns. Use Monaco's
editor-local find/replace and editing commands where appropriate. Monaco's
find widget is not a project-wide search service: searching unopened files,
workspace-level replacement, and project indexing are distinct Functor
workspace workflows. Functor automatically searches all workspace text files,
including hidden and ignored paths, or all open documents when no workspace is
open. It searches open-buffer text, returns all matches, shows bounded
previews, and activates selected results. Non-text or unreadable files are
reported as unsearchable. Replace All updates unopened files directly through
the host without opening tabs; edits in already-open buffers remain unsaved
until Save. Search history, regex, multiline, fuzzy search, and project
indexing are not current requirements; do not duplicate editor-local search
UI without a demonstrated need.

Monaco integration still needs validation for focus, clipboard, IME, keyboard
routing, accessibility, and representative Unicode text; using Monaco does
not by itself establish those host-level behaviors.

## Notebook direction

A notebook should be a workspace document composed of ordered cells with
stable identities and explicit cell kinds, initially code and Markdown.
Cells can reuse Monaco for editing; notebook structure, active-cell state, cell
operations, and future outputs belong to typed MVU state rather than being
inferred from DOM layout. Keep cell content and file/workspace identity
separate from editor widget instances so cells can be reordered, restored, and
addressed reliably.

Build notebooks on the folder workspace, not a parallel project or file
system. Decide the notebook persistence format and execution semantics before
shipping notebook files; neither is implied by adding the explorer.

## Future agent workflow

The shell exposes a workspace-level Agent panel toggle that remains independent
of the active document; the panel is currently a placeholder.
The proposed conversation and workspace-context model is recorded in the
[Agent workflow note](../../../../notes/agent-workflow.md).
Agents such as Copilot may later participate in workflows over project files
and notebook cells. Keep integrations behind explicit, permissioned commands
with stable document/cell identities, revision checks, cancellation, and
surfaced failures. Agent output must enter the same validated update path as
user actions. Agents must not receive unrestricted filesystem access or
mutate Monaco models behind Rust's state.

Agent integration is a future extension point, not a requirement for the
workspace starter milestone.

## Starter non-goals

- Reimplementing Monaco editing, editor-local find/replace, or
  language-editor behavior.
- Persistent search history and project indexing.
- Automatically writing ordinary unsaved buffer edits into the user's project
  files; explicit workspace Replace All is the documented exception.
- Notebook execution, output rendering, or a stable notebook file format.
- Agent connections, plugin discovery, or an extension marketplace.
- Recursive eager scanning of every project directory.
