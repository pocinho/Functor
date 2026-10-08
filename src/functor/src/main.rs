#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use functor::file_io;
use functor::protocol::{
    AppSettings, ApplyDocumentEditsRequest, BridgeError, BridgeErrorCode, CancelSearchRequest,
    CloseDocumentRequest, NewDocumentRequest, OpenDocumentRequest, OpenWorkspaceRequest,
    OpenWorkspaceSearchResultRequest, RecoveryCandidatesResponse, RecoveryRestoreResponse,
    ReplaceWorkspaceSearchResultsRequest, ReplaceWorkspaceSearchResultsResponse,
    RestoreWorkspaceResponse, SaveDocumentRequest, SaveRecoverySnapshotRequest,
    SaveRecoverySnapshotResponse, SaveSettingsRequest, SaveUserThemeRequest,
    SearchWorkspaceRequest, SetDocumentLanguageRequest, SettingsCatalogResponse,
    SwitchDocumentRequest, UpdateSelectionRequest, UpdateSelectionResponse, UserThemeRequest,
    UserThemeSummary, WorkspaceDirectoryRequest, WorkspaceFileRequest, WorkspaceSearchResponse,
    WorkspaceSnapshot,
};
use functor::recovery_io;
use functor::session::EditorSession;
use functor::settings_io;
use functor::workspace_io;
use tauri::{Manager, State};

const MAX_RECOVERY_RESPONSE_WARNINGS: usize = 20;

struct AppState(
    Mutex<EditorSession>,
    Mutex<Option<workspace_io::WorkspaceSessionState>>,
    OnceLock<PathBuf>,
    OnceLock<PathBuf>,
    Mutex<RecoveryRegistry>,
    Mutex<HashMap<String, Arc<AtomicBool>>>,
);

struct RecoveryRegistry {
    session_id: String,
    document_ids: HashMap<u64, String>,
    generations: HashMap<String, u64>,
}

impl RecoveryRegistry {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        Self {
            session_id: format!("session-{}-{timestamp}", std::process::id()),
            document_ids: HashMap::new(),
            generations: HashMap::new(),
        }
    }

    fn recovery_id(&mut self, document_id: u64, path: Option<&Path>) -> String {
        if let Some(recovery_id) = self.document_ids.get(&document_id) {
            return recovery_id.clone();
        }
        let recovery_id = path.map_or_else(
            || format!("untitled-{}-{document_id}", std::process::id()),
            stable_recovery_id,
        );
        self.document_ids.insert(document_id, recovery_id.clone());
        recovery_id
    }

    fn next_generation(&mut self, recovery_id: &str) -> Result<u64, BridgeError> {
        let generation = self.generations.entry(recovery_id.to_owned()).or_default();
        *generation = generation.checked_add(1).ok_or_else(|| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The recovery snapshot generation is exhausted.",
            )
        })?;
        Ok(*generation)
    }
}

fn stable_recovery_id(path: &Path) -> String {
    let mut hash = 0x6c62272e07bb014262b821756295c58du128;
    for byte in path.to_string_lossy().to_lowercase().bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000000001000000000000000000013bu128);
    }
    format!("file-{hash:032x}")
}

fn session_lock<'a>(
    state: &'a State<'_, AppState>,
) -> Result<MutexGuard<'a, EditorSession>, BridgeError> {
    state.0.lock().map_err(|_| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "The editor session is unavailable because its state lock was poisoned.",
        )
    })
}

fn recovery_lock<'a>(
    state: &'a State<'_, AppState>,
) -> Result<MutexGuard<'a, RecoveryRegistry>, BridgeError> {
    state.4.lock().map_err(|_| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "Recovery state is unavailable because its lock was poisoned.",
        )
    })
}

fn recovery_directory(state: &State<'_, AppState>) -> Result<PathBuf, BridgeError> {
    state.3.get().cloned().ok_or_else(|| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "The recovery directory was not initialized.",
        )
    })
}

fn delete_recovery_for_document(
    state: &State<'_, AppState>,
    document_id: u64,
) -> Result<Option<String>, BridgeError> {
    let mut registry = recovery_lock(state)?;
    let Some(recovery_id) = registry.document_ids.remove(&document_id) else {
        return Ok(None);
    };
    registry.next_generation(&recovery_id)?;
    Ok(
        recovery_io::delete_snapshot(recovery_directory(state)?, &recovery_id)
            .err()
            .map(|error| error.to_string()),
    )
}

fn persist_workspace_snapshot(
    state: &State<'_, AppState>,
    mut snapshot: WorkspaceSnapshot,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let saved_state = session_lock(state)?.workspace_session_state();
    let warning = if let Some(saved_state) = saved_state {
        let state_path = state.2.get().ok_or_else(|| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The workspace state path was not initialized.",
            )
        })?;
        workspace_io::save_last_workspace(state_path, &saved_state)
            .err()
            .map(|error| format!("Workspace state could not be saved: {error}"))
    } else {
        None
    };
    if let Some(warning) = warning {
        snapshot.persistence_warning = Some(match snapshot.persistence_warning {
            Some(existing) => format!("{existing} {warning}"),
            None => warning,
        });
    }
    Ok(snapshot)
}

#[tauri::command]
fn get_snapshot(state: State<'_, AppState>) -> Result<WorkspaceSnapshot, BridgeError> {
    session_lock(&state)?.snapshot()
}

#[tauri::command]
fn new_document(
    state: State<'_, AppState>,
    request: NewDocumentRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let snapshot = session_lock(&state)?.create_document(request.language)?;
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
fn set_document_language(
    state: State<'_, AppState>,
    request: SetDocumentLanguageRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let snapshot = session_lock(&state)?.set_document_language(request)?;
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
async fn reopen_closed_document(
    state: State<'_, AppState>,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let Some((path, ticket)) = session_lock(&state)?.begin_reopen_closed_document()? else {
        return Err(BridgeError::new(
            BridgeErrorCode::Internal,
            "The recently closed document was not queued.",
        ));
    };
    load_open_ticket(state, path, ticket).await
}

#[tauri::command]
fn clear_recent_documents(state: State<'_, AppState>) -> Result<WorkspaceSnapshot, BridgeError> {
    let snapshot = session_lock(&state)?.clear_recent_documents()?;
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
fn switch_document(
    state: State<'_, AppState>,
    request: SwitchDocumentRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let snapshot = session_lock(&state)?.activate_view(request.view_id)?;
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
fn close_document(
    state: State<'_, AppState>,
    request: CloseDocumentRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let (document_id, snapshot) = {
        let mut session = session_lock(&state)?;
        let document_id = session.document_id_for_view(request.view_id)?;
        (document_id, session.close_document(request)?)
    };
    let mut snapshot = snapshot;
    if let Some(warning) = delete_recovery_for_document(&state, document_id)? {
        snapshot.persistence_warning =
            Some(format!("Recovery data could not be cleared: {warning}"));
    }
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
fn apply_document_edits(
    state: State<'_, AppState>,
    request: ApplyDocumentEditsRequest,
) -> Result<functor::protocol::ApplyDocumentEditsResponse, BridgeError> {
    session_lock(&state)?.apply_edits(request)
}

#[tauri::command]
fn save_recovery_snapshot(
    state: State<'_, AppState>,
    request: SaveRecoverySnapshotRequest,
) -> Result<SaveRecoverySnapshotResponse, BridgeError> {
    let mut registry = recovery_lock(&state)?;
    let identity_path = match session_lock(&state)?
        .recovery_identity_path(request.document_id, request.expected_revision)
    {
        Ok(path) => path,
        Err(error) if error.code == BridgeErrorCode::StaleRevision => {
            return Ok(SaveRecoverySnapshotResponse {
                saved_revision: None,
                stale: true,
            });
        }
        Err(error) => return Err(error),
    };
    let recovery_id = registry.recovery_id(request.document_id, identity_path.as_deref());
    let generation = registry.next_generation(&recovery_id)?;
    let snapshot = match session_lock(&state)?.recovery_snapshot(
        request.document_id,
        request.expected_revision,
        recovery_id.clone(),
        registry.session_id.clone(),
        generation,
    ) {
        Ok(snapshot) => snapshot,
        Err(error) if error.code == BridgeErrorCode::StaleRevision => {
            return Ok(SaveRecoverySnapshotResponse {
                saved_revision: None,
                stale: true,
            });
        }
        Err(error) => return Err(error),
    };
    let Some(snapshot) = snapshot else {
        let warning = recovery_io::delete_snapshot(recovery_directory(&state)?, &recovery_id)
            .err()
            .map(|error| error.to_string());
        registry.document_ids.remove(&request.document_id);
        if let Some(warning) = warning {
            return Err(BridgeError::new(
                BridgeErrorCode::FileIo,
                format!("Recovery data could not be cleared: {warning}"),
            ));
        }
        return Ok(SaveRecoverySnapshotResponse {
            saved_revision: None,
            stale: false,
        });
    };

    recovery_io::save_snapshot(recovery_directory(&state)?, &snapshot)
        .map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))?;
    Ok(SaveRecoverySnapshotResponse {
        saved_revision: Some(snapshot.revision),
        stale: false,
    })
}

#[tauri::command]
fn list_recovery_snapshots(
    state: State<'_, AppState>,
) -> Result<RecoveryCandidatesResponse, BridgeError> {
    let _registry = recovery_lock(&state)?;
    let (snapshots, warnings) = recovery_io::list_snapshots(recovery_directory(&state)?)
        .map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))?;
    Ok(RecoveryCandidatesResponse {
        candidates: snapshots.iter().map(recovery_io::candidate).collect(),
        warnings,
    })
}

#[tauri::command]
fn restore_recovery_snapshots(
    state: State<'_, AppState>,
) -> Result<RecoveryRestoreResponse, BridgeError> {
    let mut registry = recovery_lock(&state)?;
    let (snapshots, mut warnings) = recovery_io::list_snapshots(recovery_directory(&state)?)
        .map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))?;
    let mut recovered = Vec::with_capacity(snapshots.len());
    for snapshot in snapshots {
        let loaded_file = snapshot
            .path
            .as_ref()
            .and_then(|path| match file_io::load(path) {
                Ok(file) => Some(file),
                Err(error) => {
                    warnings.push(format!(
                        "{} could not be reattached to its source file: {}",
                        snapshot.title,
                        functor::session::file_error_to_bridge(error).message
                    ));
                    None
                }
            });
        recovered.push((snapshot, loaded_file));
    }

    let (snapshot, associations, restore_warnings) =
        session_lock(&state)?.restore_recovery_documents(recovered)?;
    warnings.extend(restore_warnings);
    for (recovery_id, document_id) in associations {
        let is_dirty = snapshot
            .documents
            .iter()
            .find(|document| document.document_id == document_id)
            .is_some_and(|document| document.dirty);
        if is_dirty {
            registry.document_ids.insert(document_id, recovery_id);
        } else if let Err(error) =
            recovery_io::delete_snapshot(recovery_directory(&state)?, &recovery_id)
        {
            warnings.push(format!("Recovered data could not be cleared: {error}"));
        }
    }
    if warnings.len() > MAX_RECOVERY_RESPONSE_WARNINGS {
        let omitted = warnings.len() - (MAX_RECOVERY_RESPONSE_WARNINGS - 1);
        warnings.truncate(MAX_RECOVERY_RESPONSE_WARNINGS - 1);
        warnings.push(format!(
            "{omitted} additional recovery warnings were omitted."
        ));
    }
    Ok(RecoveryRestoreResponse { snapshot, warnings })
}

#[tauri::command]
fn discard_recovery_snapshots(state: State<'_, AppState>) -> Result<(), BridgeError> {
    let mut registry = recovery_lock(&state)?;
    recovery_io::delete_all(recovery_directory(&state)?)
        .map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))?;
    registry.document_ids.clear();
    registry.generations.clear();
    Ok(())
}

#[tauri::command]
fn update_selection(
    state: State<'_, AppState>,
    request: UpdateSelectionRequest,
) -> Result<UpdateSelectionResponse, BridgeError> {
    session_lock(&state)?.update_selection(request)
}

#[tauri::command]
async fn open_document(
    state: State<'_, AppState>,
    _request: OpenDocumentRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let path = tauri::async_runtime::spawn_blocking(|| rfd::FileDialog::new().pick_file())
        .await
        .map_err(|error| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                format!("The native open dialog failed: {error}"),
            )
        })?
        .ok_or_else(|| {
            BridgeError::new(
                BridgeErrorCode::Cancelled,
                "The open operation was cancelled.",
            )
        })?;

    open_path(state, path).await
}

#[tauri::command]
async fn open_workspace(
    state: State<'_, AppState>,
    _request: OpenWorkspaceRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    session_lock(&state)?.request_workspace_picker()?;
    let initial_directory = session_lock(&state)?.workspace_root();
    let path = tauri::async_runtime::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if let Some(directory) = initial_directory {
            dialog = dialog.set_directory(directory);
        }
        dialog.pick_folder()
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The native workspace dialog failed: {error}"),
        )
    })?
    .ok_or_else(|| {
        BridgeError::new(
            BridgeErrorCode::Cancelled,
            "Opening a workspace was cancelled.",
        )
    })?;
    let (path, previous_document_ids) = {
        let mut session = session_lock(&state)?;
        let document_ids = session
            .snapshot()?
            .documents
            .into_iter()
            .map(|document| document.document_id)
            .collect::<Vec<_>>();
        let path = session.begin_open_workspace(path)?;
        (path, document_ids)
    };
    let workspace = tauri::async_runtime::spawn_blocking(move || workspace_io::discover(path))
        .await
        .map_err(|error| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                format!("The workspace discovery worker failed: {error}"),
            )
        })?;
    let mut snapshot = session_lock(&state)?.complete_open_workspace(workspace)?;
    for document_id in previous_document_ids {
        if let Some(warning) = delete_recovery_for_document(&state, document_id)? {
            snapshot.persistence_warning = Some(match snapshot.persistence_warning {
                Some(existing) => {
                    format!("{existing} Recovery data could not be cleared: {warning}")
                }
                None => format!("Recovery data could not be cleared: {warning}"),
            });
        }
    }
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
fn close_workspace(state: State<'_, AppState>) -> Result<WorkspaceSnapshot, BridgeError> {
    let (document_ids, mut snapshot) = {
        let mut session = session_lock(&state)?;
        let document_ids = session
            .snapshot()?
            .documents
            .into_iter()
            .map(|document| document.document_id)
            .collect::<Vec<_>>();
        (document_ids, session.close_workspace()?)
    };
    for document_id in document_ids {
        if let Some(warning) = delete_recovery_for_document(&state, document_id)? {
            snapshot.persistence_warning = Some(match snapshot.persistence_warning {
                Some(existing) => {
                    format!("{existing} Recovery data could not be cleared: {warning}")
                }
                None => format!("Recovery data could not be cleared: {warning}"),
            });
        }
    }

    let state_path = state.2.get().ok_or_else(|| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "The workspace state path was not initialized.",
        )
    })?;
    let warning = match fs::remove_file(state_path) {
        Ok(()) => None,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => Some(format!("The saved workspace could not be cleared: {error}")),
    };
    *state.1.lock().map_err(|_| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "The saved workspace state is unavailable because its lock was poisoned.",
        )
    })? = None;
    if let Some(warning) = warning {
        snapshot.persistence_warning = Some(match snapshot.persistence_warning {
            Some(existing) => format!("{existing} {warning}"),
            None => warning,
        });
    }
    Ok(snapshot)
}

#[tauri::command]
async fn load_workspace_directory(
    state: State<'_, AppState>,
    request: WorkspaceDirectoryRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let relative_path = PathBuf::from(request.relative_path);
    let (root, relative_path) = session_lock(&state)?.request_workspace_directory(relative_path)?;
    let read_root = root.clone();
    let read_path = relative_path.clone();
    let entries = tauri::async_runtime::spawn_blocking(move || {
        workspace_io::read_directory(read_root, read_path)
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The workspace directory reader failed: {error}"),
        )
    })?;
    session_lock(&state)?.complete_workspace_directory(root, relative_path, entries)
}

#[tauri::command]
async fn open_workspace_file(
    state: State<'_, AppState>,
    request: WorkspaceFileRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let (path, ticket) = session_lock(&state)?.begin_workspace_file_open(&request.relative_path)?;
    match ticket {
        Some(ticket) => load_open_ticket(state, path, ticket).await,
        None => {
            let snapshot = session_lock(&state)?.snapshot()?;
            persist_workspace_snapshot(&state, snapshot)
        }
    }
}

#[tauri::command]
async fn search_workspace(
    state: State<'_, AppState>,
    request: SearchWorkspaceRequest,
) -> Result<WorkspaceSearchResponse, BridgeError> {
    let query = validate_search_request(&request)?;
    let (root, open_documents) = session_lock(&state)?.search_inputs()?;
    let expected_root = root.clone();
    let cancellation = Arc::new(AtomicBool::new(false));
    {
        let mut searches = state.5.lock().map_err(|_| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The search registry lock was poisoned.",
            )
        })?;
        if searches.contains_key(&request.search_id) {
            return Err(BridgeError::invalid_request(
                "The search identifier is already in use.",
            ));
        }
        searches.insert(request.search_id.clone(), cancellation.clone());
    }
    let search_id = request.search_id.clone();
    let case_sensitive = request.case_sensitive;
    let whole_word = request.whole_word;
    let operation_cancellation = cancellation.clone();
    let operation = tauri::async_runtime::spawn_blocking(move || {
        if let Some(root) = root {
            workspace_io::search_workspace(
                root,
                &query,
                case_sensitive,
                whole_word,
                open_documents,
                &search_id,
                &operation_cancellation,
            )
        } else {
            Ok(workspace_io::search_open_documents(
                &query,
                case_sensitive,
                whole_word,
                open_documents,
                &search_id,
                &operation_cancellation,
            ))
        }
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The workspace search worker failed: {error}"),
        )
    });
    state
        .5
        .lock()
        .map_err(|_| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The search registry lock was poisoned.",
            )
        })?
        .remove(&request.search_id);
    let response =
        operation?.map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))?;
    if session_lock(&state)?.workspace_root() != expected_root {
        return Err(BridgeError::invalid_request(
            "The workspace changed while search was running. Search again in the active workspace.",
        ));
    }
    Ok(response)
}

#[tauri::command]
async fn replace_workspace_search_results(
    state: State<'_, AppState>,
    request: ReplaceWorkspaceSearchResultsRequest,
) -> Result<ReplaceWorkspaceSearchResultsResponse, BridgeError> {
    let root = {
        let session = session_lock(&state)?;
        let root = session
            .workspace_root()
            .ok_or_else(|| BridgeError::invalid_request("No workspace is open."))?;
        if request.workspace_root != root.to_string_lossy() {
            return Err(BridgeError::new(
                BridgeErrorCode::StaleRevision,
                "The search result is stale because the workspace changed.",
            ));
        }
        for file in &request.files {
            if session.workspace_search_result_is_open(&file.relative_path)? {
                return Err(BridgeError::new(
                    BridgeErrorCode::StaleRevision,
                    "A workspace search result has been opened since the search. Search again before replacing.",
                ));
            }
        }
        root
    };
    tauri::async_runtime::spawn_blocking(move || {
        workspace_io::replace_unopened_search_files(root, request)
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The workspace replacement worker failed: {error}"),
        )
    })?
}

fn validate_search_request(request: &SearchWorkspaceRequest) -> Result<String, BridgeError> {
    let query = request.query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 256 {
        return Err(BridgeError::invalid_request(
            "The search term must contain at most 256 characters and cannot be empty.",
        ));
    }
    if request.search_id.is_empty()
        || request.search_id.len() > 64
        || !request
            .search_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
    {
        return Err(BridgeError::invalid_request(
            "The search identifier is invalid.",
        ));
    }
    Ok(query)
}

#[tauri::command]
fn cancel_search(
    state: State<'_, AppState>,
    request: CancelSearchRequest,
) -> Result<bool, BridgeError> {
    let searches = state.5.lock().map_err(|_| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            "The search registry lock was poisoned.",
        )
    })?;
    if let Some(cancellation) = searches.get(&request.search_id) {
        cancellation.store(true, Ordering::Relaxed);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn open_workspace_search_result(
    state: State<'_, AppState>,
    request: OpenWorkspaceSearchResultRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    if let Some(expected_root) = request.workspace_root.as_deref() {
        let current_root = session_lock(&state)?
            .workspace_root()
            .map(|root| root.to_string_lossy().into_owned());
        if current_root.as_deref() != Some(expected_root) {
            return Err(BridgeError::new(
                BridgeErrorCode::StaleRevision,
                "The search result is stale because the workspace changed.",
            ));
        }
    }
    let snapshot = match (request.document_id, request.expected_revision) {
        (Some(document_id), Some(expected_revision)) => session_lock(&state)?
            .navigate_to_document_search_result(
                document_id,
                expected_revision,
                request.line_number,
                request.column,
                request.match_length,
            )?,
        (None, None) => {
            let source_fingerprint = request.source_fingerprint.as_deref().ok_or_else(|| {
                BridgeError::invalid_request("The search result is missing its source version.")
            })?;
            if request.workspace_root.is_none() {
                return Err(BridgeError::invalid_request(
                    "A workspace search result must include its workspace identity.",
                ));
            }
            let (path, ticket) =
                session_lock(&state)?.begin_workspace_search_result_open(&request.relative_path)?;
            if let Some(ticket) = ticket {
                load_open_ticket(state.clone(), path.clone(), ticket).await?;
            }
            session_lock(&state)?.navigate_to_path(
                &path,
                request.line_number,
                request.column,
                request.match_length,
                Some(source_fingerprint),
            )?
        }
        _ => {
            return Err(BridgeError::invalid_request(
                "A search result must include both its document identity and revision.",
            ));
        }
    };
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
async fn restore_workspace_documents(
    state: State<'_, AppState>,
) -> Result<RestoreWorkspaceResponse, BridgeError> {
    let saved_state = state
        .1
        .lock()
        .map_err(|_| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The saved workspace state is unavailable because its lock was poisoned.",
            )
        })?
        .clone();
    let Some(saved_state) = saved_state else {
        return Ok(RestoreWorkspaceResponse {
            snapshot: session_lock(&state)?.snapshot()?,
            warnings: Vec::new(),
        });
    };

    if session_lock(&state)?.workspace_root().as_deref() != Some(&saved_state.root) {
        return Ok(RestoreWorkspaceResponse {
            snapshot: session_lock(&state)?.snapshot()?,
            warnings: vec![
                "Open tabs were not restored because the saved workspace is unavailable."
                    .to_owned(),
            ],
        });
    }

    let root = saved_state.root.clone();
    let loaded_files = tauri::async_runtime::spawn_blocking(move || {
        saved_state
            .open_files
            .into_iter()
            .map(|relative_path| {
                let result = workspace_io::resolve_file(&root, &relative_path)
                    .map_err(|error| BridgeError::new(BridgeErrorCode::FileIo, error.to_string()))
                    .and_then(|path| {
                        file_io::load(path).map_err(functor::session::file_error_to_bridge)
                    });
                (relative_path, result)
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The workspace restore worker failed: {error}"),
        )
    })?;

    let mut files = Vec::with_capacity(loaded_files.len());
    let mut warnings = Vec::new();
    for (relative_path, result) in loaded_files {
        match result {
            Ok(file) => files.push(file),
            Err(error) => warnings.push(format!(
                "Could not restore {}: {}",
                relative_path.display(),
                error.message
            )),
        }
    }

    state
        .1
        .lock()
        .map_err(|_| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                "The saved workspace state is unavailable because its lock was poisoned.",
            )
        })?
        .take();

    let snapshot =
        session_lock(&state)?.restore_workspace_documents(files, saved_state.active_file)?;
    let snapshot = persist_workspace_snapshot(&state, snapshot)?;
    Ok(RestoreWorkspaceResponse { snapshot, warnings })
}

async fn open_path(
    state: State<'_, AppState>,
    path: PathBuf,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let ticket = session_lock(&state)?.begin_open(path.clone());
    let Some(ticket) = ticket else {
        let snapshot = session_lock(&state)?.snapshot()?;
        return persist_workspace_snapshot(&state, snapshot);
    };

    load_open_ticket(state, path, ticket).await
}

async fn load_open_ticket(
    state: State<'_, AppState>,
    path: PathBuf,
    ticket: functor::session::OpenTicket,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let loaded = tauri::async_runtime::spawn_blocking(move || file_io::load(path))
        .await
        .map_err(|error| {
            BridgeError::new(
                BridgeErrorCode::Internal,
                format!("The file-loading worker failed: {error}"),
            )
        })?;
    let snapshot = session_lock(&state)?.complete_open(ticket, loaded)?;
    persist_workspace_snapshot(&state, snapshot)
}

#[tauri::command]
async fn save_document(
    state: State<'_, AppState>,
    request: SaveDocumentRequest,
) -> Result<WorkspaceSnapshot, BridgeError> {
    let existing_path =
        session_lock(&state)?.document_path(request.document_id, request.expected_revision)?;
    let destination = if request.save_as {
        None
    } else {
        existing_path.clone()
    };
    let destination = match destination {
        Some(path) => path,
        None => {
            let suggested_name = existing_path
                .as_deref()
                .and_then(Path::file_name)
                .map(|name| name.to_string_lossy().into_owned());
            tauri::async_runtime::spawn_blocking(move || {
                let mut dialog = rfd::FileDialog::new();
                if let Some(name) = suggested_name.as_deref() {
                    dialog = dialog.set_file_name(name);
                }
                dialog.save_file()
            })
            .await
            .map_err(|error| {
                BridgeError::new(
                    BridgeErrorCode::Internal,
                    format!("The native save dialog failed: {error}"),
                )
            })?
            .ok_or_else(|| {
                BridgeError::new(
                    BridgeErrorCode::Cancelled,
                    "The save operation was cancelled.",
                )
            })?
        }
    };
    let identity_path = file_io::canonical_identity_path(&destination).map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::FileIo,
            format!(
                "Could not resolve the save destination {}: {error}",
                destination.display()
            ),
        )
    })?;
    let ticket = {
        let session = session_lock(&state)?;
        session.ensure_save_destination_available(request.document_id, &identity_path)?;
        session.begin_save(
            request.document_id,
            request.expected_revision,
            Some(destination),
        )?
    };
    let path = EditorSession::save_ticket_path(&ticket).clone();
    let text = EditorSession::save_ticket_text(&ticket).to_owned();
    let expected_stamp = EditorSession::save_ticket_stamp(&ticket).cloned();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        file_io::save_if_unchanged(&path, &text, expected_stamp.as_ref())
    })
    .await
    .map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::Internal,
            format!("The file-saving worker failed: {error}"),
        )
    })?;
    let snapshot = session_lock(&state)?.complete_save(ticket, saved)?;
    let document_is_clean = snapshot
        .documents
        .iter()
        .find(|document| document.document_id == request.document_id)
        .is_some_and(|document| !document.dirty);
    let mut snapshot = snapshot;
    if document_is_clean
        && let Some(warning) = delete_recovery_for_document(&state, request.document_id)?
    {
        snapshot.persistence_warning =
            Some(format!("Recovery data could not be cleared: {warning}"));
    }
    persist_workspace_snapshot(&state, snapshot)
}

fn settings_directory(app: &tauri::AppHandle) -> Result<PathBuf, BridgeError> {
    app.path().app_config_dir().map_err(|error| {
        BridgeError::new(
            BridgeErrorCode::FileIo,
            format!("Could not resolve the application settings directory: {error}"),
        )
    })
}

fn settings_file_error(error: String) -> BridgeError {
    BridgeError::new(BridgeErrorCode::FileIo, error)
}

#[tauri::command]
fn get_settings(app: tauri::AppHandle) -> Result<SettingsCatalogResponse, BridgeError> {
    settings_io::load_catalog(settings_directory(&app)?).map_err(settings_file_error)
}

#[tauri::command]
fn save_settings(
    app: tauri::AppHandle,
    request: SaveSettingsRequest,
) -> Result<AppSettings, BridgeError> {
    let settings = settings_io::validate(request.settings).map_err(BridgeError::invalid_request)?;
    settings_io::save_settings(settings_directory(&app)?, settings).map_err(settings_file_error)
}

#[tauri::command]
fn load_user_theme(
    app: tauri::AppHandle,
    request: UserThemeRequest,
) -> Result<AppSettings, BridgeError> {
    let name = settings_io::validate_user_theme_name(&request.name)
        .map_err(BridgeError::invalid_request)?;
    settings_io::load_user_theme(settings_directory(&app)?, &name).map_err(settings_file_error)
}

#[tauri::command]
fn save_user_theme(
    app: tauri::AppHandle,
    request: SaveUserThemeRequest,
) -> Result<UserThemeSummary, BridgeError> {
    let name = settings_io::validate_user_theme_name(&request.name)
        .map_err(BridgeError::invalid_request)?;
    let settings = settings_io::validate(request.settings).map_err(BridgeError::invalid_request)?;
    settings_io::save_user_theme(settings_directory(&app)?, &name, settings)
        .map_err(settings_file_error)
}

#[tauri::command]
fn exit_application(app: tauri::AppHandle) {
    app.exit(0);
}

fn main() {
    tauri::Builder::default()
        .manage(AppState(
            Mutex::new(EditorSession::default()),
            Mutex::new(None),
            OnceLock::new(),
            OnceLock::new(),
            Mutex::new(RecoveryRegistry::new()),
            Mutex::new(HashMap::new()),
        ))
        .setup(|app| {
            let state_path = app.path().app_data_dir()?.join("last-workspace.json");
            let (mut persisted, state_error) = match workspace_io::load_last_workspace(&state_path)
            {
                Ok(persisted) => (persisted, None),
                Err(error) => (None, Some(error)),
            };
            let app_state = app.state::<AppState>();
            let mut session = app_state
                .0
                .lock()
                .map_err(|_| std::io::Error::other("The editor session lock was poisoned."))?;
            if let Some(error) = state_error {
                if let Err(restore_error) = session.complete_open_workspace(Err(error)) {
                    eprintln!(
                        "The previous workspace could not be restored: {}",
                        restore_error.message
                    );
                }
            } else if let Some(saved_state) = persisted.as_mut() {
                match workspace_io::discover(&saved_state.root) {
                    Ok(workspace) => {
                        saved_state.root = workspace.root.clone();
                        if let Err(error) = session.complete_open_workspace(Ok(workspace)) {
                            eprintln!(
                                "The previous workspace could not be restored: {}",
                                error.message
                            );
                        }
                    }
                    Err(error) => {
                        if let Err(restore_error) = session.complete_open_workspace(Err(error)) {
                            eprintln!(
                                "The previous workspace could not be restored: {}",
                                restore_error.message
                            );
                        }
                    }
                }
            }
            drop(session);
            *app_state
                .1
                .lock()
                .map_err(|_| std::io::Error::other("The workspace state lock was poisoned."))? =
                persisted;
            app_state.2.set(state_path).map_err(|_| {
                std::io::Error::other("The workspace state path was already initialized.")
            })?;
            app_state
                .3
                .set(app.path().app_data_dir()?.join("recovery"))
                .map_err(|_| {
                    std::io::Error::other("The recovery directory was already initialized.")
                })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            new_document,
            set_document_language,
            switch_document,
            close_document,
            apply_document_edits,
            save_recovery_snapshot,
            list_recovery_snapshots,
            restore_recovery_snapshots,
            discard_recovery_snapshots,
            update_selection,
            open_document,
            open_workspace,
            close_workspace,
            load_workspace_directory,
            open_workspace_file,
            search_workspace,
            replace_workspace_search_results,
            cancel_search,
            open_workspace_search_result,
            reopen_closed_document,
            restore_workspace_documents,
            save_document,
            clear_recent_documents,
            get_settings,
            save_settings,
            load_user_theme,
            save_user_theme,
            exit_application
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Functor");
}

#[cfg(test)]
mod tests {
    use super::{stable_recovery_id, validate_search_request};
    use functor::protocol::{BridgeErrorCode, SearchWorkspaceRequest};
    use std::path::Path;

    #[test]
    fn file_recovery_ids_are_stable_and_case_insensitive() {
        let expected = "file-e5aa36e08d13dd0cfe117353c71f63c7";
        assert_eq!(
            stable_recovery_id(Path::new(r"C:\Project\main.rs")),
            expected
        );
        assert_eq!(
            stable_recovery_id(Path::new(r"c:\project\MAIN.RS")),
            expected
        );
    }

    #[test]
    fn search_request_validation_trims_query() {
        let request = SearchWorkspaceRequest {
            search_id: "search-1".into(),
            query: "  hello  ".into(),
            case_sensitive: false,
            whole_word: false,
        };

        assert_eq!(validate_search_request(&request).unwrap(), "hello");
    }

    #[test]
    fn search_request_validation_rejects_empty_terms_and_invalid_ids() {
        for request in [
            SearchWorkspaceRequest {
                search_id: "search-1".into(),
                query: "  ".into(),
                case_sensitive: false,
                whole_word: false,
            },
            SearchWorkspaceRequest {
                search_id: "invalid id".into(),
                query: "hello".into(),
                case_sensitive: false,
                whole_word: false,
            },
        ] {
            let error = validate_search_request(&request).unwrap_err();
            assert_eq!(error.code, BridgeErrorCode::InvalidRequest);
        }
    }
}
