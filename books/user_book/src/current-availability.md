# Current Availability

The Tauri transition build currently provides a basic editor workspace:

- multiple open document tabs, including initial Rust and Markdown examples;
- new, open, edit, switch, and save workflows;
- Monaco editing, per-document selection, and undo;
- a workspace file panel, with right-click open/expand actions;
- a File > Exit command and an editor context-menu placeholder for Send to Agent;
- native file dialogs, with Rust performing the file operations.

The application is still being migrated. Workspace navigation, diagnostics,
full command/menu behavior, unsaved-close prompts, packaging, and several
accessibility and text-input checks remain incomplete. File > Exit currently
closes immediately without a dirty-document prompt, and the editor's Send to
Agent context-menu entry is a disabled placeholder. The archived Winit
application may still contain historical “Functors” names; the active Tauri
application is labeled “Functor”.

Do not treat the transition build as a stable release. The developer book
records implementation status and remaining acceptance work.
