use ropey::Rope;
use std::collections::BTreeMap;

use crate::file_io::{FileError, FileStamp};
use crate::workspace::{Workspace, WorkspaceError};

use crate::syntax::SyntaxLanguage;

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Position,
    pub focus: Position,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextEdit {
    pub start: Position,
    pub end: Position,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImePreedit {
    pub text: String,
    /// Cursor range in Unicode scalar offsets within the preedit text.
    pub cursor: Option<(usize, usize)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    text: Rope,
    saved_text: Rope,
    pub dirty: bool,
    pub language: SyntaxLanguage,
    pub language_overridden: bool,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            text: Rope::new(),
            saved_text: Rope::new(),
            dirty: false,
            language: SyntaxLanguage::from_extension(None),
            language_overridden: false,
        }
    }
}

impl Document {
    pub fn from_text(text: &str, language: SyntaxLanguage) -> Self {
        Self {
            text: Rope::from_str(text),
            saved_text: Rope::from_str(text),
            dirty: false,
            language,
            language_overridden: false,
        }
    }

    pub fn text(&self) -> String {
        self.text.to_string()
    }

    pub fn mark_saved(&mut self, saved_text: &str) {
        self.saved_text = Rope::from_str(saved_text);
        self.reconcile_dirty();
    }

    pub fn reconcile_dirty(&mut self) {
        self.dirty = self.text != self.saved_text;
    }

    pub fn line_count(&self) -> usize {
        self.text.len_lines()
    }

    #[cfg(test)]
    pub fn lines(&self) -> Vec<String> {
        (0..self.line_count())
            .map(|line| self.line_text(line))
            .collect()
    }

    pub fn line_text(&self, line: usize) -> String {
        let text = self.text.line(line).to_string();
        text.strip_suffix("\r\n")
            .or_else(|| text.strip_suffix('\n'))
            .or_else(|| text.strip_suffix('\r'))
            .unwrap_or(&text)
            .to_owned()
    }

    pub fn line_len(&self, line: usize) -> usize {
        self.line_text(line).chars().count()
    }

    pub fn clamp_position(&self, position: Position) -> Position {
        let line = position.line.min(self.line_count().saturating_sub(1));
        let column = position.column.min(self.line_len(line));

        Position { line, column }
    }

    pub fn insert_text(&mut self, position: &mut Position, text: &str) {
        *position = self.clamp_position(position.clone());
        let start_index = self.text.line_to_char(position.line) + position.column;

        for (offset, character) in text.chars().enumerate() {
            let char_index = start_index + offset;
            self.text.insert_char(char_index, character);
            *position = position_after_insert(position, character);
        }

        if !text.is_empty() {
            self.dirty = true;
        }
    }

    pub fn delete_range(&mut self, start: &Position, end: &Position) {
        let start = self.clamp_position(start.clone());
        let end = self.clamp_position(end.clone());
        if start > end {
            return;
        }

        let start_index = self.text.line_to_char(start.line) + start.column;
        let end_index = self.text.line_to_char(end.line) + end.column;
        self.text.remove(start_index..end_index);

        if start != end {
            self.dirty = true;
        }
    }

    pub fn backspace(&mut self, position: &mut Position) {
        *position = self.clamp_position(position.clone());

        if position.column > 0 {
            let line_start = self.text.line_to_char(position.line);
            let end = line_start + position.column;
            self.text.remove(end - 1..end);
            position.column -= 1;
            self.dirty = true;
        } else if position.line > 0 {
            let line_start = self.text.line_to_char(position.line);
            position.line -= 1;
            position.column = self.line_len(position.line);
            let separator_start = if line_start >= 2 && self.text.char(line_start - 2) == '\r' {
                line_start - 2
            } else {
                line_start - 1
            };
            self.text.remove(separator_start..line_start);
            self.dirty = true;
        }
    }

    pub fn delete(&mut self, position: &mut Position) {
        *position = self.clamp_position(position.clone());

        if position.column < self.line_len(position.line) {
            let start = self.text.line_to_char(position.line) + position.column;
            self.text.remove(start..start + 1);
            self.dirty = true;
        } else if position.line + 1 < self.line_count() {
            let next_line_start = self.text.line_to_char(position.line + 1);
            let separator_start =
                if next_line_start >= 2 && self.text.char(next_line_start - 2) == '\r' {
                    next_line_start - 2
                } else {
                    next_line_start - 1
                };
            self.text.remove(separator_start..next_line_start);
            self.dirty = true;
        }
    }
}

fn position_after_insert(position: &Position, character: char) -> Position {
    if character == '\n' {
        Position {
            line: position.line + 1,
            column: 0,
        }
    } else {
        Position {
            line: position.line,
            column: position.column + 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Insert,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorState {
    #[default]
    Empty,
    Loading,
    Active,
    Error,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub vertical_offset: usize,
    pub horizontal_offset: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DocumentId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EditorViewId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OpenRequestId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorTabState {
    pub document_id: DocumentId,
    pub view_id: EditorViewId,
    pub document: Document,
    pub file_path: Option<std::path::PathBuf>,
    pub identity_path: Option<std::path::PathBuf>,
    pub file_stamp: Option<FileStamp>,
    pub file_error: Option<FileError>,
    pub cursor: Position,
    pub selection: Option<Selection>,
    pub ime_preedit: Option<ImePreedit>,
    pub viewport: Viewport,
    pub mode: EditorMode,
    pub state: EditorState,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentlyClosedDocument {
    pub path: std::path::PathBuf,
    pub identity_path: std::path::PathBuf,
    pub cursor: Position,
    pub selection: Option<Selection>,
}

pub const MAX_RECENTLY_CLOSED_DOCUMENTS: usize = 10;

impl EditorTabState {
    fn empty(document_id: DocumentId, view_id: EditorViewId) -> Self {
        Self {
            document_id,
            view_id,
            document: Document::default(),
            file_path: None,
            identity_path: None,
            file_stamp: None,
            file_error: None,
            cursor: Position::default(),
            selection: None,
            ime_preedit: None,
            viewport: Viewport::default(),
            mode: EditorMode::Insert,
            state: EditorState::Empty,
        }
    }

    fn from_loaded_file(
        document_id: DocumentId,
        view_id: EditorViewId,
        file: crate::file_io::LoadedFile,
    ) -> Self {
        Self {
            document_id,
            view_id,
            document: Document::from_text(&file.text, file.language),
            file_path: Some(file.path),
            identity_path: Some(file.identity_path),
            file_stamp: Some(file.stamp),
            file_error: None,
            cursor: Position::default(),
            selection: None,
            ime_preedit: None,
            viewport: Viewport::default(),
            mode: EditorMode::Insert,
            state: EditorState::Active,
        }
    }

    fn is_pristine_untitled(&self) -> bool {
        self.file_path.is_none() && !self.document.dirty && self.document.text().is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Model {
    pub active_editor: EditorTabState,
    pub inactive_editors: Vec<EditorTabState>,
    pub tab_order: Vec<EditorViewId>,
    pub open_file_error: Option<(OpenRequestId, FileError)>,
    pending_open_requests: BTreeMap<OpenRequestId, u64>,
    latest_open_request: Option<OpenRequestId>,
    selection_epoch: u64,
    pub workspace: Option<Workspace>,
    pub workspace_error: Option<WorkspaceError>,
    pub recently_closed: Vec<RecentlyClosedDocument>,
    pub shift_down: bool,
    pub command_bar_open: bool,
    pub command_bar_query: String,
    pub command_bar_selection: usize,
    pub settings_open: bool,
    pub needs_redraw: bool,
    next_document_id: u64,
    next_view_id: u64,
    next_open_request_id: u64,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            active_editor: EditorTabState::empty(DocumentId(0), EditorViewId(0)),
            inactive_editors: Vec::new(),
            tab_order: vec![EditorViewId(0)],
            open_file_error: None,
            pending_open_requests: BTreeMap::new(),
            latest_open_request: None,
            selection_epoch: 0,
            workspace: None,
            workspace_error: None,
            recently_closed: Vec::new(),
            shift_down: false,
            command_bar_open: false,
            command_bar_query: String::new(),
            command_bar_selection: 0,
            settings_open: false,
            needs_redraw: true,
            next_document_id: 1,
            next_view_id: 1,
            next_open_request_id: 1,
        }
    }
}

impl Model {
    pub fn initial() -> Self {
        Self::default()
    }

    pub fn has_dirty_documents(&self) -> bool {
        self.active_editor.document.dirty
            || self
                .inactive_editors
                .iter()
                .any(|editor| editor.document.dirty)
    }

    pub fn reset_documents(&mut self) {
        let (document_id, view_id) = self.allocate_editor_ids();
        self.active_editor = EditorTabState::empty(document_id, view_id);
        self.inactive_editors.clear();
        self.tab_order = vec![view_id];
        self.open_file_error = None;
        self.pending_open_requests.clear();
        self.latest_open_request = None;
        self.recently_closed.clear();
        self.selection_epoch = self
            .selection_epoch
            .checked_add(1)
            .expect("editor selection generation exhausted");
    }

    pub fn create_untitled_document(
        &mut self,
        text: &str,
        language: SyntaxLanguage,
        activate: bool,
    ) -> EditorViewId {
        let reuses_initial = activate
            && self.active_editor.is_pristine_untitled()
            && self.inactive_editors.is_empty();
        let (document_id, view_id) = if reuses_initial {
            (self.active_editor.document_id, self.active_editor.view_id)
        } else {
            self.allocate_editor_ids()
        };
        let mut editor = EditorTabState::empty(document_id, view_id);
        editor.document = Document::from_text(text, language);

        if reuses_initial {
            self.active_editor = editor;
        } else if activate {
            let previous = std::mem::replace(&mut self.active_editor, editor);
            self.inactive_editors.push(previous);
            self.tab_order.push(view_id);
            self.selection_epoch = self
                .selection_epoch
                .checked_add(1)
                .expect("editor selection generation exhausted");
        } else {
            self.inactive_editors.push(editor);
            self.tab_order.push(view_id);
        }
        view_id
    }

    pub fn activate_view(&mut self, view_id: EditorViewId) -> bool {
        if self.active_editor.view_id == view_id {
            self.selection_epoch = self
                .selection_epoch
                .checked_add(1)
                .expect("editor selection generation exhausted");
            return false;
        }
        let Some(index) = self
            .inactive_editors
            .iter()
            .position(|editor| editor.view_id == view_id)
        else {
            return false;
        };
        let next = self.inactive_editors.remove(index);
        let previous = std::mem::replace(&mut self.active_editor, next);
        self.inactive_editors.push(previous);
        self.selection_epoch = self
            .selection_epoch
            .checked_add(1)
            .expect("editor selection generation exhausted");
        true
    }

    pub fn activate_next_view(&mut self, reverse: bool) -> bool {
        if self.tab_order.len() < 2 {
            return false;
        }
        let Some(active_index) = self
            .tab_order
            .iter()
            .position(|view_id| *view_id == self.active_editor.view_id)
        else {
            return false;
        };
        let next_index = if reverse {
            (active_index + self.tab_order.len() - 1) % self.tab_order.len()
        } else {
            (active_index + 1) % self.tab_order.len()
        };
        let next_view = self.tab_order[next_index];
        self.activate_view(next_view)
    }

    pub fn close_view(&mut self, view_id: EditorViewId) -> bool {
        let Some(index) = self.tab_order.iter().position(|id| *id == view_id) else {
            return false;
        };
        let recently_closed = self.editor_for_view(view_id).and_then(|editor| {
            if editor.document.dirty {
                return None;
            }
            let path = editor.file_path.clone()?;
            let identity_path = editor.identity_path.clone().unwrap_or_else(|| path.clone());
            Some(RecentlyClosedDocument {
                path,
                identity_path,
                cursor: editor.cursor.clone(),
                selection: editor.selection.clone(),
            })
        });
        let closing_active = self.active_editor.view_id == view_id;

        if self.tab_order.len() == 1 {
            let (document_id, empty_view_id) = self.allocate_editor_ids();
            self.active_editor = EditorTabState::empty(document_id, empty_view_id);
            self.inactive_editors.clear();
            self.tab_order = vec![empty_view_id];
        } else {
            let replacement_index = if closing_active {
                Some(
                    self.tab_order
                        .iter()
                        .filter(|id| **id != view_id)
                        .nth(index.min(self.tab_order.len() - 2))
                        .copied(),
                )
            } else {
                None
            };
            let replacement_index = match replacement_index {
                Some(Some(replacement_view)) => self
                    .inactive_editors
                    .iter()
                    .position(|editor| editor.view_id == replacement_view),
                Some(None) => return false,
                None => None,
            };
            let closing_index = if closing_active {
                None
            } else {
                self.inactive_editors
                    .iter()
                    .position(|editor| editor.view_id == view_id)
            };
            if (closing_active && replacement_index.is_none())
                || (!closing_active && closing_index.is_none())
            {
                return false;
            }

            self.tab_order.remove(index);
            if closing_active {
                if let Some(inactive_index) = replacement_index {
                    self.active_editor = self.inactive_editors.remove(inactive_index);
                }
            } else if let Some(inactive_index) = closing_index {
                self.inactive_editors.remove(inactive_index);
            }
        }

        self.selection_epoch = self
            .selection_epoch
            .checked_add(1)
            .expect("editor selection generation exhausted");
        if let Some(recently_closed) = recently_closed {
            self.remember_closed_document(recently_closed);
        }
        true
    }

    pub fn most_recently_closed_path(&self) -> Option<&std::path::Path> {
        self.recently_closed
            .first()
            .map(|document| document.path.as_path())
    }

    pub fn editor_for_view(&self, view_id: EditorViewId) -> Option<&EditorTabState> {
        if self.active_editor.view_id == view_id {
            Some(&self.active_editor)
        } else {
            self.inactive_editors
                .iter()
                .find(|editor| editor.view_id == view_id)
        }
    }

    pub fn editor_for_document(&self, document_id: DocumentId) -> Option<&EditorTabState> {
        if self.active_editor.document_id == document_id {
            Some(&self.active_editor)
        } else {
            self.inactive_editors
                .iter()
                .find(|editor| editor.document_id == document_id)
        }
    }

    pub fn begin_open_request(&mut self) -> OpenRequestId {
        let request_id = OpenRequestId(self.next_open_request_id);
        self.next_open_request_id = self
            .next_open_request_id
            .checked_add(1)
            .expect("file-open request identity space exhausted");
        self.pending_open_requests
            .insert(request_id, self.selection_epoch);
        self.latest_open_request = Some(request_id);
        request_id
    }

    pub fn complete_open_request(&mut self, request_id: OpenRequestId) -> Option<bool> {
        let request_epoch = self.pending_open_requests.remove(&request_id)?;
        let should_activate =
            self.latest_open_request == Some(request_id) && self.selection_epoch == request_epoch;
        if self.latest_open_request == Some(request_id) {
            self.latest_open_request = None;
        }
        Some(should_activate)
    }

    pub fn view_for_path(&self, path: &std::path::Path) -> Option<EditorViewId> {
        std::iter::once(&self.active_editor)
            .chain(self.inactive_editors.iter())
            .find(|editor| {
                editor
                    .file_path
                    .as_deref()
                    .is_some_and(|open_path| same_path(open_path, path))
                    || editor
                        .identity_path
                        .as_deref()
                        .is_some_and(|identity_path| same_path(identity_path, path))
            })
            .map(|editor| editor.view_id)
    }

    pub fn editor_for_document_mut(
        &mut self,
        document_id: DocumentId,
    ) -> Option<&mut EditorTabState> {
        if self.active_editor.document_id == document_id {
            Some(&mut self.active_editor)
        } else {
            self.inactive_editors
                .iter_mut()
                .find(|editor| editor.document_id == document_id)
        }
    }

    pub fn open_loaded_file(
        &mut self,
        file: crate::file_io::LoadedFile,
        activate: bool,
    ) -> EditorViewId {
        let recent = self
            .recently_closed
            .iter()
            .position(|document| same_path(&document.identity_path, &file.identity_path))
            .map(|index| self.recently_closed.remove(index));
        if let Some(view_id) = std::iter::once(&self.active_editor)
            .chain(self.inactive_editors.iter())
            .find(|editor| {
                editor
                    .identity_path
                    .as_deref()
                    .is_some_and(|path| same_path(path, &file.identity_path))
            })
            .map(|editor| editor.view_id)
        {
            if activate && self.active_editor.view_id != view_id {
                self.activate_view(view_id);
            }
            return view_id;
        }

        let replaces_pristine_initial = activate && self.active_editor.is_pristine_untitled();
        let reuses_initial_identity = replaces_pristine_initial && self.inactive_editors.is_empty();
        let (document_id, view_id) = if reuses_initial_identity {
            (self.active_editor.document_id, self.active_editor.view_id)
        } else {
            self.allocate_editor_ids()
        };
        let mut opened = EditorTabState::from_loaded_file(document_id, view_id, file);
        if let Some(recent) = recent {
            opened.cursor = opened.document.clamp_position(recent.cursor);
            opened.selection = recent.selection.map(|selection| Selection {
                anchor: opened.document.clamp_position(selection.anchor),
                focus: opened.document.clamp_position(selection.focus),
            });
        }
        if reuses_initial_identity {
            self.active_editor = opened;
        } else if replaces_pristine_initial {
            self.tab_order
                .retain(|id| *id != self.active_editor.view_id);
            self.active_editor = opened;
            self.tab_order.push(view_id);
            self.selection_epoch = self
                .selection_epoch
                .checked_add(1)
                .expect("editor selection generation exhausted");
        } else if activate {
            let previous = std::mem::replace(&mut self.active_editor, opened);
            self.inactive_editors.push(previous);
            self.tab_order.push(view_id);
            self.selection_epoch = self
                .selection_epoch
                .checked_add(1)
                .expect("editor selection generation exhausted");
        } else {
            self.inactive_editors.push(opened);
            self.tab_order.push(view_id);
        }
        view_id
    }

    fn remember_closed_document(&mut self, document: RecentlyClosedDocument) {
        self.recently_closed
            .retain(|recent| !same_path(&recent.identity_path, &document.identity_path));
        self.recently_closed.insert(0, document);
        self.recently_closed.truncate(MAX_RECENTLY_CLOSED_DOCUMENTS);
    }

    fn allocate_editor_ids(&mut self) -> (DocumentId, EditorViewId) {
        let document_id = DocumentId(self.next_document_id);
        let view_id = EditorViewId(self.next_view_id);
        self.next_document_id = self
            .next_document_id
            .checked_add(1)
            .expect("document identity space exhausted");
        self.next_view_id = self
            .next_view_id
            .checked_add(1)
            .expect("editor view identity space exhausted");
        (document_id, view_id)
    }
}

fn same_path(left: &std::path::Path, right: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        left.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case(&right.as_os_str().to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::{Document, EditorViewId, MAX_RECENTLY_CLOSED_DOCUMENTS, Model, Position};
    use crate::{
        file_io::{FileStamp, LoadedFile},
        syntax::SyntaxLanguage,
    };
    use std::path::PathBuf;

    #[test]
    fn initial_model_has_one_empty_line_and_requests_redraw() {
        let model = Model::initial();

        assert_eq!(model.active_editor.document.lines(), vec![String::new()]);
        assert!(model.needs_redraw);
        assert!(!model.active_editor.document.dirty);
        assert_eq!(model.tab_order, vec![EditorViewId(0)]);
    }

    #[test]
    fn multiline_edits_join_lines_and_clamp_document_boundaries() {
        let mut document = Document::default();
        let mut cursor = Position::default();
        document.insert_text(&mut cursor, "first\nsecond\nthird");

        assert_eq!(document.line_count(), 3);
        assert_eq!(cursor, Position { line: 2, column: 5 });

        document.delete_range(
            &Position { line: 0, column: 5 },
            &Position { line: 1, column: 1 },
        );
        assert_eq!(document.lines(), vec!["firstecond", "third"]);

        let clamped = document.clamp_position(Position {
            line: 99,
            column: 99,
        });
        assert_eq!(clamped, Position { line: 1, column: 5 });
    }

    #[test]
    fn empty_documents_and_boundary_deletes_remain_clean() {
        let mut document = Document::default();
        let mut cursor = Position {
            line: 99,
            column: 99,
        };

        document.backspace(&mut cursor);
        document.delete(&mut cursor);

        assert_eq!(cursor, Position::default());
        assert_eq!(document.lines(), vec![String::new()]);
        assert!(!document.dirty);
    }

    #[test]
    fn unicode_and_tabs_use_scalar_columns() {
        let mut document = Document::default();
        let mut cursor = Position::default();
        document.insert_text(&mut cursor, "é\t界");

        assert_eq!(document.line_len(0), 3);
        assert_eq!(cursor, Position { line: 0, column: 3 });
        assert_eq!(
            document.clamp_position(Position { line: 0, column: 8 }),
            cursor
        );
    }

    #[test]
    fn crlf_lines_exclude_carriage_returns_from_columns_and_delete_as_one_separator() {
        let mut document = Document::from_text("first\r\nsecond", SyntaxLanguage::PlainText);
        assert_eq!(document.lines(), vec!["first", "second"]);
        assert_eq!(document.line_len(0), 5);

        let mut cursor = Position { line: 1, column: 0 };
        document.backspace(&mut cursor);
        assert_eq!(document.text(), "firstsecond");
        assert_eq!(cursor, Position { line: 0, column: 5 });
    }

    #[test]
    fn undoing_back_to_the_saved_text_clears_dirty_state() {
        let mut document = Document::from_text("saved", SyntaxLanguage::PlainText);
        let mut cursor = Position { line: 0, column: 5 };
        document.insert_text(&mut cursor, "!");
        assert!(document.dirty);

        document.delete_range(
            &Position { line: 0, column: 5 },
            &Position { line: 0, column: 6 },
        );
        document.reconcile_dirty();

        assert_eq!(document.text(), "saved");
        assert!(!document.dirty);
    }

    #[test]
    fn rope_handles_a_large_multiline_document_behind_the_document_api() {
        let text = (0..10_000)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let document = Document::from_text(&text, crate::syntax::SyntaxLanguage::PlainText);

        assert_eq!(document.line_count(), 10_001);
        assert_eq!(document.line_text(9_999), "line 9999");
        assert_eq!(document.line_text(10_000), "");
    }

    #[test]
    fn opening_and_reactivating_files_preserves_document_and_view_state() {
        let mut model = Model::default();
        let first_view = model.open_loaded_file(loaded_file("first.rs", "first"), true);
        model.active_editor.cursor = Position { line: 0, column: 5 };
        model
            .active_editor
            .document
            .insert_text(&mut model.active_editor.cursor, "!");
        model.active_editor.cursor = Position { line: 0, column: 3 };
        model.active_editor.viewport.vertical_offset = 7;

        let second_view = model.open_loaded_file(loaded_file("second.rs", "second"), true);
        assert_ne!(first_view, second_view);
        assert_eq!(model.active_editor.document.text(), "second");
        assert_eq!(model.tab_order, vec![first_view, second_view]);

        assert!(model.activate_view(first_view));
        assert_eq!(model.active_editor.document.text(), "first!");
        assert_eq!(model.active_editor.cursor, Position { line: 0, column: 3 });
        assert_eq!(model.active_editor.viewport.vertical_offset, 7);
        assert!(model.active_editor.document.dirty);

        assert!(model.activate_view(second_view));
        assert_eq!(model.active_editor.document.text(), "second");
        assert!(!model.active_editor.document.dirty);
    }

    #[test]
    fn reopening_an_open_path_selects_the_existing_view_without_replacing_edits() {
        let mut model = Model::default();
        let first_view = model.open_loaded_file(loaded_file("first.rs", "first"), true);
        model.active_editor.cursor = Position { line: 0, column: 5 };
        model
            .active_editor
            .document
            .insert_text(&mut model.active_editor.cursor, "!");
        let second_view = model.open_loaded_file(loaded_file("second.rs", "second"), true);

        let reopened = model.open_loaded_file(loaded_file("first.rs", "stale disk text"), true);

        assert_eq!(reopened, first_view);
        assert_eq!(model.active_editor.view_id, first_view);
        assert_eq!(model.active_editor.document.text(), "first!");
        assert!(model.active_editor.document.dirty);
        assert_eq!(model.tab_order, vec![first_view, second_view]);
        assert_eq!(model.inactive_editors.len(), 1);
    }

    #[test]
    fn closing_active_view_selects_the_next_tab_or_previous_last_tab() {
        let mut model = Model::default();
        let first = model.open_loaded_file(loaded_file("first.rs", "first"), true);
        let second = model.open_loaded_file(loaded_file("second.rs", "second"), true);
        let third = model.open_loaded_file(loaded_file("third.rs", "third"), true);

        assert!(model.close_view(second));
        assert_eq!(model.active_editor.view_id, third);
        assert_eq!(model.tab_order, vec![first, third]);

        assert!(model.close_view(third));
        assert_eq!(model.active_editor.view_id, first);
        assert_eq!(model.tab_order, vec![first]);
    }

    #[test]
    fn closing_last_view_leaves_one_clean_untitled_view() {
        let mut model = Model::default();
        let initial_view = model.active_editor.view_id;

        assert!(model.close_view(initial_view));
        assert_eq!(model.tab_order.len(), 1);
        assert_ne!(model.active_editor.view_id, initial_view);
        assert!(model.active_editor.is_pristine_untitled());
    }

    #[test]
    fn closing_an_inactive_view_preserves_active_document() {
        let mut model = Model::default();
        let first = model.open_loaded_file(loaded_file("first.rs", "first"), true);
        let second = model.open_loaded_file(loaded_file("second.rs", "second"), true);

        assert!(model.close_view(first));
        assert_eq!(model.active_editor.view_id, second);
        assert_eq!(model.tab_order, vec![second]);
    }

    #[test]
    fn reopening_a_recently_closed_file_restores_its_selection() {
        let mut model = Model::default();
        let view_id = model.open_loaded_file(loaded_file("recent.rs", "abcdef"), true);
        model.active_editor.cursor = Position { line: 0, column: 5 };
        model.active_editor.selection = Some(crate::model::Selection {
            anchor: Position { line: 0, column: 2 },
            focus: Position { line: 0, column: 5 },
        });

        assert!(model.close_view(view_id));
        assert_eq!(
            model.most_recently_closed_path(),
            Some(std::path::Path::new("recent.rs"))
        );

        let reopened = model.open_loaded_file(loaded_file("recent.rs", "abcdef"), true);
        let editor = model.editor_for_view(reopened).unwrap();
        assert_eq!(editor.cursor, Position { line: 0, column: 5 });
        assert_eq!(
            editor.selection,
            Some(crate::model::Selection {
                anchor: Position { line: 0, column: 2 },
                focus: Position { line: 0, column: 5 },
            })
        );
        assert!(model.recently_closed.is_empty());
    }

    #[test]
    fn discarding_a_dirty_document_does_not_add_it_to_recently_closed() {
        let mut model = Model::default();
        let view_id = model.open_loaded_file(loaded_file("dirty.rs", "text"), true);
        let mut position = Position { line: 0, column: 4 };
        model.active_editor.document.insert_text(&mut position, "!");

        assert!(model.close_view(view_id));
        assert!(model.recently_closed.is_empty());
    }

    #[test]
    fn recently_closed_history_is_bounded_and_most_recent_first() {
        let mut model = Model::default();
        let count = MAX_RECENTLY_CLOSED_DOCUMENTS + 2;

        for index in 0..count {
            let path = format!("file-{index}.rs");
            let view_id = model.open_loaded_file(loaded_file(&path, "text"), true);
            assert!(model.close_view(view_id));
        }

        assert_eq!(model.recently_closed.len(), MAX_RECENTLY_CLOSED_DOCUMENTS);
        assert_eq!(
            model.most_recently_closed_path(),
            Some(std::path::Path::new(
                format!("file-{}.rs", count - 1).as_str()
            ))
        );
    }

    #[test]
    fn loaded_alias_with_the_same_identity_activates_the_existing_document() {
        let mut model = Model::default();
        let existing = model.open_loaded_file(loaded_file("original.rs", "original"), true);
        let mut alias = loaded_file("alias.rs", "stale disk contents");
        alias.identity_path = std::path::PathBuf::from("original.rs");

        let reopened = model.open_loaded_file(alias, true);

        assert_eq!(reopened, existing);
        assert_eq!(model.active_editor.view_id, existing);
        assert_eq!(model.active_editor.document.text(), "original");
        assert_eq!(model.tab_order, vec![existing]);
    }

    fn loaded_file(path: &str, text: &str) -> LoadedFile {
        LoadedFile {
            path: PathBuf::from(path),
            identity_path: PathBuf::from(path),
            text: text.to_owned(),
            language: SyntaxLanguage::from_path(std::path::Path::new(path)),
            stamp: FileStamp {
                length: text.len() as u64,
                modified: None,
            },
        }
    }
}
