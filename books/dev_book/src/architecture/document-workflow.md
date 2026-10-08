# Document and Editor Workflow

The Tauri boundary uses typed request/response commands. The frontend does not
send raw domain messages or file paths for arbitrary filesystem operations.
Open and Save As use native dialogs owned by the Rust host.

Each open document has a stable identity and a monotonically increasing
content revision. Monaco reports UTF-16 ranges; the bridge validates bounds,
surrogate boundaries, overlap, transaction size, and insertion size before
converting ranges into the model's Unicode-scalar positions. Edits are
applied through the existing pure update path.

The frontend serializes edit and selection commands per document. Rust rejects
stale or replayed edit revisions. Each document has its own Monaco model and
undo history; undo and redo changes are synchronized to Rust, including dirty
state reconciliation when content returns to the saved snapshot.
Selection synchronization is deferred until the current browser event finishes
so a selection emitted before a redo edit cannot reach Rust against stale text.
For a collapsed caret, unmodified horizontal movement, Backspace, and Delete
use Unicode grapheme boundaries from `Intl.Segmenter`; Monaco continues to
handle selection replacement, composition, word shortcuts, and all other
editor-local behavior. The Monaco Find contribution provides editor-local
Ctrl+F/Ctrl+H find and replace without a duplicate Functor search control.

Open and save work runs outside the session lock. Completion messages target
the document or open request that initiated the effect. A save records the
exact text snapshot sent to disk and only marks the current document clean if
it still matches that snapshot when the result returns. Dirty state is derived
from equality with the saved snapshot, so ordinary edits that restore the
saved text are clean even when they were not produced by undo.

New and Open create or activate tabs without replacing other documents, so
they do not prompt to discard an unsaved tab. Closing a dirty tab requires an
explicit Save, Discard, or Cancel choice. Save uses the current path; Save As
always opens the native destination picker and updates the saved path and
canonical identity only after a successful write. A destination already open
in another tab is rejected.

Files are decoded as strict UTF-8; a UTF-8 BOM is ignored on load. CRLF and
bare CR line endings normalize to LF in the document, and saves write UTF-8
without a BOM using LF newlines. Writes go to a unique same-directory
temporary file and replace the destination atomically; a failed replacement
leaves the existing destination intact.

Workspace folder selection and filesystem discovery stay in the host. The
frontend requests lazy directory listings and opens files by workspace-relative
path; Rust validates requests against the active workspace and its loaded
tree. Symbolic links are omitted from the initial explorer. Workspace state
restores the last root, ordered saved-file tabs, and active saved file. Dirty
text is autosaved separately to bounded, versioned recovery snapshots in
Functor's per-user application-data directory; ordinary editor changes reach
project files through explicit Save. Workspace Replace All is a separate
explicit operation that writes matching unopened workspace files directly,
while open-buffer replacements remain unsaved. Recovered buffers restore
automatically at startup after workspace tabs. Autosaves occur 500 ms after typing stops, or at most every
2 seconds during continuous typing. Each document is limited to 16 MiB of
text, with 100 snapshots and 256 MiB total recovery storage. A process failure
before a pending write finishes may lose edits since the previous snapshot.
Recovery copies project text into local application data. Missing or changed
files produce visible restore warnings. Routine recovery save/pending states and
successful recovery notifications stay out of the status area; actionable
recovery failures remain visible. Closing
a dirty tab requires an explicit save, discard, or cancel choice; discard also
removes its recovery snapshot.
Closing a clean saved tab adds it to a bounded, in-session recent-close list;
reopening reads the current file from disk and restores its caret and selection.
Explicitly discarded dirty tabs are not added. Reopening remains available
through the typed bridge, but the current shell has no standalone reopen button;
it can be surfaced in a future File > Recent Documents menu. Files opened outside the active
workspace remain open but are excluded from workspace tab restoration. Replacing
the workspace root first requires saving all dirty documents. The user can
cancel that decision to keep the current workspace open; once the selected
folder is successfully discovered, existing document tabs and recent-close
history are cleared. Cancelling folder selection or failing workspace
discovery leaves the current workspace and tabs intact.
Close Folder is available while a workspace is open. Closing it also requires
saving all dirty documents; after confirmation it clears the active workspace,
saved workspace location, open tabs, and their recovery snapshots. At startup
or whenever only the pristine untitled placeholder remains, the editor shows a
welcome panel with Open Folder and Open File actions instead of the empty
untitled editor. Creating a new document returns to the editor.
Versioned settings and themes are persisted. Workspace-wide literal search
automatically covers workspace text files, including hidden and ignored paths,
and uses current in-memory text for open documents; without a workspace, it
searches all open documents. It returns all matches and reports non-text or
unreadable files as unsearchable. Search history is intentionally not retained.
See the [workspace-first product direction](product-direction.md).

The proof-of-concept limits edit transactions to 256 changes and 1 MiB of
inserted text. These are bridge safety bounds, not public file-size promises.
The model, update logic, syntax types, and pure domain tests live under
`functor_core/`. Tauri command/session code, bridge DTOs, native file
operations, and their integration tests remain under `functor/`; the DTOs
describe the IPC boundary and are not part of the core domain API.
