# Editing Documents

In the current Tauri transition build:

- Select a tab to switch documents. Each tab retains its own editor model and
  undo history.
- Use **New** to create a blank Rust document.
- Use **Open** to choose a UTF-8 text file with the operating-system dialog.
- Edit in Monaco. The Rust application owns the synchronized document state;
  the editor reports changes through revisioned commands.
- Use **Save** to write the current document. An untitled document opens a
  native Save As dialog; an already opened or saved document writes to its
  selected path.
- In the workspace file panel, right-click a file to open it or a folder to
  expand it. Right-clicking the editor shows a disabled **Send to Agent**
  placeholder; the browser's native context menu is suppressed throughout the
  app.
- Use **File > Exit** to close the app immediately. Save dirty documents first;
  Exit does not prompt to save.

External file changes are checked before overwriting a file. If a save or
bridge operation fails, the interface reports the error rather than claiming
the document was saved.

The workspace file panel is a basic browser; diagnostics, a complete agent
integration, and an unsaved-change prompt on application shutdown are not yet
available.
