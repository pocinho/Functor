use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::protocol::{
    ApplyDocumentEditsRequest, ApplyDocumentEditsResponse, BridgeError, BridgeErrorCode,
    CloseDocumentRequest, DocumentLanguage, DocumentSnapshot, MonacoPosition, MonacoSelection,
    MonacoTextChange, SetDocumentLanguageRequest, UpdateSelectionRequest, UpdateSelectionResponse,
    WorkspaceEntryKind as ProtocolEntryKind, WorkspaceEntrySnapshot, WorkspaceSnapshot,
    WorkspaceStateSnapshot, validate_text_changes,
};
use crate::recovery_io::{RecoveryFileStamp, RecoverySnapshot};
use crate::workspace_io::{self, SearchDocumentInput, WorkspaceSessionState};
use functor_core::file_io::{FileError, FileStamp, LoadedFile, SavedFile, normalize_newlines};
use functor_core::model::{DocumentId, EditorViewId, Model, Position, Selection, TextEdit};
use functor_core::mvu::{self, Command, Message};
use functor_core::syntax::SyntaxLanguage;
use functor_core::workspace::{Workspace, WorkspaceEntry, WorkspaceEntryKind, WorkspaceError};

pub struct EditorSession {
    model: Model,
    revisions: BTreeMap<DocumentId, u64>,
}

#[derive(Clone)]
pub struct OpenTicket {
    request_id: functor_core::model::OpenRequestId,
    path: PathBuf,
    workspace_root: Option<PathBuf>,
    require_loaded_tree_entry: bool,
}

pub struct SaveTicket {
    document_id: DocumentId,
    saved_text: String,
    path: PathBuf,
    expected_stamp: Option<FileStamp>,
}

impl Default for EditorSession {
    fn default() -> Self {
        let mut session = Self {
            model: Model::default(),
            revisions: BTreeMap::new(),
        };
        session.register_documents();
        session
    }
}

impl EditorSession {
    pub fn snapshot(&self) -> Result<WorkspaceSnapshot, BridgeError> {
        let mut documents = Vec::with_capacity(self.model.tab_order.len());
        for view_id in &self.model.tab_order {
            let editor = self.model.editor_for_view(*view_id).ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::Internal,
                    "The editor tab order refers to a missing editor view.",
                )
            })?;
            let revision = self
                .revisions
                .get(&editor.document_id)
                .copied()
                .ok_or_else(|| {
                    BridgeError::new(
                        BridgeErrorCode::Internal,
                        "The document revision is missing.",
                    )
                })?;
            let title = editor
                .file_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Untitled {}", editor.document_id.0 + 1));
            let cursor = position_to_monaco(&editor.document, &editor.cursor);
            let selection = editor.selection.as_ref().map(|selection| MonacoSelection {
                anchor: position_to_monaco(&editor.document, &selection.anchor),
                focus: position_to_monaco(&editor.document, &selection.focus),
            });
            documents.push(DocumentSnapshot {
                document_id: editor.document_id.0,
                view_id: editor.view_id.0,
                title,
                is_untitled: editor.file_path.is_none(),
                text: editor.document.text(),
                language: language_to_protocol(editor.document.language),
                revision,
                dirty: editor.document.dirty,
                cursor,
                selection,
            });
        }
        Ok(WorkspaceSnapshot {
            documents,
            active_view_id: self.model.active_editor.view_id.0,
            workspace: self
                .model
                .workspace
                .as_ref()
                .map(workspace_snapshot)
                .transpose()?,
            workspace_error: self.model.workspace_error.as_ref().map(ToString::to_string),
            persistence_warning: None,
            can_reopen_closed_document: self.model.most_recently_closed_path().is_some(),
        })
    }

    pub fn request_workspace_picker(&mut self) -> Result<(), BridgeError> {
        self.ensure_workspace_switch_documents_clean()?;
        let transition = mvu::update(self.model.clone(), Message::OpenWorkspacePickerRequested);
        let requested = transition
            .commands
            .iter()
            .any(|command| matches!(command, Command::OpenWorkspacePicker));
        self.model = transition.model;
        if requested {
            Ok(())
        } else {
            Err(BridgeError::new(
                BridgeErrorCode::Internal,
                "The workspace picker effect was not produced.",
            ))
        }
    }

    pub fn workspace_root(&self) -> Option<PathBuf> {
        self.model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
    }

    pub fn ensure_workspace_switch_documents_clean(&self) -> Result<(), BridgeError> {
        if self.model.has_dirty_documents() {
            return Err(BridgeError::invalid_request(
                "Save all open documents before opening another workspace.",
            ));
        }
        Ok(())
    }

    pub fn recovery_identity_path(
        &self,
        document_id: u64,
        expected_revision: u64,
    ) -> Result<Option<PathBuf>, BridgeError> {
        let id = DocumentId(document_id);
        self.current_revision(id, expected_revision)?;
        self.model
            .editor_for_document(id)
            .map(|editor| editor.identity_path.clone())
            .ok_or_else(|| document_not_found(document_id))
    }

    pub fn document_id_for_view(&self, view_id: u64) -> Result<u64, BridgeError> {
        self.model
            .editor_for_view(EditorViewId(view_id))
            .map(|editor| editor.document_id.0)
            .ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::DocumentNotFound,
                    "The requested editor view does not exist.",
                )
            })
    }

    pub fn workspace_session_state(&self) -> Option<WorkspaceSessionState> {
        let workspace = self.model.workspace.as_ref()?;
        let mut open_files = Vec::new();
        for view_id in &self.model.tab_order {
            let Some(editor) = self.model.editor_for_view(*view_id) else {
                continue;
            };
            let Some(path) = editor.identity_path.as_deref() else {
                continue;
            };
            if let Ok(relative_path) = path.strip_prefix(&workspace.root)
                && !relative_path.as_os_str().is_empty()
                && !open_files.contains(&relative_path.to_path_buf())
            {
                open_files.push(relative_path.to_path_buf());
            }
        }
        let active_file = self
            .model
            .active_editor
            .identity_path
            .as_deref()
            .and_then(|path| path.strip_prefix(&workspace.root).ok())
            .map(PathBuf::from);
        Some(WorkspaceSessionState {
            root: workspace.root.clone(),
            open_files,
            active_file,
        })
    }

    pub fn restore_workspace_documents(
        &mut self,
        files: Vec<LoadedFile>,
        active_file: Option<PathBuf>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        for (index, file) in files.into_iter().enumerate() {
            self.model.open_loaded_file(file, index == 0);
        }
        if let Some(active_file) = active_file {
            let path = root.join(active_file);
            if let Some(view_id) = self.model.view_for_path(&path) {
                self.model.activate_view(view_id);
            }
        }
        self.register_documents();
        self.snapshot()
    }

    pub fn recovery_snapshot(
        &self,
        document_id: u64,
        expected_revision: u64,
        recovery_id: String,
        session_id: String,
        generation: u64,
    ) -> Result<Option<RecoverySnapshot>, BridgeError> {
        let id = DocumentId(document_id);
        let revision = self.current_revision(id, expected_revision)?;
        let editor = self
            .model
            .editor_for_document(id)
            .ok_or_else(|| document_not_found(document_id))?;
        if !editor.document.dirty {
            return Ok(None);
        }
        let title = editor
            .file_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("Untitled {}", editor.document_id.0 + 1));
        let tab_index = self
            .model
            .tab_order
            .iter()
            .position(|view_id| *view_id == editor.view_id)
            .ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::Internal,
                    "The recovery document is missing from the tab order.",
                )
            })?;
        Ok(Some(RecoverySnapshot::new(
            recovery_id,
            session_id,
            generation,
            title,
            editor.file_path.clone(),
            editor.identity_path.clone(),
            editor.document.text(),
            language_to_protocol(editor.document.language),
            revision,
            position_to_monaco(&editor.document, &editor.cursor),
            editor.selection.as_ref().map(|selection| MonacoSelection {
                anchor: position_to_monaco(&editor.document, &selection.anchor),
                focus: position_to_monaco(&editor.document, &selection.focus),
            }),
            editor.file_stamp.as_ref().map(RecoveryFileStamp::from),
            tab_index,
            self.model.active_editor.document_id == id,
        )))
    }

    pub fn restore_recovery_documents(
        &mut self,
        recovered: Vec<(RecoverySnapshot, Option<LoadedFile>)>,
    ) -> Result<(WorkspaceSnapshot, Vec<(String, u64)>, Vec<String>), BridgeError> {
        let mut associations = Vec::with_capacity(recovered.len());
        let mut warnings = Vec::new();
        let mut active_view = None;
        let mut first_recovery = true;

        for (recovery, loaded_file) in recovered {
            let language = language_from_protocol(recovery.language);
            let recovery_text = normalize_newlines(&recovery.text);
            let should_activate = first_recovery || recovery.is_active;
            first_recovery = false;
            let loaded_file = match loaded_file {
                Some(file)
                    if recovery
                        .identity_path
                        .as_deref()
                        .is_some_and(|identity_path| {
                            !same_identity_path(identity_path, &file.identity_path)
                        }) =>
                {
                    warnings.push(format!(
                        "{} now resolves to a different file and was recovered as an untitled document.",
                        recovery.title
                    ));
                    None
                }
                file => file,
            };
            let has_source_file = loaded_file.is_some();
            let view_id = if let Some(mut loaded_file) = loaded_file {
                loaded_file.language = language;
                if let Some(stamp) = recovery.file_stamp() {
                    if loaded_file.stamp != stamp {
                        warnings.push(format!(
                            "{} changed on disk after its recovery snapshot. The recovery was kept as an unsaved buffer.",
                            recovery.title
                        ));
                    }
                    loaded_file.stamp = stamp;
                }
                self.model.open_loaded_file(loaded_file, should_activate)
            } else {
                warnings.push(format!(
                    "{} was recovered as an untitled document because its source file is unavailable.",
                    recovery.title
                ));
                self.model
                    .create_untitled_document("", language, should_activate)
            };
            let document_id = self
                .model
                .editor_for_view(view_id)
                .ok_or_else(|| document_not_found(view_id.0))?
                .document_id;
            if has_source_file && let Some(stamp) = recovery.file_stamp() {
                if let Some(editor) = self.model.editor_for_document_mut(document_id) {
                    editor.file_stamp = Some(stamp);
                }
            }
            let current_text = self
                .model
                .editor_for_document(document_id)
                .ok_or_else(|| document_not_found(document_id.0))?
                .document
                .text();
            let cursor = match position_from_monaco(&recovery_text, recovery.cursor) {
                Ok(cursor) => cursor,
                Err(_) => {
                    warnings.push(format!(
                        "The saved cursor position for {} was invalid and was reset.",
                        recovery.title
                    ));
                    Position::default()
                }
            };
            let selection = recovery.selection.and_then(|selection| {
                let anchor = position_from_monaco(&recovery_text, selection.anchor).ok()?;
                let focus = position_from_monaco(&recovery_text, selection.focus).ok()?;
                Some(Selection { anchor, focus })
            });
            if recovery.selection.is_some() && selection.is_none() {
                warnings.push(format!(
                    "The saved selection for {} was invalid and was cleared.",
                    recovery.title
                ));
            }
            if current_text != recovery_text {
                let editor = self
                    .model
                    .editor_for_document(document_id)
                    .ok_or_else(|| document_not_found(document_id.0))?;
                let last_line = editor.document.line_count().saturating_sub(1);
                let end = Position {
                    line: last_line,
                    column: editor.document.line_len(last_line),
                };
                self.model = mvu::update(
                    self.model.clone(),
                    Message::ApplyDocumentEdits {
                        document_id,
                        edits: vec![TextEdit {
                            start: Position::default(),
                            end,
                            text: recovery_text.clone(),
                        }],
                        cursor: cursor.clone(),
                        selection: selection.clone(),
                        reconcile_dirty: false,
                    },
                )
                .model;
            }
            self.model = mvu::update(
                self.model.clone(),
                Message::SetEditorSelection {
                    document_id,
                    cursor,
                    selection,
                },
            )
            .model;
            if recovery.is_active {
                active_view = Some(view_id);
            }
            associations.push((recovery.recovery_id, document_id.0));
        }

        if let Some(view_id) = active_view {
            self.model.activate_view(view_id);
        }
        self.register_documents();
        Ok((self.snapshot()?, associations, warnings))
    }

    pub fn begin_open_workspace(&mut self, path: PathBuf) -> Result<PathBuf, BridgeError> {
        self.ensure_workspace_switch_documents_clean()?;
        let transition = mvu::update(self.model.clone(), Message::OpenWorkspaceRequested(path));
        let requested_path = transition
            .commands
            .iter()
            .find_map(|command| match command {
                Command::OpenWorkspace(path) => Some(path.clone()),
                _ => None,
            });
        self.model = transition.model;
        requested_path.ok_or_else(|| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The workspace-open effect was not produced.",
            )
        })
    }

    pub fn complete_open_workspace(
        &mut self,
        result: Result<Workspace, WorkspaceError>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let error = result.as_ref().err().cloned();
        let opened = error.is_none();
        self.model = mvu::update(self.model.clone(), Message::WorkspaceOpened(result)).model;
        if let Some(error) = error {
            return Err(workspace_error_to_bridge(error));
        }
        if opened {
            self.revisions
                .retain(|document_id, _| self.model.editor_for_document(*document_id).is_some());
            self.register_documents();
        }
        self.snapshot()
    }

    pub fn close_workspace(&mut self) -> Result<WorkspaceSnapshot, BridgeError> {
        if self.model.workspace.is_none() {
            return Err(BridgeError::invalid_request("No workspace is open."));
        }
        self.ensure_workspace_switch_documents_clean()?;
        self.model = mvu::update(self.model.clone(), Message::CloseWorkspace).model;
        self.revisions
            .retain(|document_id, _| self.model.editor_for_document(*document_id).is_some());
        self.register_documents();
        self.snapshot()
    }

    pub fn request_workspace_directory(
        &mut self,
        relative_path: PathBuf,
    ) -> Result<(PathBuf, PathBuf), BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        let transition = mvu::update(
            self.model.clone(),
            Message::LoadWorkspaceDirectoryRequested {
                root: root.clone(),
                relative_path: relative_path.clone(),
            },
        );
        let requested = transition.commands.iter().any(|command| {
            matches!(
                command,
                Command::ReadWorkspaceDirectory {
                    root: command_root,
                    relative_path: command_path,
                } if command_root == &root && command_path == &relative_path
            )
        });
        self.model = transition.model;
        if requested {
            Ok((root, relative_path))
        } else {
            Err(BridgeError::invalid_request(
                "The requested directory is not an unloaded directory in the active workspace.",
            ))
        }
    }

    pub fn complete_workspace_directory(
        &mut self,
        root: PathBuf,
        relative_path: PathBuf,
        result: Result<Vec<WorkspaceEntry>, WorkspaceError>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        if !self
            .model
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.root == root)
        {
            return self.snapshot();
        }
        let error = result.as_ref().err().cloned();
        self.model = mvu::update(
            self.model.clone(),
            Message::WorkspaceDirectoryLoaded {
                root,
                relative_path,
                result,
            },
        )
        .model;
        if let Some(error) = error {
            return Err(workspace_error_to_bridge(error));
        }
        self.snapshot()
    }

    pub fn workspace_file_path(&self, relative_path: &str) -> Result<PathBuf, BridgeError> {
        let workspace = self
            .model
            .workspace
            .as_ref()
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        let relative_path = PathBuf::from(relative_path);
        let path = workspace_io::resolve_file(&workspace.root, &relative_path)
            .map_err(workspace_error_to_bridge)?;
        if !workspace.contains_file(&path) {
            return Err(workspace_error_to_bridge(WorkspaceError::InvalidPath(
                relative_path,
            )));
        }
        Ok(path)
    }

    pub fn workspace_search_result_path(
        &self,
        relative_path: &str,
    ) -> Result<PathBuf, BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        workspace_io::resolve_file(root, PathBuf::from(relative_path))
            .map_err(workspace_error_to_bridge)
    }

    pub fn workspace_search_result_is_open(
        &self,
        relative_path: &str,
    ) -> Result<bool, BridgeError> {
        let path = self.workspace_search_result_path(relative_path)?;
        Ok(self.model.view_for_path(&path).is_some())
    }

    pub fn search_inputs(
        &self,
    ) -> Result<(Option<PathBuf>, Vec<SearchDocumentInput>), BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone());
        let editors =
            std::iter::once(&self.model.active_editor).chain(self.model.inactive_editors.iter());
        let mut documents = Vec::new();
        for editor in editors {
            let path = editor.identity_path.clone();
            if root
                .as_ref()
                .is_some_and(|root| path.as_ref().is_none_or(|path| !path.starts_with(root)))
            {
                continue;
            }
            let revision = self
                .revisions
                .get(&editor.document_id)
                .copied()
                .ok_or_else(|| document_not_found(editor.document_id.0))?;
            let title = editor
                .file_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| format!("Untitled {}", editor.document_id.0));
            documents.push(SearchDocumentInput {
                document_id: editor.document_id.0,
                revision,
                title,
                path,
                text: editor.document.text(),
            });
        }
        Ok((root, documents))
    }

    pub fn navigate_to_path(
        &mut self,
        path: &PathBuf,
        line_number: u32,
        column: u32,
        match_length: u32,
        expected_fingerprint: Option<&str>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let view_id = self.model.view_for_path(path).ok_or_else(|| {
            BridgeError::new(
                BridgeErrorCode::DocumentNotFound,
                "The search result document is no longer open.",
            )
        })?;
        let editor = self
            .model
            .editor_for_view(view_id)
            .ok_or_else(|| document_not_found(view_id.0))?;
        let document_id = editor.document_id;
        let source = editor.document.text();
        if expected_fingerprint
            .is_some_and(|expected| workspace_io::search_fingerprint(&source) != expected)
        {
            return Err(BridgeError::new(
                BridgeErrorCode::StaleRevision,
                "The search result is stale because the file changed after the search.",
            ));
        }
        self.navigate_to_view_search_result(
            view_id,
            document_id,
            &source,
            line_number,
            column,
            match_length,
        )
    }

    pub fn navigate_to_document_search_result(
        &mut self,
        document_id: u64,
        expected_revision: u64,
        line_number: u32,
        column: u32,
        match_length: u32,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let document_id = DocumentId(document_id);
        self.current_revision(document_id, expected_revision)?;
        let (view_id, source) = self
            .model
            .editor_for_document(document_id)
            .map(|editor| (editor.view_id, editor.document.text()))
            .ok_or_else(|| document_not_found(document_id.0))?;
        self.navigate_to_view_search_result(
            view_id,
            document_id,
            &source,
            line_number,
            column,
            match_length,
        )
    }

    fn navigate_to_view_search_result(
        &mut self,
        view_id: EditorViewId,
        document_id: DocumentId,
        source: &str,
        line_number: u32,
        column: u32,
        match_length: u32,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let start = position_from_monaco(
            source,
            MonacoPosition {
                line_number,
                column,
            },
        )?;
        let end = position_from_monaco(
            source,
            MonacoPosition {
                line_number,
                column: column.checked_add(match_length).ok_or_else(|| {
                    BridgeError::invalid_request("The search result range is too large.")
                })?,
            },
        )?;
        self.model = mvu::update(self.model.clone(), Message::ActivateEditorView(view_id)).model;
        self.model = mvu::update(
            self.model.clone(),
            Message::SetEditorSelection {
                document_id,
                cursor: end.clone(),
                selection: Some(functor_core::model::Selection {
                    anchor: start,
                    focus: end,
                }),
            },
        )
        .model;
        self.snapshot()
    }

    pub fn create_document(
        &mut self,
        language: DocumentLanguage,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let syntax_language = language_from_protocol(language);
        let transition = mvu::update(
            self.model.clone(),
            Message::NewUntitledDocument {
                text: String::new(),
                language: syntax_language,
            },
        );
        self.model = transition.model;
        self.register_documents();
        self.snapshot()
    }

    pub fn set_document_language(
        &mut self,
        request: SetDocumentLanguageRequest,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let document_id = DocumentId(request.document_id);
        self.current_revision(document_id, request.expected_revision)?;
        self.model = mvu::update(
            self.model.clone(),
            Message::SetDocumentLanguage {
                document_id,
                language: request.language.into_syntax(),
            },
        )
        .model;
        self.snapshot()
    }

    pub fn activate_view(&mut self, view_id: u64) -> Result<WorkspaceSnapshot, BridgeError> {
        if !self.model.activate_view(EditorViewId(view_id)) {
            if self.model.active_editor.view_id != EditorViewId(view_id) {
                return Err(BridgeError::new(
                    BridgeErrorCode::DocumentNotFound,
                    "The requested editor view does not exist.",
                ));
            }
        }
        self.snapshot()
    }

    pub fn close_document(
        &mut self,
        request: CloseDocumentRequest,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let view_id = EditorViewId(request.view_id);
        let editor = self
            .model
            .editor_for_view(view_id)
            .ok_or_else(|| document_not_found(request.view_id))?;
        self.current_revision(editor.document_id, request.expected_revision)?;
        if editor.document.dirty && !request.discard_changes {
            return Err(BridgeError::invalid_request(
                "The document has unsaved changes and requires an explicit discard decision.",
            ));
        }

        self.model = mvu::update(self.model.clone(), Message::CloseEditorView(view_id)).model;
        self.revisions
            .retain(|document_id, _| self.model.editor_for_document(*document_id).is_some());
        self.register_documents();
        self.snapshot()
    }

    pub fn apply_edits(
        &mut self,
        request: ApplyDocumentEditsRequest,
    ) -> Result<ApplyDocumentEditsResponse, BridgeError> {
        let document_id = DocumentId(request.document_id);
        let revision = self.current_revision(document_id, request.expected_revision)?;
        let editor = self
            .model
            .editor_for_document(document_id)
            .ok_or_else(|| document_not_found(request.document_id))?;
        let source = editor.document.text();
        validate_text_changes(&request.changes, &source)?;

        let mut edits = Vec::with_capacity(request.changes.len());
        for change in &request.changes {
            let start_offset = utf16_offset_to_char_index(&source, change.range_offset as usize)?;
            let end_offset = utf16_offset_to_char_index(
                &source,
                change.range_offset as usize + change.range_length as usize,
            )?;
            edits.push(TextEdit {
                start: char_offset_to_position(&source, start_offset)?,
                end: char_offset_to_position(&source, end_offset)?,
                text: change.text.clone(),
            });
        }
        let updated_text = apply_text_changes(&source, &request.changes)?;
        let cursor = position_from_monaco(&updated_text, request.cursor)?;
        let selection = request
            .selection
            .map(|selection| {
                Ok(Selection {
                    anchor: position_from_monaco(&updated_text, selection.anchor)?,
                    focus: position_from_monaco(&updated_text, selection.focus)?,
                })
            })
            .transpose()?;

        let message = if updated_text == source {
            Message::SetEditorSelection {
                document_id,
                cursor,
                selection,
            }
        } else {
            Message::ApplyDocumentEdits {
                document_id,
                edits,
                cursor,
                selection,
                reconcile_dirty: true,
            }
        };
        let transition = mvu::update(self.model.clone(), message);
        self.model = transition.model;
        if updated_text != source {
            let next_revision = revision.checked_add(1).ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::Internal,
                    "The document revision counter is exhausted.",
                )
            })?;
            self.revisions.insert(document_id, next_revision);
        }
        let editor = self
            .model
            .editor_for_document(document_id)
            .ok_or_else(|| document_not_found(request.document_id))?;
        Ok(ApplyDocumentEditsResponse {
            document_id: document_id.0,
            revision: self.revisions[&document_id],
            dirty: editor.document.dirty,
        })
    }

    pub fn update_selection(
        &mut self,
        request: UpdateSelectionRequest,
    ) -> Result<UpdateSelectionResponse, BridgeError> {
        let document_id = DocumentId(request.document_id);
        let revision = self.current_revision(document_id, request.expected_revision)?;
        let editor = self
            .model
            .editor_for_document(document_id)
            .ok_or_else(|| document_not_found(request.document_id))?;
        let text = editor.document.text();
        let cursor = position_from_monaco(&text, request.cursor)?;
        let selection = request
            .selection
            .map(|selection| {
                Ok(Selection {
                    anchor: position_from_monaco(&text, selection.anchor)?,
                    focus: position_from_monaco(&text, selection.focus)?,
                })
            })
            .transpose()?;
        self.model = mvu::update(
            self.model.clone(),
            Message::SetEditorSelection {
                document_id,
                cursor,
                selection,
            },
        )
        .model;
        Ok(UpdateSelectionResponse {
            document_id: document_id.0,
            revision,
        })
    }

    pub fn begin_open(&mut self, path: PathBuf) -> Option<OpenTicket> {
        let transition = mvu::update(self.model.clone(), Message::OpenFileRequested(path.clone()));
        let request_id = transition
            .commands
            .iter()
            .find_map(|command| match command {
                Command::OpenFile { request_id, .. } => Some(*request_id),
                _ => None,
            });
        self.model = transition.model;
        request_id.map(|request_id| OpenTicket {
            request_id,
            path,
            workspace_root: None,
            require_loaded_tree_entry: false,
        })
    }

    pub fn begin_workspace_file_open(
        &mut self,
        relative_path: &str,
    ) -> Result<(PathBuf, Option<OpenTicket>), BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        let path = self.workspace_file_path(relative_path)?;
        let ticket = self.begin_open(path.clone()).map(|mut ticket| {
            ticket.workspace_root = Some(root);
            ticket.require_loaded_tree_entry = true;
            ticket
        });
        Ok((path, ticket))
    }

    pub fn begin_workspace_search_result_open(
        &mut self,
        relative_path: &str,
    ) -> Result<(PathBuf, Option<OpenTicket>), BridgeError> {
        let root = self
            .model
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        let path = self.workspace_search_result_path(relative_path)?;
        let ticket = self.begin_open(path.clone()).map(|mut ticket| {
            ticket.workspace_root = Some(root);
            ticket
        });
        Ok((path, ticket))
    }

    pub fn begin_reopen_closed_document(
        &mut self,
    ) -> Result<Option<(PathBuf, OpenTicket)>, BridgeError> {
        if self.model.most_recently_closed_path().is_none() {
            return Err(BridgeError::invalid_request(
                "There are no recently closed documents to reopen.",
            ));
        }
        let transition = mvu::update(self.model.clone(), Message::ReopenClosedDocumentRequested);
        let requested = transition
            .commands
            .iter()
            .find_map(|command| match command {
                Command::OpenFile { request_id, path } => Some((*request_id, path.clone())),
                _ => None,
            });
        self.model = transition.model;
        requested
            .map(|(request_id, path)| {
                (
                    path.clone(),
                    OpenTicket {
                        request_id,
                        path,
                        workspace_root: None,
                        require_loaded_tree_entry: false,
                    },
                )
            })
            .map(Some)
            .ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::Internal,
                    "The recently closed document could not be queued for opening.",
                )
            })
    }

    pub fn clear_recent_documents(&mut self) -> Result<WorkspaceSnapshot, BridgeError> {
        self.model = mvu::update(self.model.clone(), Message::ClearRecentlyClosedDocuments).model;
        self.snapshot()
    }

    pub fn complete_open(
        &mut self,
        ticket: OpenTicket,
        result: Result<LoadedFile, FileError>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        if let Some(root) = ticket.workspace_root.as_ref() {
            let workspace = self.model.workspace.as_ref();
            let workspace_changed = workspace.is_none_or(|workspace| &workspace.root != root);
            let result_escaped_workspace = result.as_ref().is_ok_and(|file| {
                !same_identity_path(&ticket.path, &file.identity_path)
                    || !file.identity_path.starts_with(root)
                    || (ticket.require_loaded_tree_entry
                        && workspace
                            .is_none_or(|workspace| !workspace.contains_file(&file.identity_path)))
            });
            if workspace_changed || result_escaped_workspace {
                self.model.complete_open_request(ticket.request_id);
                return Err(BridgeError::invalid_request(
                    "The workspace or file identity changed while the file was opening; the file was not opened.",
                ));
            }
        }
        let load_error = result.as_ref().err().cloned();
        self.model = mvu::update(
            self.model.clone(),
            Message::FileOpened {
                request_id: ticket.request_id,
                result,
            },
        )
        .model;
        if self.model.view_for_path(&ticket.path).is_none() {
            if let Some(error) = load_error {
                return Err(file_error_to_bridge(error));
            }
            return self.snapshot();
        }
        self.register_documents();
        self.snapshot()
    }

    pub fn document_path(
        &self,
        document_id: u64,
        expected_revision: u64,
    ) -> Result<Option<PathBuf>, BridgeError> {
        let id = DocumentId(document_id);
        self.current_revision(id, expected_revision)?;
        self.model
            .editor_for_document(id)
            .map(|editor| editor.file_path.clone())
            .ok_or_else(|| document_not_found(document_id))
    }

    pub fn begin_save(
        &self,
        document_id: u64,
        expected_revision: u64,
        path: Option<PathBuf>,
    ) -> Result<SaveTicket, BridgeError> {
        let id = DocumentId(document_id);
        self.current_revision(id, expected_revision)?;
        let transition = mvu::update(
            self.model.clone(),
            Message::SaveDocumentRequested {
                document_id: id,
                path: path.clone(),
            },
        );
        let Some(Command::SaveFile {
            path,
            text,
            expected_stamp,
            ..
        }) = transition.commands.into_iter().find(|command| {
            matches!(
                command,
                Command::SaveFile {
                    document_id: queued_document,
                    ..
                } if *queued_document == id
            )
        })
        else {
            return Err(BridgeError::invalid_request(
                "A save destination is required for this untitled document.",
            ));
        };
        Ok(SaveTicket {
            document_id: id,
            saved_text: normalize_newlines(&text),
            path,
            expected_stamp,
        })
    }

    pub fn ensure_save_destination_available(
        &self,
        document_id: u64,
        identity_path: &Path,
    ) -> Result<(), BridgeError> {
        let document_id = DocumentId(document_id);
        if self.model.editor_for_document(document_id).is_none() {
            return Err(document_not_found(document_id.0));
        }
        let is_open_in_another_document = std::iter::once(&self.model.active_editor)
            .chain(self.model.inactive_editors.iter())
            .any(|editor| {
                editor.document_id != document_id
                    && editor
                        .identity_path
                        .as_deref()
                        .is_some_and(|open_path| same_identity_path(open_path, identity_path))
            });
        if is_open_in_another_document {
            return Err(BridgeError::invalid_request(
                "The save destination is already open in another document.",
            ));
        }
        Ok(())
    }

    pub fn save_ticket_path(ticket: &SaveTicket) -> &PathBuf {
        &ticket.path
    }

    pub fn save_ticket_text(ticket: &SaveTicket) -> &str {
        &ticket.saved_text
    }

    pub fn save_ticket_stamp(ticket: &SaveTicket) -> Option<&FileStamp> {
        ticket.expected_stamp.as_ref()
    }

    pub fn complete_save(
        &mut self,
        ticket: SaveTicket,
        result: Result<SavedFile, FileError>,
    ) -> Result<WorkspaceSnapshot, BridgeError> {
        let save_error = result.as_ref().err().cloned();
        self.model = mvu::update(
            self.model.clone(),
            Message::FileSaved {
                document_id: ticket.document_id,
                saved_text: ticket.saved_text,
                result,
            },
        )
        .model;
        if let Some(error) = save_error {
            return Err(file_error_to_bridge(error));
        }
        self.register_documents();
        self.snapshot()
    }

    fn current_revision(
        &self,
        document_id: DocumentId,
        expected_revision: u64,
    ) -> Result<u64, BridgeError> {
        let current = self
            .revisions
            .get(&document_id)
            .copied()
            .ok_or_else(|| document_not_found(document_id.0))?;
        if self.model.editor_for_document(document_id).is_none() {
            return Err(document_not_found(document_id.0));
        }
        if current != expected_revision {
            return Err(BridgeError::new(
                BridgeErrorCode::StaleRevision,
                format!(
                    "The document revision is stale: expected {expected_revision}, current revision is {current}."
                ),
            ));
        }
        Ok(current)
    }

    fn register_documents(&mut self) {
        for editor in
            std::iter::once(&self.model.active_editor).chain(self.model.inactive_editors.iter())
        {
            self.revisions.entry(editor.document_id).or_insert(0);
        }
    }
}

fn language_from_protocol(language: DocumentLanguage) -> SyntaxLanguage {
    language.into_syntax()
}

fn language_to_protocol(language: SyntaxLanguage) -> DocumentLanguage {
    DocumentLanguage::from_syntax(language)
}

fn workspace_snapshot(workspace: &Workspace) -> Result<WorkspaceStateSnapshot, BridgeError> {
    Ok(WorkspaceStateSnapshot {
        root: display_workspace_root(&workspace.root),
        name: workspace
            .root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| workspace.root.to_string_lossy().into_owned()),
        entries: workspace
            .entries
            .iter()
            .map(|entry| workspace_entry_snapshot(&workspace.root, entry))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn display_workspace_root(root: &std::path::Path) -> String {
    let root = root.to_string_lossy();
    #[cfg(windows)]
    {
        if let Some(unc_path) = root.strip_prefix("\\\\?\\UNC\\") {
            return format!("\\\\{unc_path}");
        }

        if let Some(path) = root.strip_prefix("\\\\?\\") {
            return path.to_owned();
        }
    }
    root.into_owned()
}

fn same_identity_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn workspace_entry_snapshot(
    root: &std::path::Path,
    entry: &WorkspaceEntry,
) -> Result<WorkspaceEntrySnapshot, BridgeError> {
    let relative_path = entry
        .path
        .strip_prefix(root)
        .map_err(|_| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "A workspace entry is outside its workspace root.",
            )
        })?
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    let children = entry
        .children
        .as_ref()
        .map(|children| {
            children
                .iter()
                .map(|child| workspace_entry_snapshot(root, child))
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()?;
    Ok(WorkspaceEntrySnapshot {
        name: entry.name.clone(),
        relative_path,
        kind: match entry.kind {
            WorkspaceEntryKind::File => ProtocolEntryKind::File,
            WorkspaceEntryKind::Directory => ProtocolEntryKind::Directory,
        },
        children,
    })
}

fn workspace_error_to_bridge(error: WorkspaceError) -> BridgeError {
    BridgeError::new(BridgeErrorCode::FileIo, error.to_string())
}

fn position_to_monaco(
    document: &functor_core::model::Document,
    position: &Position,
) -> MonacoPosition {
    let position = document.clamp_position(position.clone());
    let line_text = document.line_text(position.line);
    let column = line_text
        .chars()
        .take(position.column)
        .map(char::len_utf16)
        .sum::<usize>();
    MonacoPosition {
        line_number: position.line as u32 + 1,
        column: column as u32 + 1,
    }
}

fn position_from_monaco(text: &str, position: MonacoPosition) -> Result<Position, BridgeError> {
    if position.line_number == 0 || position.column == 0 {
        return Err(BridgeError::invalid_request(
            "Editor positions use one-based line and column numbers.",
        ));
    }
    let line = text
        .split('\n')
        .nth(position.line_number as usize - 1)
        .ok_or_else(|| {
            BridgeError::invalid_request("The editor position is beyond the document.")
        })?;
    let line = line.strip_suffix('\r').unwrap_or(line);
    let column = utf16_offset_to_char_index(line, position.column as usize - 1)?;
    Ok(Position {
        line: position.line_number as usize - 1,
        column,
    })
}

fn utf16_offset_to_char_index(text: &str, target: usize) -> Result<usize, BridgeError> {
    let mut units = 0usize;
    for (index, character) in text.chars().enumerate() {
        if units == target {
            return Ok(index);
        }
        units += character.len_utf16();
        if units > target {
            return Err(BridgeError::invalid_request(
                "An editor position splits a Unicode character.",
            ));
        }
    }
    if units == target {
        Ok(text.chars().count())
    } else {
        Err(BridgeError::invalid_request(
            "An editor position is beyond the line.",
        ))
    }
}

fn char_offset_to_position(text: &str, target: usize) -> Result<Position, BridgeError> {
    let mut line = 0usize;
    let mut column = 0usize;
    let mut chars = text.chars().enumerate().peekable();
    while let Some((index, character)) = chars.next() {
        if index == target {
            return Ok(Position { line, column });
        }
        match character {
            '\r' if chars.peek().is_some_and(|(_, next)| *next == '\n') => {
                if index + 1 == target {
                    return Err(BridgeError::invalid_request(
                        "An edit range cannot split a CRLF line ending.",
                    ));
                }
                chars.next();
                line += 1;
                column = 0;
            }
            '\n' | '\r' => {
                line += 1;
                column = 0;
            }
            _ => column += 1,
        }
    }
    if text.chars().count() == target {
        Ok(Position { line, column })
    } else {
        Err(BridgeError::invalid_request(
            "An edit range is beyond the document.",
        ))
    }
}

fn apply_text_changes(source: &str, changes: &[MonacoTextChange]) -> Result<String, BridgeError> {
    let mut ranges = changes
        .iter()
        .map(|change| {
            let start = utf16_offset_to_char_index(source, change.range_offset as usize)?;
            let end = utf16_offset_to_char_index(
                source,
                change.range_offset as usize + change.range_length as usize,
            )?;
            Ok((start, end, change.text.as_str()))
        })
        .collect::<Result<Vec<_>, BridgeError>>()?;
    ranges.sort_by(|left, right| right.0.cmp(&left.0));
    let mut updated = source.to_owned();
    for (start, end, replacement) in ranges {
        let start_byte = char_to_byte_offset(&updated, start);
        let end_byte = char_to_byte_offset(&updated, end);
        updated.replace_range(start_byte..end_byte, replacement);
    }
    Ok(updated)
}

fn char_to_byte_offset(text: &str, char_offset: usize) -> usize {
    text.char_indices()
        .nth(char_offset)
        .map_or(text.len(), |(byte_offset, _)| byte_offset)
}

fn document_not_found(document_id: u64) -> BridgeError {
    BridgeError::new(
        BridgeErrorCode::DocumentNotFound,
        format!("Document {document_id} does not exist."),
    )
}

pub fn file_error_to_bridge(error: FileError) -> BridgeError {
    match error {
        FileError::Read { path, message } | FileError::Write { path, message } => BridgeError::new(
            BridgeErrorCode::FileIo,
            format!("{}: {message}", path.display()),
        ),
        FileError::InvalidUtf8 { path } => BridgeError::new(
            BridgeErrorCode::FileIo,
            format!("{} is not valid UTF-8 text.", path.display()),
        ),
        FileError::Modified { path } => BridgeError::new(
            BridgeErrorCode::FileIo,
            format!(
                "{} was modified outside Functor; reload it before saving.",
                path.display()
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::EditorSession;
    use crate::protocol::{
        ApplyDocumentEditsRequest, BridgeErrorCode, CloseDocumentRequest, DocumentLanguage,
        MonacoPosition, MonacoSelection, MonacoTextChange, SetDocumentLanguageRequest,
        UpdateSelectionRequest,
    };
    use crate::recovery_io::{RecoveryFileStamp, RecoverySnapshot};
    use crate::workspace_io;
    use functor_core::file_io::{FileError, FileStamp, LoadedFile, SavedFile};
    use functor_core::model::DocumentId;
    use functor_core::syntax::SyntaxLanguage;
    use functor_core::workspace::{Workspace, WorkspaceEntry, WorkspaceEntryKind};
    use std::path::PathBuf;

    #[test]
    fn new_documents_keep_the_requested_monaco_language_in_snapshots() {
        let mut session = EditorSession::default();
        let snapshot = session.create_document(DocumentLanguage::FSharp).unwrap();

        assert!(
            snapshot
                .documents
                .iter()
                .any(|document| document.language == DocumentLanguage::FSharp)
        );
    }

    #[test]
    fn changing_language_updates_only_the_document_mode() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        let language =
            DocumentLanguage::from_syntax(SyntaxLanguage::from_monaco_id("typescript").unwrap());
        let snapshot = session
            .set_document_language(SetDocumentLanguageRequest {
                document_id: document.document_id,
                expected_revision: document.revision,
                language,
            })
            .unwrap();
        let updated = snapshot
            .documents
            .iter()
            .find(|entry| entry.document_id == document.document_id)
            .unwrap();

        assert_eq!(updated.language, language);
        assert_eq!(updated.revision, document.revision);
        assert!(
            session
                .model
                .editor_for_document(DocumentId(document.document_id))
                .unwrap()
                .document
                .language_overridden
        );
    }

    #[test]
    fn search_inputs_choose_open_documents_or_workspace_files_automatically() {
        let root = test_workspace_root("search-scopes");
        let file_path = root.join("main.rs");
        std::fs::write(&file_path, "hello").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let file_path = std::fs::canonicalize(file_path).unwrap();
        let mut session = EditorSession::default();
        let first_document = session.snapshot().unwrap().documents[0].clone();
        session
            .apply_edits(edit_request(
                first_document.document_id,
                first_document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "first".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 6,
                },
            ))
            .unwrap();
        let second_snapshot = session
            .create_document(DocumentLanguage::PlainText)
            .unwrap();
        let second_document = second_snapshot
            .documents
            .iter()
            .find(|document| document.document_id != first_document.document_id)
            .unwrap()
            .clone();

        let (open_root, open_documents) = session.search_inputs().unwrap();
        assert!(open_root.is_none());
        assert_eq!(open_documents.len(), 2);
        assert!(
            open_documents
                .iter()
                .any(|document| document.document_id == second_document.document_id)
        );

        let mut workspace_session = EditorSession::default();
        workspace_session.request_workspace_picker().unwrap();
        workspace_session
            .begin_open_workspace(root.clone())
            .unwrap();
        workspace_session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();
        let ticket = workspace_session.begin_open(file_path.clone()).unwrap();
        workspace_session
            .complete_open(
                ticket,
                Ok(LoadedFile {
                    path: file_path.clone(),
                    identity_path: file_path.clone(),
                    text: "hello".into(),
                    language: SyntaxLanguage::Rust,
                    stamp: FileStamp {
                        length: 5,
                        modified: None,
                    },
                }),
            )
            .unwrap();

        assert!(
            workspace_session
                .workspace_search_result_is_open("main.rs")
                .unwrap()
        );
        std::fs::write(root.join("closed.rs"), "closed").unwrap();
        assert!(
            !workspace_session
                .workspace_search_result_is_open("closed.rs")
                .unwrap()
        );

        let (workspace_root, workspace_documents) = workspace_session.search_inputs().unwrap();
        assert_eq!(workspace_root.as_deref(), Some(root.as_path()));
        assert_eq!(workspace_documents.len(), 1);
        assert_eq!(workspace_documents[0].title, "main.rs");
        assert_eq!(
            workspace_documents[0].path.as_deref(),
            Some(file_path.as_path())
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn search_navigation_rejects_a_stale_document_revision() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();

        let error = session
            .navigate_to_document_search_result(
                document.document_id,
                document.revision + 1,
                1,
                1,
                1,
            )
            .unwrap_err();

        assert_eq!(error.code, BridgeErrorCode::StaleRevision);
    }

    #[cfg(windows)]
    #[test]
    fn display_workspace_root_hides_the_windows_extended_path_prefix() {
        assert_eq!(
            super::display_workspace_root(std::path::Path::new(r"\\?\C:\projects")),
            r"C:\projects"
        );
        assert_eq!(
            super::display_workspace_root(std::path::Path::new(r"\\?\UNC\server\share")),
            r"\\server\share"
        );
    }

    #[test]
    fn workspace_effects_round_trip_a_lazy_file_tree_through_the_model() {
        let mut session = EditorSession::default();
        session.request_workspace_picker().unwrap();
        let root = PathBuf::from("project");
        session.begin_open_workspace(root.clone()).unwrap();
        let snapshot = session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: vec![WorkspaceEntry {
                    name: "src".into(),
                    path: root.join("src"),
                    kind: WorkspaceEntryKind::Directory,
                    children: None,
                }],
            }))
            .unwrap();
        assert_eq!(snapshot.workspace.as_ref().unwrap().name, "project");
        assert_eq!(
            snapshot.workspace.as_ref().unwrap().entries[0].relative_path,
            "src"
        );
        assert_eq!(
            snapshot.workspace.as_ref().unwrap().entries[0].children,
            None
        );

        assert_eq!(
            session
                .request_workspace_directory(PathBuf::from("src"))
                .unwrap(),
            (root.clone(), PathBuf::from("src"))
        );
        let snapshot = session
            .complete_workspace_directory(
                root.clone(),
                PathBuf::from("src"),
                Ok(vec![WorkspaceEntry {
                    name: "main.rs".into(),
                    path: root.join("src").join("main.rs"),
                    kind: WorkspaceEntryKind::File,
                    children: None,
                }]),
            )
            .unwrap();
        assert_eq!(
            snapshot.workspace.as_ref().unwrap().entries[0]
                .children
                .as_ref()
                .unwrap()[0]
                .relative_path,
            "src/main.rs"
        );
    }

    #[test]
    fn workspace_file_open_requires_a_loaded_tree_entry_and_a_relative_path() {
        let root = test_workspace_root("loaded-tree-membership");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("README.md"), "readme").unwrap();
        std::fs::write(root.join("src/main.rs"), "main").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let workspace = workspace_io::discover(&root).unwrap();
        let mut session = EditorSession::default();
        session.begin_open_workspace(root.clone()).unwrap();
        session.complete_open_workspace(Ok(workspace)).unwrap();

        assert!(session.workspace_file_path("src/main.rs").is_err());
        assert!(session.workspace_file_path("../outside.rs").is_err());
        assert_eq!(
            session.workspace_file_path("README.md").unwrap(),
            root.join("README.md")
        );

        let (directory_root, relative_path) = session
            .request_workspace_directory(PathBuf::from("src"))
            .unwrap();
        let entries = workspace_io::read_directory(&directory_root, &relative_path).unwrap();
        session
            .complete_workspace_directory(directory_root, relative_path, Ok(entries))
            .unwrap();
        assert_eq!(
            session.workspace_file_path("src/main.rs").unwrap(),
            root.join("src/main.rs")
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_open_completion_is_rejected_after_replacing_its_root() {
        let first_root = test_workspace_root("stale-workspace-open");
        let second_root = test_workspace_root("replacement-workspace-open");
        let file_path = first_root.join("main.rs");
        std::fs::write(&file_path, "main").unwrap();
        let first_root = std::fs::canonicalize(first_root).unwrap();
        let second_root = std::fs::canonicalize(second_root).unwrap();
        let mut session = EditorSession::default();
        session.begin_open_workspace(first_root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(workspace_io::discover(&first_root).unwrap()))
            .unwrap();
        let (_, ticket) = session.begin_workspace_file_open("main.rs").unwrap();
        let ticket = ticket.expect("file should require an asynchronous load");

        session.begin_open_workspace(second_root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(workspace_io::discover(&second_root).unwrap()))
            .unwrap();

        assert_eq!(
            session
                .complete_open(ticket, crate::file_io::load(&file_path))
                .unwrap_err()
                .code,
            BridgeErrorCode::InvalidRequest
        );
        assert_eq!(session.workspace_root(), Some(second_root.clone()));
        assert_eq!(session.snapshot().unwrap().documents.len(), 1);

        std::fs::remove_dir_all(first_root).unwrap();
        std::fs::remove_dir_all(second_root).unwrap();
    }

    #[test]
    fn workspace_open_rejects_a_file_identity_outside_its_ticket() {
        let root = test_workspace_root("workspace-open-identity");
        let external_root = test_workspace_root("workspace-open-identity-external");
        let workspace_path = root.join("main.rs");
        let external_path = external_root.join("main.rs");
        std::fs::write(&workspace_path, "inside").unwrap();
        std::fs::write(&external_path, "outside").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let external_path = std::fs::canonicalize(external_path).unwrap();
        let mut session = EditorSession::default();
        session.begin_open_workspace(root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(workspace_io::discover(&root).unwrap()))
            .unwrap();
        let (_, ticket) = session.begin_workspace_file_open("main.rs").unwrap();
        let ticket = ticket.expect("the tree entry should start an open request");

        assert_eq!(
            session
                .complete_open(ticket, crate::file_io::load(&external_path))
                .unwrap_err()
                .code,
            BridgeErrorCode::InvalidRequest
        );
        assert_eq!(session.snapshot().unwrap().documents.len(), 1);

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(external_root).unwrap();
    }

    #[test]
    fn workspace_switch_rejects_dirty_documents_without_mutating_the_session() {
        let first_root = test_workspace_root("workspace-replace-first");
        let second_root = test_workspace_root("workspace-replace-second");
        let first_root = std::fs::canonicalize(first_root).unwrap();
        let second_root = std::fs::canonicalize(second_root).unwrap();
        let mut session = EditorSession::default();
        session.begin_open_workspace(first_root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(Workspace {
                root: first_root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();
        let document = session.snapshot().unwrap().documents[0].clone();
        let edited = session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "keep this work".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 15,
                },
            ))
            .unwrap();

        let error = session
            .begin_open_workspace(second_root.clone())
            .unwrap_err();
        let unchanged = session.snapshot().unwrap();
        assert_eq!(error.code, BridgeErrorCode::InvalidRequest);
        assert_eq!(
            unchanged.workspace.as_ref().unwrap().root,
            super::display_workspace_root(&first_root)
        );
        assert!(unchanged.documents.iter().any(|document| {
            document.document_id == edited.document_id
                && document.text == "keep this work"
                && document.dirty
        }));

        std::fs::remove_dir_all(first_root).unwrap();
        std::fs::remove_dir_all(second_root).unwrap();
    }

    #[test]
    fn closing_workspace_rejects_dirty_documents_and_clears_clean_tabs() {
        let root = test_workspace_root("close-workspace");
        let mut dirty_session = EditorSession::default();
        dirty_session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();
        let document = dirty_session.snapshot().unwrap().documents[0].clone();
        dirty_session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "unsaved".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 8,
                },
            ))
            .unwrap();

        let error = dirty_session.close_workspace().unwrap_err();
        assert_eq!(error.code, BridgeErrorCode::InvalidRequest);
        assert_eq!(
            dirty_session
                .snapshot()
                .unwrap()
                .workspace
                .as_ref()
                .unwrap()
                .root,
            super::display_workspace_root(&root)
        );

        let mut clean_session = EditorSession::default();
        clean_session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();
        let snapshot = clean_session.close_workspace().unwrap();

        assert!(snapshot.workspace.is_none());
        assert_eq!(snapshot.documents.len(), 1);
        assert!(!snapshot.documents[0].dirty);
        assert_eq!(snapshot.documents[0].text, "");
        assert!(snapshot.documents[0].title.starts_with("Untitled"));
        assert!(snapshot.documents[0].is_untitled);
        assert_eq!(snapshot.can_reopen_closed_document, false);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn successful_workspace_switch_closes_tabs_and_clears_recent_history() {
        let first_root = test_workspace_root("workspace-switch-first");
        let second_root = test_workspace_root("workspace-switch-second");
        let path = first_root.join("main.rs");
        std::fs::write(&path, "saved").unwrap();
        let first_root = std::fs::canonicalize(first_root).unwrap();
        let second_root = std::fs::canonicalize(second_root).unwrap();
        let mut session = EditorSession::default();
        session.begin_open_workspace(first_root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(workspace_io::discover(&first_root).unwrap()))
            .unwrap();
        let ticket = session.begin_open(path.clone()).unwrap();
        let opened = session
            .complete_open(ticket, crate::file_io::load(&path))
            .unwrap();
        let document = opened
            .documents
            .iter()
            .find(|document| document.view_id == opened.active_view_id)
            .unwrap()
            .clone();
        session
            .close_document(CloseDocumentRequest {
                view_id: document.view_id,
                expected_revision: document.revision,
                discard_changes: false,
            })
            .unwrap();
        let before_switch = session.snapshot().unwrap();
        assert!(before_switch.can_reopen_closed_document);
        let old_document_id = before_switch.documents[0].document_id;

        session.begin_open_workspace(second_root.clone()).unwrap();
        let switched = session
            .complete_open_workspace(Ok(workspace_io::discover(&second_root).unwrap()))
            .unwrap();

        assert_eq!(
            switched.workspace.as_ref().unwrap().root,
            super::display_workspace_root(&second_root)
        );
        assert_eq!(switched.documents.len(), 1);
        assert_ne!(switched.documents[0].document_id, old_document_id);
        assert_eq!(switched.documents[0].text, "");
        assert!(!switched.documents[0].dirty);
        assert!(!switched.can_reopen_closed_document);

        std::fs::remove_dir_all(first_root).unwrap();
        std::fs::remove_dir_all(second_root).unwrap();
    }

    #[test]
    fn recently_closed_saved_document_reopens_with_its_cursor() {
        let root = test_workspace_root("recently-closed");
        let path = root.join("main.rs");
        std::fs::write(&path, "first\nsecond").unwrap();
        let mut session = EditorSession::default();
        let ticket = session.begin_open(path.clone()).unwrap();
        let opened = session
            .complete_open(ticket, crate::file_io::load(&path))
            .unwrap();
        let document = opened
            .documents
            .iter()
            .find(|document| document.view_id == opened.active_view_id)
            .unwrap()
            .clone();
        session
            .update_selection(UpdateSelectionRequest {
                document_id: document.document_id,
                expected_revision: document.revision,
                cursor: MonacoPosition {
                    line_number: 2,
                    column: 4,
                },
                selection: None,
            })
            .unwrap();
        session
            .close_document(CloseDocumentRequest {
                view_id: document.view_id,
                expected_revision: document.revision,
                discard_changes: false,
            })
            .unwrap();
        assert!(session.snapshot().unwrap().can_reopen_closed_document);

        let (reopen_path, ticket) = session
            .begin_reopen_closed_document()
            .unwrap()
            .expect("a recently closed file should be available");
        let reopened = session
            .complete_open(ticket, crate::file_io::load(&reopen_path))
            .unwrap();
        let active = reopened
            .documents
            .iter()
            .find(|document| document.view_id == reopened.active_view_id)
            .unwrap();
        assert_eq!(active.title, "main.rs");
        assert_eq!(active.cursor.line_number, 2);
        assert_eq!(active.cursor.column, 4);
        assert!(!reopened.can_reopen_closed_document);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn clear_recent_documents_removes_the_reopen_command_target() {
        let root = test_workspace_root("recent-history-cleared");
        let path = root.join("main.rs");
        std::fs::write(&path, "text").unwrap();
        let mut session = EditorSession::default();
        let ticket = session.begin_open(path.clone()).unwrap();
        let opened = session
            .complete_open(ticket, crate::file_io::load(&path))
            .unwrap();
        let document = opened.documents[0].clone();
        session
            .close_document(CloseDocumentRequest {
                view_id: document.view_id,
                expected_revision: document.revision,
                discard_changes: false,
            })
            .unwrap();
        assert!(session.snapshot().unwrap().can_reopen_closed_document);

        let cleared = session.clear_recent_documents().unwrap();
        assert!(!cleared.can_reopen_closed_document);
        assert!(session.begin_reopen_closed_document().is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_recent_reopen_keeps_the_entry_available_for_retry() {
        let root = test_workspace_root("recently-closed-missing");
        let path = root.join("missing.rs");
        std::fs::write(&path, "text").unwrap();
        let mut session = EditorSession::default();
        let ticket = session.begin_open(path.clone()).unwrap();
        let opened = session
            .complete_open(ticket, crate::file_io::load(&path))
            .unwrap();
        let document = opened.documents[0].clone();
        session
            .close_document(CloseDocumentRequest {
                view_id: document.view_id,
                expected_revision: document.revision,
                discard_changes: false,
            })
            .unwrap();
        std::fs::remove_file(&path).unwrap();

        let (reopen_path, ticket) = session
            .begin_reopen_closed_document()
            .unwrap()
            .expect("the missing file should remain in recent history");
        assert!(
            session
                .complete_open(ticket, crate::file_io::load(&reopen_path))
                .is_err()
        );
        assert!(session.snapshot().unwrap().can_reopen_closed_document);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn external_files_remain_open_but_are_not_saved_as_workspace_tabs() {
        let root = test_workspace_root("external-workspace");
        let external_root = test_workspace_root("external-workspace-file");
        let external_path = external_root.join("outside.rs");
        std::fs::write(root.join("inside.rs"), "inside").unwrap();
        std::fs::write(&external_path, "outside").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let external_root = std::fs::canonicalize(external_root).unwrap();
        let external_path = external_root.join("outside.rs");
        let mut session = EditorSession::default();
        session.begin_open_workspace(root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(workspace_io::discover(&root).unwrap()))
            .unwrap();

        let ticket = session.begin_open(external_path.clone()).unwrap();
        let snapshot = session
            .complete_open(ticket, crate::file_io::load(&external_path))
            .unwrap();

        assert_eq!(snapshot.documents.len(), 1);
        assert_eq!(snapshot.documents[0].title, "outside.rs");
        assert!(
            session
                .workspace_session_state()
                .unwrap()
                .open_files
                .is_empty()
        );
        let escape_path = PathBuf::from("..")
            .join(external_root.file_name().unwrap())
            .join("outside.rs");
        assert!(
            session
                .workspace_file_path(&escape_path.to_string_lossy())
                .is_err()
        );

        std::fs::remove_dir_all(root).unwrap();
        std::fs::remove_dir_all(external_root).unwrap();
    }

    #[test]
    fn workspace_session_state_preserves_open_tab_order_and_active_file() {
        let root = test_workspace_root("session-state");
        let first_path = root.join("first.rs");
        let second_path = root.join("second.rs");
        std::fs::write(&first_path, "first").unwrap();
        std::fs::write(&second_path, "second").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let mut session = EditorSession::default();
        session.request_workspace_picker().unwrap();
        session.begin_open_workspace(root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();

        for path in [root.join("first.rs"), root.join("second.rs")] {
            let ticket = session.begin_open(path.clone()).unwrap();
            let loaded = LoadedFile {
                path: path.clone(),
                identity_path: path.clone(),
                text: std::fs::read_to_string(&path).unwrap(),
                language: SyntaxLanguage::Rust,
                stamp: FileStamp {
                    length: 5,
                    modified: None,
                },
            };
            session.complete_open(ticket, Ok(loaded)).unwrap();
        }

        assert_eq!(
            session.workspace_session_state().unwrap(),
            super::WorkspaceSessionState {
                root: root.clone(),
                open_files: vec!["first.rs".into(), "second.rs".into()],
                active_file: Some("second.rs".into()),
            }
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restoring_workspace_files_preserves_order_and_selects_saved_active_file() {
        let root = test_workspace_root("session-restore");
        let first_path = root.join("first.rs");
        let second_path = root.join("second.rs");
        std::fs::write(&first_path, "first").unwrap();
        std::fs::write(&second_path, "second").unwrap();
        let root = std::fs::canonicalize(root).unwrap();
        let mut session = EditorSession::default();
        session.request_workspace_picker().unwrap();
        session.begin_open_workspace(root.clone()).unwrap();
        session
            .complete_open_workspace(Ok(Workspace {
                root: root.clone(),
                entries: Vec::new(),
            }))
            .unwrap();
        let files = ["first.rs", "second.rs"]
            .into_iter()
            .map(|relative_path| {
                let path = root.join(relative_path);
                LoadedFile {
                    text: std::fs::read_to_string(&path).unwrap(),
                    path: path.clone(),
                    identity_path: path,
                    language: SyntaxLanguage::Rust,
                    stamp: FileStamp {
                        length: 5,
                        modified: None,
                    },
                }
            })
            .collect();

        let snapshot = session
            .restore_workspace_documents(files, Some("second.rs".into()))
            .unwrap();

        assert_eq!(
            snapshot
                .documents
                .iter()
                .map(|document| document.title.as_str())
                .collect::<Vec<_>>(),
            vec!["first.rs", "second.rs"]
        );
        assert_eq!(
            snapshot
                .documents
                .iter()
                .find(|document| document.view_id == snapshot.active_view_id)
                .unwrap()
                .title,
            "second.rs"
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_snapshot_requires_the_current_dirty_revision() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        let edited = session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "recovered".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 10,
                },
            ))
            .unwrap();

        let snapshot = session
            .recovery_snapshot(
                document.document_id,
                edited.revision,
                "untitled-1-1".into(),
                "session-1".into(),
                1,
            )
            .unwrap()
            .unwrap();

        assert_eq!(snapshot.text, "recovered");
        assert_eq!(snapshot.revision, edited.revision);
        assert!(snapshot.is_active);
        assert!(
            session
                .recovery_snapshot(
                    document.document_id,
                    document.revision,
                    "untitled-1-1".into(),
                    "session-1".into(),
                    2,
                )
                .is_err()
        );
    }

    #[test]
    fn restoring_recovery_replaces_disk_text_and_remains_dirty() {
        let root = test_workspace_root("recovery-session");
        let path = root.join("main.rs");
        std::fs::write(&path, "disk text").unwrap();
        let loaded = crate::file_io::load(&path).unwrap();
        let path = loaded.identity_path.clone();
        let recovery = RecoverySnapshot::new(
            "file-1".into(),
            "session-1".into(),
            1,
            "main.rs".into(),
            Some(path.clone()),
            Some(path.clone()),
            "recovered text".into(),
            DocumentLanguage::Rust,
            1,
            MonacoPosition {
                line_number: 1,
                column: 5,
            },
            None,
            Some(RecoveryFileStamp::from(&loaded.stamp)),
            0,
            true,
        );
        let mut session = EditorSession::default();
        let ticket = session.begin_open(path.clone()).unwrap();
        session.complete_open(ticket, Ok(loaded.clone())).unwrap();

        let (snapshot, associations, warnings) = session
            .restore_recovery_documents(vec![(recovery, Some(loaded))])
            .unwrap();

        assert!(warnings.is_empty());
        assert_eq!(associations.len(), 1);
        let document = snapshot
            .documents
            .iter()
            .find(|document| document.document_id == associations[0].1)
            .unwrap();
        assert_eq!(document.text, "recovered text");
        assert!(document.dirty);
        assert_eq!(document.cursor.column, 5);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_recovery_source_is_restored_as_dirty_untitled_document() {
        let recovery = RecoverySnapshot::new(
            "file-2".into(),
            "session-1".into(),
            1,
            "missing.rs".into(),
            Some(PathBuf::from("C:\\project\\missing.rs")),
            Some(PathBuf::from("C:\\project\\missing.rs")),
            "unsaved contents".into(),
            DocumentLanguage::Rust,
            1,
            MonacoPosition {
                line_number: 1,
                column: 1,
            },
            None,
            Some(RecoveryFileStamp::from(&FileStamp {
                length: 0,
                modified: None,
            })),
            0,
            true,
        );
        let mut session = EditorSession::default();

        let (snapshot, _, warnings) = session
            .restore_recovery_documents(vec![(recovery, None)])
            .unwrap();

        assert_eq!(warnings.len(), 1);
        let document = snapshot
            .documents
            .iter()
            .find(|document| document.view_id == snapshot.active_view_id)
            .unwrap();
        assert_eq!(document.text, "unsaved contents");
        assert!(document.dirty);
        assert!(document.title.starts_with("Untitled"));
    }

    #[test]
    fn edits_are_revisioned_and_replayed_requests_are_rejected() {
        let mut session = EditorSession::default();
        let first = session.snapshot().unwrap().documents[0].clone();
        let response = session
            .apply_edits(edit_request(
                first.document_id,
                0,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "// ".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 4,
                },
            ))
            .unwrap();
        assert_eq!(response.revision, 1);
        let mut undo = edit_request(
            first.document_id,
            1,
            vec![MonacoTextChange {
                range_offset: 0,
                range_length: 3,
                text: String::new(),
            }],
            MonacoPosition {
                line_number: 1,
                column: 1,
            },
        );
        undo.is_undoing = true;
        let undone = session.apply_edits(undo).unwrap();
        assert_eq!(undone.revision, 2);
        assert!(!undone.dirty);
        assert_eq!(
            session
                .apply_edits(edit_request(
                    first.document_id,
                    0,
                    vec![MonacoTextChange {
                        range_offset: 0,
                        range_length: 0,
                        text: "// ".into(),
                    }],
                    MonacoPosition {
                        line_number: 1,
                        column: 4,
                    },
                ))
                .unwrap_err()
                .code,
            BridgeErrorCode::StaleRevision
        );
    }

    #[test]
    fn maps_monaco_utf16_edits_and_selection_to_rust_scalar_positions() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        session
            .model
            .editor_for_document_mut(DocumentId(document.document_id))
            .unwrap()
            .document = functor_core::model::Document::from_text("a🦀b", SyntaxLanguage::PlainText);
        let response = session
            .apply_edits(edit_request(
                document.document_id,
                0,
                vec![MonacoTextChange {
                    range_offset: 1,
                    range_length: 2,
                    text: "界".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 4,
                },
            ))
            .unwrap();
        let editor = session
            .model
            .editor_for_document(DocumentId(document.document_id))
            .unwrap();

        assert_eq!(editor.document.text(), "a界b");
        assert_eq!(editor.cursor.column, 3);
        assert_eq!(
            editor.selection,
            Some(functor_core::model::Selection {
                anchor: functor_core::model::Position { line: 0, column: 3 },
                focus: functor_core::model::Position { line: 0, column: 3 },
            })
        );
        assert_eq!(response.revision, 1);
    }

    #[test]
    fn maps_unicode_positions_across_surrogate_pairs_and_combining_marks() {
        let text = "a🦀e\u{301}z";
        let document = functor_core::model::Document::from_text(text, SyntaxLanguage::PlainText);

        assert_eq!(
            super::position_from_monaco(
                text,
                MonacoPosition {
                    line_number: 1,
                    column: 4,
                },
            )
            .unwrap(),
            functor_core::model::Position { line: 0, column: 2 }
        );
        assert_eq!(
            super::position_from_monaco(
                text,
                MonacoPosition {
                    line_number: 1,
                    column: 5,
                },
            )
            .unwrap(),
            functor_core::model::Position { line: 0, column: 3 }
        );
        assert_eq!(
            super::position_from_monaco(
                text,
                MonacoPosition {
                    line_number: 1,
                    column: 6,
                },
            )
            .unwrap(),
            functor_core::model::Position { line: 0, column: 4 }
        );
        assert!(
            super::position_from_monaco(
                text,
                MonacoPosition {
                    line_number: 1,
                    column: 3,
                },
            )
            .is_err()
        );
        assert_eq!(
            super::position_to_monaco(
                &document,
                &functor_core::model::Position { line: 0, column: 3 },
            ),
            MonacoPosition {
                line_number: 1,
                column: 5,
            }
        );
    }

    #[test]
    fn cursor_selection_updates_preserve_content_revision() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "first\nsecond".into(),
                }],
                MonacoPosition {
                    line_number: 2,
                    column: 7,
                },
            ))
            .unwrap();
        let response = session
            .update_selection(UpdateSelectionRequest {
                document_id: document.document_id,
                expected_revision: 1,
                cursor: MonacoPosition {
                    line_number: 2,
                    column: 5,
                },
                selection: Some(MonacoSelection {
                    anchor: MonacoPosition {
                        line_number: 1,
                        column: 1,
                    },
                    focus: MonacoPosition {
                        line_number: 2,
                        column: 5,
                    },
                }),
            })
            .unwrap();
        let editor = session
            .model
            .editor_for_document(DocumentId(document.document_id))
            .unwrap();

        assert_eq!(response.revision, 1);
        assert_eq!(
            editor.cursor,
            functor_core::model::Position { line: 1, column: 4 }
        );
        assert_eq!(
            editor.selection,
            Some(functor_core::model::Selection {
                anchor: functor_core::model::Position { line: 0, column: 0 },
                focus: functor_core::model::Position { line: 1, column: 4 },
            })
        );
        assert_eq!(
            session
                .update_selection(UpdateSelectionRequest {
                    document_id: document.document_id,
                    expected_revision: 0,
                    cursor: MonacoPosition {
                        line_number: 1,
                        column: 1,
                    },
                    selection: None,
                })
                .unwrap_err()
                .code,
            BridgeErrorCode::StaleRevision
        );
    }

    #[test]
    fn normal_edits_returning_to_the_saved_snapshot_clear_dirty_state() {
        let mut session = EditorSession::default();
        let path = PathBuf::from("saved.rs");
        let ticket = session.begin_open(path.clone()).unwrap();
        session
            .complete_open(
                ticket,
                Ok(LoadedFile {
                    path: path.clone(),
                    identity_path: path,
                    text: "saved".into(),
                    language: SyntaxLanguage::Rust,
                    stamp: FileStamp {
                        length: 5,
                        modified: None,
                    },
                }),
            )
            .unwrap();
        let document = session
            .snapshot()
            .unwrap()
            .documents
            .into_iter()
            .find(|document| document.title == "saved.rs")
            .unwrap();

        let inserted = session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 5,
                    range_length: 0,
                    text: "!".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 7,
                },
            ))
            .unwrap();
        assert!(inserted.dirty);
        let restored = session
            .apply_edits(edit_request(
                document.document_id,
                inserted.revision,
                vec![MonacoTextChange {
                    range_offset: 5,
                    range_length: 1,
                    text: String::new(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 6,
                },
            ))
            .unwrap();

        assert!(!restored.dirty);
    }

    #[test]
    fn save_as_updates_identity_and_uses_normalized_saved_text() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        session
            .model
            .editor_for_document_mut(DocumentId(document.document_id))
            .unwrap()
            .document =
            functor_core::model::Document::from_text("first\r\nsecond", SyntaxLanguage::PlainText);
        let directory = test_workspace_root("save-as");
        let path = directory.join("notes.md");
        let identity_path = crate::file_io::canonical_identity_path(&path).unwrap();
        let ticket = session
            .begin_save(document.document_id, document.revision, Some(path.clone()))
            .unwrap();

        assert_eq!(EditorSession::save_ticket_text(&ticket), "first\nsecond");
        session
            .complete_save(
                ticket,
                Ok(SavedFile {
                    path: path.clone(),
                    identity_path: identity_path.clone(),
                    stamp: FileStamp {
                        length: 12,
                        modified: None,
                    },
                }),
            )
            .unwrap();

        let saved = session.snapshot().unwrap();
        let saved_document = saved
            .documents
            .iter()
            .find(|open| open.document_id == document.document_id)
            .unwrap();
        assert_eq!(saved_document.title, "notes.md");
        assert_eq!(saved_document.language, DocumentLanguage::Markdown);
        assert!(saved_document.dirty);
        assert_eq!(
            session.document_path(document.document_id, document.revision),
            Ok(Some(path.clone()))
        );
        assert!(
            session.begin_open(path.clone()).is_none(),
            "opening a file after Save As must reactivate the saved document"
        );

        let another = session
            .create_document(DocumentLanguage::PlainText)
            .unwrap()
            .documents
            .into_iter()
            .find(|open| open.document_id != document.document_id)
            .unwrap();
        assert!(
            session
                .ensure_save_destination_available(another.document_id, &identity_path)
                .is_err()
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_save_keeps_the_dirty_document_and_its_original_identity() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        let edited = session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "work in progress".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 17,
                },
            ))
            .unwrap();
        let destination = PathBuf::from("failed-save.rs");
        let ticket = session
            .begin_save(
                document.document_id,
                edited.revision,
                Some(destination.clone()),
            )
            .unwrap();

        assert_eq!(
            session
                .complete_save(
                    ticket,
                    Err(FileError::Write {
                        path: destination,
                        message: "simulated write failure".into(),
                    }),
                )
                .unwrap_err()
                .code,
            BridgeErrorCode::FileIo
        );
        let snapshot = session.snapshot().unwrap();
        let current = snapshot
            .documents
            .iter()
            .find(|open| open.document_id == document.document_id)
            .unwrap();
        assert_eq!(current.text, "work in progress");
        assert!(current.dirty);
        assert!(
            session
                .model
                .editor_for_document(DocumentId(document.document_id))
                .unwrap()
                .identity_path
                .is_none()
        );
    }

    #[test]
    fn async_open_and_save_results_remain_attached_to_their_documents() {
        let mut session = EditorSession::default();
        let mut first = session.snapshot().unwrap().documents[0].clone();
        first.revision = session
            .apply_edits(edit_request(
                first.document_id,
                first.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "first".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 6,
                },
            ))
            .unwrap()
            .revision;
        let second = session
            .create_document(DocumentLanguage::Markdown)
            .unwrap()
            .documents
            .into_iter()
            .find(|document| document.document_id != first.document_id)
            .unwrap();
        let path = PathBuf::from("opened.rs");
        let ticket = session.begin_open(path.clone()).unwrap();
        let duplicate_ticket = ticket.clone();
        session.activate_view(first.view_id).unwrap();
        let loaded = LoadedFile {
            path: path.clone(),
            identity_path: path.clone(),
            text: "opened".into(),
            language: SyntaxLanguage::Rust,
            stamp: FileStamp {
                length: 6,
                modified: None,
            },
        };
        session.complete_open(ticket, Ok(loaded.clone())).unwrap();
        let duplicate_snapshot = session.complete_open(duplicate_ticket, Ok(loaded)).unwrap();
        assert_eq!(duplicate_snapshot.documents.len(), 3);
        let snapshot = session.snapshot().unwrap();
        assert_eq!(snapshot.active_view_id, first.view_id);
        assert!(
            snapshot
                .documents
                .iter()
                .any(|document| document.title == "opened.rs")
        );

        let save_ticket = session
            .begin_save(
                first.document_id,
                first.revision,
                Some(PathBuf::from("first.rs")),
            )
            .unwrap();
        session
            .apply_edits(edit_request(
                first.document_id,
                first.revision,
                vec![MonacoTextChange {
                    range_offset: 0,
                    range_length: 0,
                    text: "changed ".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 9,
                },
            ))
            .unwrap();
        session.activate_view(second.view_id).unwrap();
        session
            .complete_save(
                save_ticket,
                Ok(SavedFile {
                    path: PathBuf::from("first.rs"),
                    identity_path: PathBuf::from("first.rs"),
                    stamp: FileStamp {
                        length: 1,
                        modified: None,
                    },
                }),
            )
            .unwrap();
        let first_after_save = session
            .snapshot()
            .unwrap()
            .documents
            .into_iter()
            .find(|document| document.document_id == first.document_id)
            .unwrap();
        assert!(first_after_save.dirty);
        assert_eq!(first_after_save.revision, 2);
    }

    #[test]
    fn dirty_document_close_requires_explicit_discard_and_removes_only_that_tab() {
        let mut session = EditorSession::default();
        let document = session.snapshot().unwrap().documents[0].clone();
        let insertion_offset = document.text.encode_utf16().count() as u32;
        let dirty = session
            .apply_edits(edit_request(
                document.document_id,
                document.revision,
                vec![MonacoTextChange {
                    range_offset: insertion_offset,
                    range_length: 0,
                    text: "!".into(),
                }],
                MonacoPosition {
                    line_number: 1,
                    column: 1,
                },
            ))
            .unwrap();

        let request = CloseDocumentRequest {
            view_id: document.view_id,
            expected_revision: dirty.revision,
            discard_changes: false,
        };
        assert_eq!(
            session.close_document(request.clone()).unwrap_err().code,
            BridgeErrorCode::InvalidRequest
        );
        assert!(
            session
                .snapshot()
                .unwrap()
                .documents
                .iter()
                .any(|open| open.view_id == document.view_id && open.dirty)
        );

        let closed = session
            .close_document(CloseDocumentRequest {
                discard_changes: true,
                ..request
            })
            .unwrap();
        assert!(
            !closed
                .documents
                .iter()
                .any(|open| open.view_id == document.view_id)
        );
        assert_eq!(closed.documents.len(), 1);
    }

    fn edit_request(
        document_id: u64,
        expected_revision: u64,
        changes: Vec<MonacoTextChange>,
        cursor: MonacoPosition,
    ) -> ApplyDocumentEditsRequest {
        ApplyDocumentEditsRequest {
            document_id,
            expected_revision,
            changes,
            is_undoing: false,
            is_redoing: false,
            cursor,
            selection: Some(MonacoSelection {
                anchor: cursor,
                focus: cursor,
            }),
        }
    }

    fn test_workspace_root(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("functors-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
