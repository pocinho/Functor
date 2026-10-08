use crate::file_io::{FileError, LoadedFile, SavedFile};
use crate::model::{
    DocumentId, EditorState, EditorViewId, Model, OpenRequestId, Position, Selection, TextEdit,
};
use crate::syntax::SyntaxLanguage;
use crate::workspace::{Workspace, WorkspaceError};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Backspace,
    Delete,
    Enter,
    Home,
    End,
    PageUp,
    PageDown,
    Escape,
}

pub const COMMAND_BAR_COMMANDS: [&str; 4] = ["Open File", "Open Workspace", "Save", "Settings"];

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    WindowResized {
        width: u32,
        height: u32,
    },
    ModifiersChanged {
        shift: bool,
    },
    RedrawRequested,
    KeyPressed(Key),
    ActivateEditorView(EditorViewId),
    CloseEditorView(EditorViewId),
    CycleEditorView {
        reverse: bool,
    },
    #[allow(dead_code)]
    NewUntitledDocument {
        text: String,
        language: SyntaxLanguage,
    },
    SetDocumentLanguage {
        document_id: DocumentId,
        language: SyntaxLanguage,
    },
    #[allow(dead_code)]
    ApplyDocumentEdits {
        document_id: DocumentId,
        edits: Vec<TextEdit>,
        cursor: Position,
        selection: Option<Selection>,
        reconcile_dirty: bool,
    },
    #[allow(dead_code)]
    SetEditorSelection {
        document_id: DocumentId,
        cursor: Position,
        selection: Option<Selection>,
    },
    TextInput(String),
    ImePreeditChanged {
        text: String,
        cursor: Option<(usize, usize)>,
    },
    Scrolled {
        vertical: i32,
        horizontal: i32,
    },
    ToggleCommandBar,
    OpenSettings,
    PointerPressed {
        position: Position,
    },
    PointerDragged {
        position: Position,
    },
    CloseRequested,
    OpenWorkspacePickerRequested,
    OpenWorkspaceRequested(PathBuf),
    WorkspaceOpened(Result<Workspace, WorkspaceError>),
    CloseWorkspace,
    ReopenClosedDocumentRequested,
    ClearRecentlyClosedDocuments,
    LoadWorkspaceDirectoryRequested {
        root: PathBuf,
        relative_path: PathBuf,
    },
    WorkspaceDirectoryLoaded {
        root: PathBuf,
        relative_path: PathBuf,
        result: Result<Vec<crate::workspace::WorkspaceEntry>, WorkspaceError>,
    },
    OpenFilePickerRequested,
    OpenFileRequested(PathBuf),
    FileOpened {
        request_id: OpenRequestId,
        result: Result<LoadedFile, FileError>,
    },
    SaveFileRequested,
    #[allow(dead_code)]
    SaveDocumentRequested {
        document_id: DocumentId,
        path: Option<PathBuf>,
    },
    FileSaved {
        document_id: DocumentId,
        saved_text: String,
        result: Result<SavedFile, FileError>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    RequestRedraw,
    Exit,
    OpenWorkspace(PathBuf),
    ReadWorkspaceDirectory {
        root: PathBuf,
        relative_path: PathBuf,
    },
    OpenFile {
        request_id: OpenRequestId,
        path: PathBuf,
    },
    SaveFile {
        document_id: DocumentId,
        path: PathBuf,
        text: String,
        expected_stamp: Option<crate::file_io::FileStamp>,
    },
    OpenWorkspacePicker,
    OpenFilePicker,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Transition {
    pub model: Model,
    pub commands: Vec<Command>,
}

pub fn update(mut model: Model, message: Message) -> Transition {
    let mut commands = Vec::new();

    match message {
        Message::WindowResized { width, height } => {
            model.active_editor.viewport.width = width;
            model.active_editor.viewport.height = height;
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::ModifiersChanged { shift } => model.shift_down = shift,
        Message::ActivateEditorView(view_id) => {
            if model.activate_view(view_id) {
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::CloseEditorView(view_id) => {
            if model.close_view(view_id) {
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::CycleEditorView { reverse } => {
            if model.activate_next_view(reverse) {
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::NewUntitledDocument { text, language } => {
            model.create_untitled_document(&text, language, true);
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::SetDocumentLanguage {
            document_id,
            language,
        } => {
            if let Some(editor) = model.editor_for_document_mut(document_id) {
                editor.document.language = language;
                editor.document.language_overridden = true;
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::ApplyDocumentEdits {
            document_id,
            mut edits,
            cursor,
            selection,
            reconcile_dirty,
        } => {
            edits.sort_by(|left, right| {
                right
                    .start
                    .cmp(&left.start)
                    .then_with(|| right.end.cmp(&left.end))
            });
            if let Some(editor) = model.editor_for_document_mut(document_id) {
                for edit in edits {
                    editor.document.delete_range(&edit.start, &edit.end);
                    let mut position = edit.start;
                    editor.document.insert_text(&mut position, &edit.text);
                }
                editor.cursor = editor.document.clamp_position(cursor);
                editor.selection = selection.map(|selection| Selection {
                    anchor: editor.document.clamp_position(selection.anchor),
                    focus: editor.document.clamp_position(selection.focus),
                });
                if reconcile_dirty {
                    editor.document.reconcile_dirty();
                }
                editor.ime_preedit = None;
                editor.state = if editor.file_path.is_some() {
                    EditorState::Active
                } else {
                    EditorState::Empty
                };
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::SetEditorSelection {
            document_id,
            cursor,
            selection,
        } => {
            if let Some(editor) = model.editor_for_document_mut(document_id) {
                editor.cursor = editor.document.clamp_position(cursor);
                editor.selection = selection.map(|selection| Selection {
                    anchor: editor.document.clamp_position(selection.anchor),
                    focus: editor.document.clamp_position(selection.focus),
                });
            }
        }
        Message::RedrawRequested => model.needs_redraw = false,
        Message::TextInput(text) => {
            model.active_editor.ime_preedit = None;
            if model.command_bar_open {
                model.command_bar_query.push_str(&text);
                model.command_bar_selection = 0;
            } else {
                consume_selection(&mut model);
                let editor = &mut model.active_editor;
                editor.document.insert_text(&mut editor.cursor, &text);
                if !text.is_empty() {
                    model.active_editor.state = if model.active_editor.file_path.is_some() {
                        EditorState::Active
                    } else {
                        EditorState::Empty
                    };
                }
            }
            model.needs_redraw = true;
            if !text.is_empty() {
                commands.push(Command::RequestRedraw);
            }
        }
        Message::ImePreeditChanged { text, cursor } => {
            model.active_editor.ime_preedit =
                (!text.is_empty()).then_some(crate::model::ImePreedit { text, cursor });
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::Scrolled {
            vertical,
            horizontal,
        } => {
            model.active_editor.viewport.vertical_offset =
                offset_by(model.active_editor.viewport.vertical_offset, vertical);
            model.active_editor.viewport.horizontal_offset =
                offset_by(model.active_editor.viewport.horizontal_offset, horizontal);
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::KeyPressed(key) => {
            if model.command_bar_open && key == Key::Backspace {
                model.command_bar_query.pop();
                model.command_bar_selection = 0;
            } else if model.command_bar_open && key == Key::Escape {
                model.command_bar_open = false;
                model.command_bar_query.clear();
                model.command_bar_selection = 0;
            } else if model.command_bar_open && key == Key::Up {
                model.command_bar_selection = model.command_bar_selection.saturating_sub(1);
            } else if model.command_bar_open && key == Key::Down {
                model.command_bar_selection = (model.command_bar_selection + 1)
                    .min(COMMAND_BAR_COMMANDS.len().saturating_sub(1));
            } else if model.command_bar_open && key == Key::Enter {
                submit_command_bar(&mut model, &mut commands);
            } else if !model.command_bar_open {
                apply_key(&mut model, key);
                model.active_editor.state = if model.active_editor.file_path.is_some() {
                    EditorState::Active
                } else {
                    EditorState::Empty
                };
            }
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::ToggleCommandBar => {
            model.command_bar_open = !model.command_bar_open;
            if model.command_bar_open {
                model.settings_open = false;
            }
            model.command_bar_query.clear();
            model.command_bar_selection = 0;
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::OpenSettings => {
            model.command_bar_open = false;
            model.command_bar_query.clear();
            model.command_bar_selection = 0;
            model.settings_open = true;
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::PointerPressed { position } => {
            model.active_editor.ime_preedit = None;
            let position = model.active_editor.document.clamp_position(position);
            model.active_editor.cursor = position.clone();
            model.active_editor.selection = Some(Selection {
                anchor: position.clone(),
                focus: position,
            });
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::PointerDragged { position } => {
            model.active_editor.ime_preedit = None;
            let position = model.active_editor.document.clamp_position(position);
            model.active_editor.cursor = position.clone();
            if let Some(selection) = &mut model.active_editor.selection {
                selection.focus = position;
            }
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::CloseRequested => commands.push(Command::Exit),
        Message::OpenWorkspacePickerRequested => {
            commands.push(Command::OpenWorkspacePicker);
        }
        Message::OpenFilePickerRequested => {}
        Message::OpenWorkspaceRequested(path) => {
            commands.push(Command::OpenWorkspace(path));
        }
        Message::WorkspaceOpened(result) => {
            match result {
                Ok(workspace) => {
                    model.reset_documents();
                    model.workspace = Some(workspace);
                    model.workspace_error = None;
                }
                Err(error) => {
                    model.workspace_error = Some(error);
                }
            }
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::CloseWorkspace => {
            model.reset_documents();
            model.workspace = None;
            model.workspace_error = None;
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::ReopenClosedDocumentRequested => {
            if let Some(path) = model.most_recently_closed_path().map(PathBuf::from) {
                let request_id = model.begin_open_request();
                commands.push(Command::OpenFile { request_id, path });
            }
        }
        Message::ClearRecentlyClosedDocuments => {
            model.recently_closed.clear();
            model.needs_redraw = true;
            commands.push(Command::RequestRedraw);
        }
        Message::LoadWorkspaceDirectoryRequested {
            root,
            relative_path,
        } => {
            if model.workspace.as_ref().is_some_and(|workspace| {
                workspace.root == root && workspace.contains_directory(&root.join(&relative_path))
            }) {
                commands.push(Command::ReadWorkspaceDirectory {
                    root,
                    relative_path,
                });
            }
        }
        Message::WorkspaceDirectoryLoaded {
            root,
            relative_path,
            result,
        } => {
            if model
                .workspace
                .as_ref()
                .is_some_and(|workspace| workspace.root == root)
            {
                let mut update_error = None;
                if let Some(workspace) = model.workspace.as_mut() {
                    match result {
                        Ok(entries) => {
                            let directory = root.join(relative_path);
                            if !workspace.set_directory_entries(&directory, entries) {
                                update_error = Some(WorkspaceError::InvalidPath(directory));
                            }
                        }
                        Err(error) => update_error = Some(error),
                    }
                }
                model.workspace_error = update_error;
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::OpenFileRequested(path) => {
            if let Some(view_id) = model.view_for_path(&path) {
                model.activate_view(view_id);
                model.open_file_error = None;
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            } else {
                let request_id = model.begin_open_request();
                commands.push(Command::OpenFile { request_id, path });
            }
        }
        Message::FileOpened { request_id, result } => {
            if let Some(activate) = model.complete_open_request(request_id) {
                match result {
                    Ok(file) => {
                        model.open_loaded_file(file, activate);
                        if model
                            .open_file_error
                            .as_ref()
                            .is_some_and(|(id, _)| *id <= request_id)
                        {
                            model.open_file_error = None;
                        }
                    }
                    Err(error) => {
                        if model
                            .open_file_error
                            .as_ref()
                            .is_none_or(|(id, _)| request_id >= *id)
                        {
                            model.open_file_error = Some((request_id, error));
                        }
                    }
                }
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
        Message::SaveFileRequested => {
            queue_save_file(&model, &mut commands, model.active_editor.document_id, None);
        }
        Message::SaveDocumentRequested { document_id, path } => {
            queue_save_file(&model, &mut commands, document_id, path);
        }
        Message::FileSaved {
            document_id,
            saved_text,
            result,
        } => {
            if let Some(editor) = model.editor_for_document_mut(document_id) {
                match result {
                    Ok(path) => {
                        editor.document.mark_saved(&saved_text);
                        if !editor.document.language_overridden {
                            editor.document.language =
                                SyntaxLanguage::from_path(&path.identity_path);
                        }
                        editor.file_path = Some(path.path);
                        editor.identity_path = Some(path.identity_path);
                        editor.file_stamp = Some(path.stamp);
                        editor.file_error = None;
                        editor.state = EditorState::Active;
                    }
                    Err(error) => {
                        editor.file_error = Some(error);
                        editor.state = EditorState::Error;
                    }
                }
                model.needs_redraw = true;
                commands.push(Command::RequestRedraw);
            }
        }
    }

    Transition { model, commands }
}

fn queue_save_file(
    model: &Model,
    commands: &mut Vec<Command>,
    document_id: DocumentId,
    path_override: Option<PathBuf>,
) {
    let Some(editor) = model.editor_for_document(document_id) else {
        return;
    };
    let Some(path) = path_override.or_else(|| editor.file_path.clone()) else {
        return;
    };
    let expected_stamp = editor
        .file_path
        .as_ref()
        .filter(|current_path| **current_path == path)
        .and(editor.file_stamp.clone());
    commands.push(Command::SaveFile {
        document_id,
        path,
        text: editor.document.text(),
        expected_stamp,
    });
}

fn submit_command_bar(model: &mut Model, commands: &mut Vec<Command>) {
    let query = model.command_bar_query.trim().to_ascii_lowercase();
    let query = if query.is_empty() {
        COMMAND_BAR_COMMANDS[model.command_bar_selection].to_ascii_lowercase()
    } else {
        query
    };
    model.command_bar_open = false;
    model.command_bar_query.clear();
    model.command_bar_selection = 0;
    match query.as_str() {
        "settings" | "open settings" => model.settings_open = true,
        "open workspace" | "workspace" => commands.push(Command::OpenWorkspacePicker),
        "open file" | "file" => commands.push(Command::OpenFilePicker),
        "save" => {
            if let Some(path) = &model.active_editor.file_path {
                commands.push(Command::SaveFile {
                    document_id: model.active_editor.document_id,
                    path: path.clone(),
                    text: model.active_editor.document.text(),
                    expected_stamp: model.active_editor.file_stamp.clone(),
                });
            }
        }
        _ => {}
    }
    model.needs_redraw = true;
}

fn apply_key(model: &mut Model, key: Key) {
    if model.shift_down && is_selection_key(key) {
        let anchor = model
            .active_editor
            .selection
            .as_ref()
            .map(|selection| selection.anchor.clone())
            .unwrap_or_else(|| model.active_editor.cursor.clone());
        move_cursor(model, key);
        model.active_editor.selection =
            (anchor != model.active_editor.cursor).then_some(Selection {
                anchor,
                focus: model.active_editor.cursor.clone(),
            });
        return;
    }

    let selection_consumed =
        matches!(key, Key::Backspace | Key::Delete | Key::Enter) && consume_selection(model);
    model.active_editor.selection = None;
    match key {
        Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End => {
            move_cursor(model, key)
        }
        Key::Backspace | Key::Delete if selection_consumed => {}
        Key::Backspace => {
            let editor = &mut model.active_editor;
            editor.document.backspace(&mut editor.cursor);
        }
        Key::Delete => {
            let editor = &mut model.active_editor;
            editor.document.delete(&mut editor.cursor);
        }
        Key::Enter => {
            let editor = &mut model.active_editor;
            editor.document.insert_text(&mut editor.cursor, "\n");
        }
        Key::PageUp => {
            let page = (model.active_editor.viewport.height / 20).max(1) as usize;
            model.active_editor.viewport.vertical_offset = model
                .active_editor
                .viewport
                .vertical_offset
                .saturating_sub(page);
        }
        Key::PageDown => {
            let page = (model.active_editor.viewport.height / 20).max(1) as usize;
            model.active_editor.viewport.vertical_offset = model
                .active_editor
                .viewport
                .vertical_offset
                .saturating_add(page);
        }
        Key::Escape => {}
    }
}

fn is_selection_key(key: Key) -> bool {
    matches!(
        key,
        Key::Left | Key::Right | Key::Up | Key::Down | Key::Home | Key::End
    )
}

fn offset_by(offset: usize, delta: i32) -> usize {
    if delta.is_negative() {
        offset.saturating_sub(delta.unsigned_abs() as usize)
    } else {
        offset.saturating_add(delta as usize)
    }
}

fn move_cursor(model: &mut Model, key: Key) {
    match key {
        Key::Left => {
            if model.active_editor.cursor.column > 0 {
                model.active_editor.cursor.column -= 1;
            } else if model.active_editor.cursor.line > 0 {
                model.active_editor.cursor.line -= 1;
                model.active_editor.cursor.column = model
                    .active_editor
                    .document
                    .line_len(model.active_editor.cursor.line);
            }
        }
        Key::Right => {
            let line_length = model
                .active_editor
                .document
                .line_len(model.active_editor.cursor.line);
            if model.active_editor.cursor.column < line_length {
                model.active_editor.cursor.column += 1;
            } else if model.active_editor.cursor.line + 1
                < model.active_editor.document.line_count()
            {
                model.active_editor.cursor.line += 1;
                model.active_editor.cursor.column = 0;
            }
        }
        Key::Up => {
            model.active_editor.cursor.line = model.active_editor.cursor.line.saturating_sub(1);
            model.active_editor.cursor = model
                .active_editor
                .document
                .clamp_position(model.active_editor.cursor.clone());
        }
        Key::Down => {
            model.active_editor.cursor.line = (model.active_editor.cursor.line + 1)
                .min(model.active_editor.document.line_count() - 1);
            model.active_editor.cursor = model
                .active_editor
                .document
                .clamp_position(model.active_editor.cursor.clone());
        }
        Key::Home => model.active_editor.cursor.column = 0,
        Key::End => {
            model.active_editor.cursor.column = model
                .active_editor
                .document
                .line_len(model.active_editor.cursor.line)
        }
        _ => {}
    }
}

fn consume_selection(model: &mut Model) -> bool {
    let Some(selection) = model.active_editor.selection.take() else {
        return false;
    };

    let (start, end) = if selection.anchor <= selection.focus {
        (selection.anchor, selection.focus)
    } else {
        (selection.focus, selection.anchor)
    };
    if start == end {
        return false;
    }

    model.active_editor.document.delete_range(&start, &end);
    model.active_editor.cursor = start;
    true
}

#[cfg(test)]
mod tests {
    use super::{Command, Key, Message, update};
    use crate::file_io::{FileError, FileStamp, LoadedFile, SavedFile};
    use crate::model::{Model, Position, Selection};
    use crate::syntax::SyntaxLanguage;
    use crate::workspace::{Workspace, WorkspaceError};
    use std::path::PathBuf;

    #[test]
    fn initial_model_has_one_empty_line_and_requests_a_frame() {
        let model = Model::default();

        assert_eq!(model.active_editor.document.lines(), vec![String::new()]);
        assert_eq!(model.active_editor.cursor, Position::default());
        assert!(model.needs_redraw);
    }

    #[test]
    fn text_input_updates_document_cursor_and_redraw_command() {
        let transition = update(Model::default(), Message::TextInput("hello".into()));

        assert_eq!(
            transition.model.active_editor.document.lines(),
            vec!["hello"]
        );
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 5 }
        );
        assert!(transition.model.active_editor.document.dirty);
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn ime_preedit_is_presentation_state_and_clears_on_commit() {
        let model = update(Model::default(), Message::TextInput("base".into())).model;
        let model = update(
            model,
            Message::ImePreeditChanged {
                text: "😀".into(),
                cursor: Some((1, 1)),
            },
        )
        .model;

        assert_eq!(model.active_editor.document.lines(), vec!["base"]);
        assert_eq!(model.active_editor.cursor, Position { line: 0, column: 4 });
        assert_eq!(
            model.active_editor.ime_preedit,
            Some(crate::model::ImePreedit {
                text: "😀".into(),
                cursor: Some((1, 1)),
            })
        );

        let model = update(
            model,
            Message::ImePreeditChanged {
                text: String::new(),
                cursor: None,
            },
        )
        .model;
        assert_eq!(model.active_editor.ime_preedit, None);

        let model = update(
            update(
                model,
                Message::ImePreeditChanged {
                    text: "x".into(),
                    cursor: Some((1, 1)),
                },
            )
            .model,
            Message::TextInput("é".into()),
        )
        .model;
        assert_eq!(model.active_editor.document.lines(), vec!["baseé"]);
        assert_eq!(model.active_editor.ime_preedit, None);
    }

    #[test]
    fn command_bar_navigates_commands_and_submits_the_selected_item() {
        let model = update(Model::default(), Message::ToggleCommandBar).model;
        let model = update(model, Message::KeyPressed(Key::Down)).model;
        assert_eq!(model.command_bar_selection, 1);

        let transition = update(model, Message::KeyPressed(Key::Enter));

        assert!(!transition.model.command_bar_open);
        assert_eq!(transition.model.command_bar_selection, 0);
        assert_eq!(
            transition.commands,
            vec![Command::OpenWorkspacePicker, Command::RequestRedraw]
        );
    }

    #[test]
    fn command_bar_escape_clears_query_and_selection() {
        let model = update(Model::default(), Message::ToggleCommandBar).model;
        let model = update(model, Message::TextInput("save".into())).model;
        let model = update(model, Message::KeyPressed(Key::Down)).model;

        let transition = update(model, Message::KeyPressed(Key::Escape));

        assert!(!transition.model.command_bar_open);
        assert!(transition.model.command_bar_query.is_empty());
        assert_eq!(transition.model.command_bar_selection, 0);
    }

    #[test]
    fn menu_overlays_are_mutually_exclusive() {
        let settings = update(Model::default(), Message::OpenSettings);
        assert!(settings.model.settings_open);
        assert!(!settings.model.command_bar_open);
        assert_eq!(settings.commands, vec![Command::RequestRedraw]);

        let command_bar = update(settings.model, Message::ToggleCommandBar);
        assert!(command_bar.model.command_bar_open);
        assert!(!command_bar.model.settings_open);

        let settings_again = update(command_bar.model, Message::OpenSettings);
        assert!(!settings_again.model.command_bar_open);
        assert!(settings_again.model.settings_open);
    }

    #[test]
    fn enter_and_backspace_preserve_cursor_invariants() {
        let model = update(Model::default(), Message::TextInput("ab".into())).model;
        let model = update(model, Message::KeyPressed(Key::Enter)).model;
        let model = update(model, Message::TextInput("c".into())).model;
        let model = update(model, Message::KeyPressed(Key::Backspace)).model;

        assert_eq!(
            model.active_editor.document.lines(),
            vec![String::from("ab"), String::new()]
        );
        assert_eq!(model.active_editor.cursor, Position { line: 1, column: 0 });
    }

    #[test]
    fn delete_removes_text_at_cursor_and_joins_lines() {
        let mut model = update(Model::default(), Message::TextInput("ab\ncd".into())).model;
        model.active_editor.cursor = Position { line: 0, column: 1 };

        let model = update(model, Message::KeyPressed(Key::Delete)).model;
        assert_eq!(model.active_editor.document.lines(), vec!["a", "cd"]);

        let mut model = model;
        model.active_editor.cursor = Position { line: 0, column: 1 };
        let model = update(model, Message::KeyPressed(Key::Delete)).model;
        assert_eq!(model.active_editor.document.lines(), vec!["acd"]);
    }

    #[test]
    fn movement_keys_clamp_at_line_and_document_edges() {
        let model = update(Model::default(), Message::TextInput("abc\ndef".into())).model;
        let model = update(model, Message::KeyPressed(Key::Up)).model;
        let model = update(model, Message::KeyPressed(Key::Home)).model;
        let model = update(model, Message::KeyPressed(Key::Right)).model;
        let model = update(model, Message::KeyPressed(Key::Down)).model;
        let model = update(model, Message::KeyPressed(Key::End)).model;
        let model = update(model, Message::KeyPressed(Key::Right)).model;

        assert_eq!(model.active_editor.cursor, Position { line: 1, column: 3 });
    }

    #[test]
    fn resize_updates_viewport_without_platform_types() {
        let transition = update(
            Model::default(),
            Message::WindowResized {
                width: 800,
                height: 600,
            },
        );

        assert_eq!(transition.model.active_editor.viewport.width, 800);
        assert_eq!(transition.model.active_editor.viewport.height, 600);
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn workspace_request_declares_an_effect_and_failure_is_retained() {
        let path = PathBuf::from("missing-workspace");
        let transition = update(
            Model::default(),
            Message::OpenWorkspaceRequested(path.clone()),
        );
        assert_eq!(transition.commands, vec![Command::OpenWorkspace(path)]);

        let error = WorkspaceError::InvalidRoot(PathBuf::from("missing-workspace"));
        let transition = update(
            transition.model,
            Message::WorkspaceOpened(Err(error.clone())),
        );
        assert_eq!(transition.model.workspace, None);
        assert_eq!(transition.model.workspace_error, Some(error));
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn workspace_picker_and_nested_directory_requests_declare_effects() {
        let picker = update(Model::default(), Message::OpenWorkspacePickerRequested);
        assert_eq!(picker.commands, vec![Command::OpenWorkspacePicker]);

        let root = PathBuf::from("project");
        let directory = root.join("src");
        let mut model = Model::default();
        model.workspace = Some(Workspace {
            root: root.clone(),
            entries: vec![crate::workspace::WorkspaceEntry {
                name: "src".into(),
                path: directory.clone(),
                kind: crate::workspace::WorkspaceEntryKind::Directory,
                children: None,
            }],
        });
        let transition = update(
            model,
            Message::LoadWorkspaceDirectoryRequested {
                root: root.clone(),
                relative_path: PathBuf::from("src"),
            },
        );
        assert_eq!(
            transition.commands,
            vec![Command::ReadWorkspaceDirectory {
                root: root.clone(),
                relative_path: PathBuf::from("src"),
            }]
        );

        let entries = vec![crate::workspace::WorkspaceEntry {
            name: "main.rs".into(),
            path: directory.join("main.rs"),
            kind: crate::workspace::WorkspaceEntryKind::File,
            children: None,
        }];
        let transition = update(
            transition.model,
            Message::WorkspaceDirectoryLoaded {
                root: root.clone(),
                relative_path: PathBuf::from("src"),
                result: Ok(entries.clone()),
            },
        );
        assert_eq!(
            transition.model.workspace.unwrap().entries[0].children,
            Some(entries)
        );
    }

    #[test]
    fn failed_file_request_preserves_active_editor_and_retains_open_error() {
        let path = PathBuf::from("main.rs");
        let mut initial = Model::default();
        {
            let editor = &mut initial.active_editor;
            editor.document.insert_text(&mut editor.cursor, "keep me");
        }
        let transition = update(initial, Message::OpenFileRequested(path.clone()));
        let error = FileError::Read {
            path: path.clone(),
            message: "missing".into(),
        };
        assert_eq!(
            transition.model.active_editor.state,
            crate::model::EditorState::Empty
        );
        assert_eq!(transition.model.active_editor.document.text(), "keep me");
        let request_id = match transition.commands.as_slice() {
            [
                Command::OpenFile {
                    request_id,
                    path: command_path,
                },
            ] => {
                assert_eq!(command_path, &path);
                *request_id
            }
            commands => panic!("expected an open-file command, got {commands:?}"),
        };

        let transition = update(
            transition.model,
            Message::FileOpened {
                request_id,
                result: Err(error.clone()),
            },
        );
        assert_eq!(
            transition.model.active_editor.state,
            crate::model::EditorState::Empty
        );
        assert_eq!(transition.model.active_editor.document.text(), "keep me");
        assert_eq!(transition.model.open_file_error, Some((request_id, error)));
    }

    #[test]
    fn out_of_order_open_results_do_not_replace_the_latest_requested_editor() {
        let first_path = PathBuf::from("first.rs");
        let first_request = update(
            Model::default(),
            Message::OpenFileRequested(first_path.clone()),
        );
        let first_request_id = open_request_id(&first_request.commands);
        let second_path = PathBuf::from("second.rs");
        let second_request = update(
            first_request.model,
            Message::OpenFileRequested(second_path.clone()),
        );
        let second_request_id = open_request_id(&second_request.commands);

        let first_completed = update(
            second_request.model,
            Message::FileOpened {
                request_id: first_request_id,
                result: Ok(loaded_file(&first_path, "first")),
            },
        );
        assert_eq!(
            first_completed.model.active_editor.view_id,
            crate::model::EditorViewId(0)
        );
        assert_eq!(
            first_completed.model.inactive_editors[0].file_path.as_ref(),
            Some(&first_path)
        );

        let second_completed = update(
            first_completed.model,
            Message::FileOpened {
                request_id: second_request_id,
                result: Ok(loaded_file(&second_path, "second")),
            },
        );
        assert_eq!(
            second_completed.model.active_editor.file_path,
            Some(second_path.clone())
        );
        assert_eq!(
            second_completed.model.active_editor.document.text(),
            "second"
        );
        assert_eq!(
            second_completed.model.tab_order.len(),
            2,
            "the pristine initial editor should not become an extra tab"
        );
        assert!(
            second_completed
                .model
                .inactive_editors
                .iter()
                .any(|editor| editor.file_path.as_ref() == Some(&first_path))
        );
    }

    #[test]
    fn selecting_a_view_while_an_open_is_pending_keeps_that_selection() {
        let current_path = PathBuf::from("current.rs");
        let current = open_file(Model::default(), loaded_file(&current_path, "current"));
        let current_view = current.active_editor.view_id;
        let pending = update(
            current,
            Message::OpenFileRequested(PathBuf::from("background.rs")),
        );
        let request_id = open_request_id(&pending.commands);
        let mut model = pending.model;
        assert!(!model.activate_view(current_view));

        let completed = update(
            model,
            Message::FileOpened {
                request_id,
                result: Ok(loaded_file(&PathBuf::from("background.rs"), "background")),
            },
        );
        assert_eq!(completed.model.active_editor.view_id, current_view);
        assert_eq!(completed.model.active_editor.document.text(), "current");
        assert!(
            completed
                .model
                .inactive_editors
                .iter()
                .any(|editor| editor.document.text() == "background")
        );
    }

    #[test]
    fn tab_activation_and_cycle_restore_each_editor_state() {
        let first_path = PathBuf::from("first.rs");
        let mut model = open_file(Model::default(), loaded_file(&first_path, "first"));
        let first_view = model.active_editor.view_id;
        model = update(model, Message::TextInput("!".to_owned())).model;
        let second_path = PathBuf::from("second.rs");
        model = open_file(model, loaded_file(&second_path, "second"));
        let second_view = model.active_editor.view_id;

        let first_selected = update(model, Message::ActivateEditorView(first_view));
        assert_eq!(first_selected.model.active_editor.view_id, first_view);
        assert_eq!(first_selected.model.active_editor.document.text(), "!first");
        assert!(first_selected.model.active_editor.document.dirty);
        assert_eq!(first_selected.model.active_editor.cursor.column, 1);
        assert_eq!(first_selected.commands, vec![Command::RequestRedraw]);

        let previous = update(
            first_selected.model,
            Message::CycleEditorView { reverse: true },
        );
        assert_eq!(previous.model.active_editor.view_id, second_view);
        assert_eq!(previous.model.active_editor.document.text(), "second");

        let next = update(previous.model, Message::CycleEditorView { reverse: false });
        assert_eq!(next.model.active_editor.view_id, first_view);
        assert_eq!(next.model.active_editor.document.text(), "!first");
    }

    #[test]
    fn scroll_messages_update_both_viewport_offsets() {
        let mut model = Model::default();
        model.active_editor.viewport.vertical_offset = 4;
        model.active_editor.viewport.horizontal_offset = 3;

        let transition = update(
            model,
            Message::Scrolled {
                vertical: -2,
                horizontal: 5,
            },
        );

        assert_eq!(transition.model.active_editor.viewport.vertical_offset, 2);
        assert_eq!(transition.model.active_editor.viewport.horizontal_offset, 8);
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn workspace_success_replaces_old_error() {
        let error = WorkspaceError::InvalidRoot(PathBuf::from("missing-workspace"));
        let model = update(Model::default(), Message::WorkspaceOpened(Err(error))).model;
        let workspace = Workspace {
            root: PathBuf::from("workspace"),
            entries: Vec::new(),
        };

        let transition = update(model, Message::WorkspaceOpened(Ok(workspace.clone())));
        assert_eq!(transition.model.workspace, Some(workspace));
        assert_eq!(transition.model.workspace_error, None);
    }

    #[test]
    fn opening_a_workspace_closes_the_previous_documents() {
        let mut model = Model::default();
        model.active_editor.file_path = Some(PathBuf::from("main.rs"));
        model.active_editor.state = crate::model::EditorState::Active;
        let workspace = Workspace {
            root: PathBuf::from("workspace"),
            entries: Vec::new(),
        };

        let transition = update(model, Message::WorkspaceOpened(Ok(workspace)));

        assert_eq!(
            transition.model.active_editor.state,
            crate::model::EditorState::Empty
        );
        assert_eq!(transition.model.active_editor.file_path, None);
        assert!(transition.model.inactive_editors.is_empty());
        assert_eq!(
            transition.model.tab_order,
            vec![transition.model.active_editor.view_id]
        );
    }

    #[test]
    fn closing_a_workspace_clears_the_workspace_and_restores_an_empty_editor() {
        let mut model = Model::default();
        model.workspace = Some(Workspace {
            root: PathBuf::from("workspace"),
            entries: Vec::new(),
        });
        model.workspace_error = Some(WorkspaceError::InvalidRoot(PathBuf::from("old")));
        model.active_editor.file_path = Some(PathBuf::from("workspace/main.rs"));
        model.active_editor.state = crate::model::EditorState::Active;

        let transition = update(model, Message::CloseWorkspace);

        assert_eq!(transition.model.workspace, None);
        assert_eq!(transition.model.workspace_error, None);
        assert_eq!(transition.model.active_editor.file_path, None);
        assert_eq!(
            transition.model.active_editor.state,
            crate::model::EditorState::Empty
        );
        assert!(transition.model.inactive_editors.is_empty());
        assert_eq!(
            transition.model.tab_order,
            vec![transition.model.active_editor.view_id]
        );
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn opening_a_file_resets_document_state_and_selects_its_language() {
        let path = PathBuf::from("main.rs");
        let transition = open_file(
            Model::default(),
            LoadedFile {
                path: path.clone(),
                identity_path: path.clone(),
                text: "fn main() {}".into(),
                language: SyntaxLanguage::Rust,
                stamp: FileStamp {
                    length: 12,
                    modified: None,
                },
            },
        );

        assert_eq!(transition.active_editor.file_path, Some(path));
        assert_eq!(transition.active_editor.document.text(), "fn main() {}");
        assert_eq!(
            transition.active_editor.document.language,
            SyntaxLanguage::Rust
        );
        assert!(!transition.active_editor.document.dirty);
        assert_eq!(transition.active_editor.cursor, Position::default());
    }

    #[test]
    fn failed_save_keeps_dirty_state_and_reports_the_error() {
        let path = PathBuf::from("main.rs");
        let model = open_file(
            Model::default(),
            LoadedFile {
                path: path.clone(),
                identity_path: path.clone(),
                text: "fn main() {}".into(),
                language: SyntaxLanguage::Rust,
                stamp: FileStamp {
                    length: 12,
                    modified: None,
                },
            },
        );
        let model = update(model, Message::TextInput("!".into())).model;
        let document_id = model.active_editor.document_id;
        let transition = update(
            model,
            Message::FileSaved {
                document_id,
                saved_text: "!fn main() {}".into(),
                result: Err(FileError::Write {
                    path: path.clone(),
                    message: "permission denied".into(),
                }),
            },
        );

        assert!(transition.model.active_editor.document.dirty);
        assert_eq!(transition.model.active_editor.file_path, Some(path.clone()));
        assert_eq!(
            transition.model.active_editor.file_error,
            Some(FileError::Write {
                path,
                message: "permission denied".into(),
            })
        );
    }

    #[test]
    fn save_as_detects_extensions_unless_the_document_mode_was_overridden() {
        let path = PathBuf::from("main.ts");
        let saved_file = || SavedFile {
            path: path.clone(),
            identity_path: path.clone(),
            stamp: FileStamp {
                length: 0,
                modified: None,
            },
        };
        let initial = Model::default();
        let document_id = initial.active_editor.document_id;
        let automatic = update(
            initial,
            Message::FileSaved {
                document_id,
                saved_text: String::new(),
                result: Ok(saved_file()),
            },
        )
        .model;
        assert_eq!(
            automatic.active_editor.document.language,
            SyntaxLanguage::Monaco("typescript")
        );

        let initial = Model::default();
        let document_id = initial.active_editor.document_id;
        let overridden = update(
            initial,
            Message::SetDocumentLanguage {
                document_id,
                language: SyntaxLanguage::Rust,
            },
        )
        .model;
        let overridden = update(
            overridden,
            Message::FileSaved {
                document_id,
                saved_text: String::new(),
                result: Ok(saved_file()),
            },
        )
        .model;
        assert_eq!(
            overridden.active_editor.document.language,
            SyntaxLanguage::Rust
        );
    }

    #[test]
    fn save_completion_updates_its_document_after_another_view_becomes_active() {
        let first_path = PathBuf::from("first.rs");
        let first = open_file(
            Model::default(),
            LoadedFile {
                path: first_path.clone(),
                identity_path: first_path.clone(),
                text: "first".into(),
                language: SyntaxLanguage::Rust,
                stamp: FileStamp {
                    length: 5,
                    modified: None,
                },
            },
        );
        let first_view = first.active_editor.view_id;
        let first_document = first.active_editor.document_id;
        let dirty_first = update(first, Message::TextInput("!".into())).model;
        let save = update(dirty_first, Message::SaveFileRequested);

        assert_eq!(
            save.commands,
            vec![Command::SaveFile {
                document_id: first_document,
                path: first_path.clone(),
                text: "!first".into(),
                expected_stamp: Some(FileStamp {
                    length: 5,
                    modified: None,
                }),
            }]
        );

        let second_path = PathBuf::from("second.rs");
        let second = open_file(
            save.model,
            LoadedFile {
                path: second_path.clone(),
                identity_path: second_path.clone(),
                text: "second".into(),
                language: SyntaxLanguage::Rust,
                stamp: FileStamp {
                    length: 6,
                    modified: None,
                },
            },
        );
        let second_view = second.active_editor.view_id;
        let completion = update(
            second,
            Message::FileSaved {
                document_id: first_document,
                saved_text: "!first".into(),
                result: Ok(crate::file_io::SavedFile {
                    path: first_path.clone(),
                    identity_path: first_path,
                    stamp: FileStamp {
                        length: 6,
                        modified: None,
                    },
                }),
            },
        );

        assert_eq!(completion.model.active_editor.view_id, second_view);
        assert_eq!(completion.model.active_editor.document.text(), "second");
        let saved_first = completion
            .model
            .inactive_editors
            .iter()
            .find(|editor| editor.view_id == first_view)
            .unwrap();
        assert!(!saved_first.document.dirty);
        assert_eq!(
            saved_first.file_stamp,
            Some(FileStamp {
                length: 6,
                modified: None,
            })
        );
    }

    fn open_file(model: Model, file: LoadedFile) -> Model {
        let requested = update(model, Message::OpenFileRequested(file.path.clone()));
        let request_id = requested
            .commands
            .iter()
            .find_map(|command| match command {
                Command::OpenFile { request_id, .. } => Some(*request_id),
                _ => None,
            })
            .expect("a new path should request a file load");
        update(
            requested.model,
            Message::FileOpened {
                request_id,
                result: Ok(file),
            },
        )
        .model
    }

    fn open_request_id(commands: &[Command]) -> crate::model::OpenRequestId {
        match commands {
            [Command::OpenFile { request_id, .. }] => *request_id,
            commands => panic!("expected one open-file command, got {commands:?}"),
        }
    }

    fn loaded_file(path: &std::path::Path, text: &str) -> LoadedFile {
        LoadedFile {
            path: path.to_path_buf(),
            identity_path: path.to_path_buf(),
            text: text.to_owned(),
            language: SyntaxLanguage::from_path(path),
            stamp: FileStamp {
                length: text.len() as u64,
                modified: None,
            },
        }
    }

    #[test]
    fn pointer_press_moves_cursor_and_requests_redraw() {
        let model = update(Model::default(), Message::TextInput("abc\ndef".into())).model;
        let transition = update(
            model,
            Message::PointerPressed {
                position: Position {
                    line: 1,
                    column: 99,
                },
            },
        );

        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 1, column: 3 }
        );
        assert_eq!(
            transition.model.active_editor.selection,
            Some(Selection {
                anchor: Position { line: 1, column: 3 },
                focus: Position { line: 1, column: 3 },
            })
        );
        assert_eq!(transition.commands, vec![Command::RequestRedraw]);
    }

    #[test]
    fn pointer_drag_extends_selection_from_the_pressed_position() {
        let model = update(Model::default(), Message::TextInput("abcdef".into())).model;
        let model = update(
            model,
            Message::PointerPressed {
                position: Position { line: 0, column: 1 },
            },
        )
        .model;
        let transition = update(
            model,
            Message::PointerDragged {
                position: Position { line: 0, column: 4 },
            },
        );

        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 4 }
        );
        assert_eq!(
            transition.model.active_editor.selection,
            Some(Selection {
                anchor: Position { line: 0, column: 1 },
                focus: Position { line: 0, column: 4 },
            })
        );
    }

    #[test]
    fn text_input_replaces_a_selected_range() {
        let model = update(Model::default(), Message::TextInput("abcdef".into())).model;
        let model = update(
            update(
                model,
                Message::PointerPressed {
                    position: Position { line: 0, column: 1 },
                },
            )
            .model,
            Message::PointerDragged {
                position: Position { line: 0, column: 4 },
            },
        )
        .model;

        let transition = update(model, Message::TextInput("X".into()));

        assert_eq!(
            transition.model.active_editor.document.lines(),
            vec!["aXef"]
        );
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 2 }
        );
        assert_eq!(transition.model.active_editor.selection, None);
    }

    #[test]
    fn text_input_replaces_a_multiline_selection() {
        let model = update(Model::default(), Message::TextInput("ab\ncd\nef".into())).model;
        let model = update(
            model,
            Message::PointerPressed {
                position: Position { line: 0, column: 1 },
            },
        )
        .model;
        let model = update(
            model,
            Message::PointerDragged {
                position: Position { line: 2, column: 1 },
            },
        )
        .model;

        let transition = update(model, Message::TextInput("X".into()));

        assert_eq!(transition.model.active_editor.document.lines(), vec!["aXf"]);
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 2 }
        );
    }

    #[test]
    fn backspace_removes_a_selected_range() {
        let model = update(Model::default(), Message::TextInput("abcdef".into())).model;
        let model = update(
            update(
                model,
                Message::PointerPressed {
                    position: Position { line: 0, column: 1 },
                },
            )
            .model,
            Message::PointerDragged {
                position: Position { line: 0, column: 4 },
            },
        )
        .model;

        let transition = update(model, Message::KeyPressed(Key::Backspace));

        assert_eq!(transition.model.active_editor.document.lines(), vec!["aef"]);
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 1 }
        );
    }

    #[test]
    fn enter_replaces_a_selected_range_with_a_newline() {
        let model = update(Model::default(), Message::TextInput("abcdef".into())).model;
        let model = update(
            update(
                model,
                Message::PointerPressed {
                    position: Position { line: 0, column: 1 },
                },
            )
            .model,
            Message::PointerDragged {
                position: Position { line: 0, column: 4 },
            },
        )
        .model;

        let transition = update(model, Message::KeyPressed(Key::Enter));

        assert_eq!(
            transition.model.active_editor.document.lines(),
            vec!["a", "ef"]
        );
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 1, column: 0 }
        );
    }

    #[test]
    fn shift_navigation_extends_selection_without_mutating_text() {
        let model = update(Model::default(), Message::TextInput("abcd".into())).model;
        let model = update(model, Message::KeyPressed(Key::Home)).model;
        let model = update(model, Message::ModifiersChanged { shift: true }).model;
        let model = update(model, Message::KeyPressed(Key::Right)).model;
        let transition = update(model, Message::KeyPressed(Key::Right));

        assert_eq!(
            transition.model.active_editor.document.lines(),
            vec!["abcd"]
        );
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 2 }
        );
        assert_eq!(
            transition.model.active_editor.selection,
            Some(Selection {
                anchor: Position { line: 0, column: 0 },
                focus: Position { line: 0, column: 2 },
            })
        );
    }

    #[test]
    fn movement_clears_selection_without_deleting_text() {
        let model = update(Model::default(), Message::TextInput("abcd".into())).model;
        let model = update(
            update(
                model,
                Message::PointerPressed {
                    position: Position { line: 0, column: 1 },
                },
            )
            .model,
            Message::PointerDragged {
                position: Position { line: 0, column: 3 },
            },
        )
        .model;

        let transition = update(model, Message::KeyPressed(Key::Left));

        assert_eq!(
            transition.model.active_editor.document.lines(),
            vec!["abcd"]
        );
        assert_eq!(
            transition.model.active_editor.cursor,
            Position { line: 0, column: 2 }
        );
        assert_eq!(transition.model.active_editor.selection, None);
    }

    #[test]
    fn close_requests_exit_without_mutating_model() {
        let model = Model::default();
        let transition = update(model.clone(), Message::CloseRequested);

        assert_eq!(transition.model, model);
        assert_eq!(transition.commands, vec![Command::Exit]);
    }
}
