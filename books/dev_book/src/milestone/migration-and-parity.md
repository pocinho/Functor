# Tauri Migration Baseline (Complete)

**Status:** Complete for the agreed migration scope. This is an archived
record of the initial F# comparison and the Tauri implementation baseline,
not an ongoing feature-parity checklist. Functor's product direction and
independently scoped feature work supersede incomplete or partial F# roadmap
items.

## Historical source and comparison rules

The initial migration comparison used the implemented product behavior in the
original F# application at `D:\dev\projects\pp\Functor-FSharp` (reviewed at
source revision `b7620bb`). The roadmap, application/domain code, and tests
were evidence for that historical comparison only. They do not require Functor
to reproduce incomplete F# work or constrain its evolving product direction.

Functor's accepted architecture is Tauri 2, React/TypeScript, Monaco, and
Rust-owned document/session invariants. This archive records how the initial
implementation was compared with F#; it is not a release gate. New work is
scoped against Functor's current product direction and its own acceptance
criteria.

## Original F# feature reference

| Capability | Observed F# behavior (historical reference) | Boundary or status |
|---|---|---|
| Editing | Create and edit text; cursor movement; selection and selection replacement; line operations; basic undo/redo and overwrite mode; clipboard; grapheme-safe cursor movement, backspace, and delete. | Match supported editing behavior, not Avalonia-specific input/rendering. Advanced undo/redo work is not included. |
| Documents and files | New/open/save/save-as/close; UTF-8 loading and saving with newline normalization; dirty/saved-state tracking; surfaced file errors; unsaved-change confirmation and cancellation. | Native dialogs and filesystem operations belong behind the Tauri host, not the WebView. |
| Workspaces and tabs | Open a folder; workspace-root/file-tree projections; multiple documents with independent edit, cursor, selection, history, syntax, diagnostics, and view state; tab activation/close; canonical-path duplicate prevention; active-tab fallback; recently closed tab reopening; workspace replacement that saves dirty documents and closes old tabs after successful discovery. | Canceling the save decision, folder picker, or failed discovery leaves the current workspace and tabs intact. |
| Search and replacement | Literal line-local search, case-insensitive by default with a case-sensitive option, active/inactive match highlights, bounded previews, next/previous navigation, open-document and workspace search, unopened-file activation, revision-safe current-document/workspace replacement, and bounded session-only history. | Monaco owns editor-local find/replace in the Tauri UI; do not recreate its local controls or require identical local-match presentation. Functor's workspace search and replacement are separate project workflows that can evolve independently. Regex, multiline, and fuzzy modes are not current requirements for custom project search. |
| Syntax highlighting | Monaco 0.57.0 basic lexical tokenization for every registered language plus JSON; file-extension detection and explicit per-document language selection. | Lexical highlighting only: compiler-backed validation, semantic tokens, and language services are future plugin work, not part of this baseline. |
| Shell and commands | File/workspace/search/settings commands, searchable command palette, editor status information, tabs, and Search/Notebook tool-panel plus Agent-panel shell affordances. | Notebook and Agent panel presence is shell hosting, not a complete notebook, agent, or plugin feature. |
| Settings and themes | Settings UI; Graphite Light/Dark presets; user theme files; persisted, validated editor/UI typography, colors, syntax colors, and editor/shell geometry; versioned theme/settings schema. | Persisted settings must retain validation, schema evolution, defaults, and round-trip behavior. Do not reproduce Avalonia resource mechanics. |
| Rendering and input | Viewport-aware rendering; cursor, selection, line-number gutter, diagnostic geometry; pointer selection, scrolling, keyboard routing, and focus handling. | Implement with the accepted WebView/Monaco architecture rather than porting the F# renderer or Avalonia widgets. |
| Diagnostics | Diagnostic domain state and an internal LSP adapter for lifecycle, versioned publication, cancellation, and stale-result rejection exist. | The F# roadmap still marks end-to-end diagnostics presentation (severity, inline display, gutter markers) in progress. Treat this as a partial source feature, not evidence of completed parity. |

### F# roadmap work not in the completed baseline

Do not make the following automatic migration requirements solely because
they appear in the F# source tree or roadmap:

- Advanced navigation: jump-list UI, symbol index/navigation, go-to-definition,
  or document outline.
- Full diagnostics presentation and broader LSP/server compatibility.
- Multi-cursor editing, block selections, external-file-change detection, and
  crash-recovery workflows beyond the existing local snapshot/reopen behavior.
- Regex, whole-word, multiline, or fuzzy modes in custom Functor project search.
- Skia rendering, Avalonia browser/mobile hosting, browser-specific file flows,
  and platform packaging.
- A supported third-party plugin system or complete Notebook/Agent integration.

These may be proposed as later product work, but require an explicit scope
decision and their own acceptance criteria. In particular, F# internal APIs,
draft plugin designs, and shell placeholders do not imply shipped user-facing
features.

## Tauri implementation

### Foundation

- [x] Select Tauri 2 and Monaco as the default architecture.
- [x] Separate the pure Rust model/update engine into `functor_core/`.
- [x] Establish `functor_web/` and `functor_ui/` as the React/TypeScript
  frontend packages and connect the web build to the Tauri host.
- [x] Establish typed requests, errors, document IDs, and edit revisions.
- [x] Prove two-document edit, switch, open, save, cursor/selection, and undo
  flows through shared Rust model/update behavior.
- [x] Maintain the developer and user books as authoritative references.

The foundation completed the initial document-flow scope. It does not require
Functors' later feature work to match the entire historical F# application.

### 1. Document lifecycle and edit transactions

Basic multi-document editing, switching, open/save, cursor/selection updates,
and undo synchronization are proven by the foundation. Complete and verify the
remaining lifecycle semantics before building more workspace behavior on top:

- [x] Support new/untitled, open, save, save-as, and close flows. New/Open
  preserve other tabs instead of prompting to discard them; closing a dirty
  tab offers Save, Discard, and Cancel. Native cancellation and file failures
  remain visible, and neither replaces the current document content.
- [x] Preserve the F# file contract: strict UTF-8 loading/saving, UTF-8 BOM
  removal, CRLF/CR-to-LF normalization, and dirty state derived from the exact
  saved snapshot.
- [x] Verify document identity and revision checks across edit, selection,
  save, and asynchronous completion paths. Focused tests cover stale
  transactions, edits and tab switching during save, failed saves, and
  out-of-order open completion.
- [x] Preserve per-document editing state and basic undo/redo when switching.
  Each tab retains its own Monaco model and undo history; Rust remains
  authoritative for accepted text, identity, revisions, and dirty state.
- [x] Clamp cursor and selection positions captured during Monaco content
  changes to the updated model, including select-all replacements that shorten
  a document.

**Exit gate — complete:** Rust host/session tests and a mocked-WebView smoke
cover save failures, Save As cancellation/success, and edits during save.
Live Windows Tauri testing with native dialogs verified Open cancellation,
Save As success/cancellation, dirty-close Save/Discard/Cancel, select-all
replacement, and a failed save under an exclusive file lock. The failed save
remained visible, preserved the dirty recovery snapshot, and left the original
file unchanged; explicit discard removed the recovery snapshot.

### 2. Workspace and tab lifecycle

The Windows target has a native folder picker, lazy tree, open-from-tree flow,
tab close confirmation, workspace-wide literal search, startup restoration of
saved-file tabs and active file, and local recovery snapshots for unsaved
buffers. Settings and themes are persisted through a versioned schema.
Missing source files are reported and recovered as untitled buffers. macOS and
Linux validation are deferred. Files opened through the native file picker may
be outside the workspace; they remain ordinary tabs but are not included in
workspace tab restoration. Opening another workspace requires saving all
dirty documents.
After successful folder discovery, existing tabs and recent-close history are
cleared; cancelling the save decision, folder picker, or failing discovery
leaves the current workspace and tabs intact.

- [x] Open a project folder, display its root and lazy file tree, open a tree
  file in Monaco, and restore the workspace root and saved-file tabs on
  startup.
- [x] Maintain canonical document identity and prevent duplicate opens of the
  same file, including concurrent opens that finish out of order. Files outside
  the active workspace are allowed through the native file picker and remain
  open, but only workspace-relative files are restored as workspace tabs.
- [x] Activate tabs and close them with revision-checked Save/Discard/Cancel
  behavior and deterministic active-tab fallback.
- [x] Support reopening recently closed saved-file tabs. The in-session
  history is bounded to ten files, restores caret/selection, reloads the
  current disk contents, and retains an entry if reopening fails. Dirty
  documents explicitly discarded are not added to history.
- [x] Preserve independent document state across tab switches: text, dirty
  state, cursor, selection, undo history, syntax, and Monaco view/scroll state.
- [x] Require saving all dirty documents before workspace replacement; allow
  cancellation to keep the current workspace. Close all existing tabs and
  clear recent-close history only after successful folder discovery. Native
  folder cancellation and discovery failures leave the previous root and tabs
  intact.
- [x] Verify loaded-tree membership and workspace-relative paths for explorer
  opens; bind asynchronous completion to the active root and canonical file
  identity. Search-result opens are separately constrained to the active root.
- [x] Restore saved-file tab order and active file from versioned workspace
  session state; surface missing-file warnings and keep persisted paths
  workspace-relative.
- [x] Autosave dirty text into bounded, versioned local recovery snapshots;
  offer explicit restore/discard on startup, keep ordinary buffer edits
  separate from project files until Save, and clear snapshots after save or
  explicit discard. Workspace Replace All is an explicit operation that can
  update unopened workspace text files directly.
- [x] Persist validated settings and themes through a versioned schema.

**Exit gate — complete:** focused model/session tests cover duplicate identity,
outside-workspace tabs, lazy-tree membership, stale workspace open completion,
dirty-document switch rejection, successful tab clearing, recent-close retry,
tab restoration, and active-tab fallback. The recent-close command remains
available for a future File > Recent Documents menu; the current shell has no
standalone reopen button. Frontend tests cover the Save All/Cancel
workspace-switch prompt. A live Windows Tauri smoke verified the
save-before-switch flow, cancellation, and that a successful workspace switch
leaves no tabs from the previous workspace.

### 3. Editor behavior and host input

Monaco owns editor-local editing UI and rendering. Migration means equivalent
supported behavior in the accepted architecture, not a second custom editor.

- [x] Verify selection replacement through the Monaco-to-Rust edit contract.
- [x] Confirm line operations and overwrite mode are not exposed by the F#
  host command/keymap surface; no additional editor controls are required.
- [x] Verify per-document undo/redo synchronization and dirty-state
  reconciliation through the Tauri WebView.
- [x] Restore Monaco keyboard focus after opening or activating a document
  from the shell.
- [x] Verify native clipboard copy/paste through the Tauri WebView.
- [x] Verify pointer selection, scrolling, and caret/selection
  synchronization through the Tauri WebView.
- [x] Verify grapheme-safe cursor movement, backspace, and delete for
  surrogate-pair emoji and combining sequences. Rust tests cover UTF-16
  conversion and surrogate boundaries.
- [x] Verify Monaco's editor-local find/replace and keyboard behavior without
  adding duplicate Functor-local editor controls.

**Exit gate:** focused bridge and frontend tests cover editing/coordinate
contracts, with a live Tauri smoke covering focus, clipboard, scrolling, and
representative Unicode input. Focus restoration, selection replacement,
undo/redo, pointer selection, scrolling, grapheme-aware Unicode editing, and
editor-local find/replace have passed a live Windows WebView smoke. Native
clipboard copy/paste passed user testing. Do not treat a mocked-IPC browser test
as verification of native WebView behavior.

### 4. Shell, commands, settings, and themes

- [x] Migrate the implemented command catalog and searchable command palette,
  including file, workspace, search, and settings entry points.
- [x] Complete tab and editor status presentation for active document,
  language/file type, cursor position, and dirty state.
- [x] Reproduce the shell affordances that are implemented in F#: Search and
  Notebook left tool panels plus the document-specific Agent panel. These are
  panel-hosting behaviors only; they do not imply notebook execution or agent
  integration.
- [x] Implement the settings UI and validated settings draft/apply flow.
- [x] Support Graphite Light/Dark presets and user theme files; persist the
  implemented typography, UI/editor/syntax colors, and shell/editor geometry
  through a versioned schema with defaults and round-trip tests.

**Exit gate — complete:** command discovery and shell controls are reachable
by keyboard and UI; settings validation, defaults, preset selection,
persistence, schema loading, and theme round trips have automated coverage.

### 5. Lexical syntax highlighting

- [x] Preserve lexical highlighting for F#, C#, JSON, and Markdown, including
  language selection by document type and the supported token categories.
- [x] Expose every basic language registered by Monaco 0.57.0 through
  extension-based detection and the per-document language-mode selector.
- [x] Keep language-server providers and JSON validation services out of this
  lexical baseline; add them later through the plugin/language-services work.
- [x] Preserve incremental range updates, multiline lexer state, cancellation,
  and rejection of token results for stale document identities or revisions.
- [x] Test token offsets and edited ranges with Unicode text and line
  insertion/deletion.
- [x] Keep the contract lexical only. Compiler validation, semantic tokens,
  FSharp.Compiler.Service, and broader language-server support are not parity
  requirements.

**Exit gate — complete:** focused token tests and a UI smoke cover supported
languages after edits. Monaco owns the token state per model and invalidates
affected lines after edits; closed-document models are disposed. The full
Monaco 0.57.0 basic-language registry is checked against Rust's validated
language catalogue. There is no asynchronous token-result bridge, so token
results cannot cross document identities or revisions and require no parallel
cancellation protocol.

### 6. Open-document and workspace search

Monaco remains responsible for editor-local find/replace. Functor's workspace
search is a separate project workflow; it has automatic scope selection,
all-result search, and Replace All:

- [x] Search every workspace text file for literal text with case-sensitive
  and whole-word options. Automatically search every workspace file when a
  workspace is open, using open-buffer text; otherwise search all open
  documents. Return all matches, show bounded previews, and activate a result
  in an unopened file. Non-text or unreadable files are reported.
- [x] Add active/inactive match presentation and next/previous navigation.
- [x] Resolve search scope automatically; cancellation remains available while
  searching.
- [x] Keep search results bound to document identity and revision; reject
  stale document results and workspace results after workspace changes.
- [x] Replace all matching results after revision and matched-text checks.
  Unopened workspace files are revalidated and updated through the host
  without opening editor tabs, following workspace-search replacement behavior
  in VS Code. Open-buffer replacements remain dirty until Save.
- [x] Test empty/no-match/error results and replacement races as well as
  successful searches. Existing coverage includes automatic scope selection,
  case/whole-word matching, cancellation, all-result behavior, no-match
  results, request validation, stale document revisions, and replacement-plan
  races. Closed-file replacement tests cover Unicode ranges, stale-file
  rejection, and avoiding writes when replacement text is unchanged. A
  mocked-bridge browser smoke covers workspace/open-document search,
  active/inactive highlighting, cancellation, navigation, and Replace All
  without opening unopened results as tabs.

Whole-word matching treats Unicode alphanumeric characters and underscore as
word characters. Regex, multiline, and fuzzy search are not current Functor
requirements.

**Exit gate:** passed. Automated Rust/frontend tests and the mocked-bridge
browser smoke cover the listed outcomes, including replacing a workspace file
that was not already loaded in Monaco without opening it as a tab.

### 7. Diagnostics and language services (separate Functor work)

The F# application's partial diagnostics implementation is historical context,
not a migration requirement. Monaco provides marker APIs and editor
presentation for diagnostics; Functor still needs to choose and connect
validation providers or language services for the languages it supports.
That work belongs to the product/plugin roadmap and does not block this
completed migration baseline.

### 8. Independent product and release work

Accessibility, platform coverage, packaging, and performance remain worthwhile
quality work, but they are not F# parity exit gates and do not block current
Functor feature development. Scope them according to product priorities and
release plans.
