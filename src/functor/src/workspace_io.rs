use std::collections::HashMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::file_io;
use crate::protocol::{
    BridgeError, BridgeErrorCode, ReplaceWorkspaceSearchResultsRequest,
    ReplaceWorkspaceSearchResultsResponse, WorkspaceSearchHit, WorkspaceSearchResponse,
};
use functor_core::file_io::FileError;
use functor_core::workspace::{Workspace, WorkspaceEntry, WorkspaceEntryKind, WorkspaceError};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};

const LAST_WORKSPACE_VERSION: u32 = 2;
const MAX_RESTORED_OPEN_FILES: usize = 100;
const MAX_PREVIEW_CHARS: usize = 220;
const MAX_SEARCH_WARNINGS: usize = 20;
static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchDocumentInput {
    pub document_id: u64,
    pub revision: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub text: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LastWorkspace {
    version: u32,
    root: PathBuf,
    #[serde(default)]
    open_files: Vec<PathBuf>,
    #[serde(default)]
    active_file: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceSessionState {
    pub root: PathBuf,
    pub open_files: Vec<PathBuf>,
    pub active_file: Option<PathBuf>,
}

pub fn load_last_workspace(
    path: impl AsRef<Path>,
) -> Result<Option<WorkspaceSessionState>, WorkspaceError> {
    let path = path.as_ref();
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(WorkspaceError::ReadFailed {
                path: path.to_path_buf(),
                message: error.to_string(),
            });
        }
    };
    let persisted = serde_json::from_slice::<LastWorkspace>(&contents).map_err(|error| {
        WorkspaceError::ReadFailed {
            path: path.to_path_buf(),
            message: format!("The saved workspace state is invalid: {error}"),
        }
    })?;
    if persisted.version != 1 && persisted.version != LAST_WORKSPACE_VERSION {
        return Err(WorkspaceError::ReadFailed {
            path: path.to_path_buf(),
            message: format!(
                "The saved workspace state version {} is not supported.",
                persisted.version
            ),
        });
    }
    if !persisted.root.is_absolute() {
        return Err(WorkspaceError::InvalidPath(persisted.root));
    }
    if persisted.open_files.len() > MAX_RESTORED_OPEN_FILES
        || persisted
            .open_files
            .iter()
            .any(|path| !is_normal_relative_path(path))
    {
        return Err(WorkspaceError::ReadFailed {
            path: path.to_path_buf(),
            message: "The saved workspace contains invalid or excessive open-file state.".into(),
        });
    }
    if persisted
        .active_file
        .as_ref()
        .is_some_and(|active| !persisted.open_files.contains(active))
    {
        return Err(WorkspaceError::ReadFailed {
            path: path.to_path_buf(),
            message: "The saved active file is not in the open-file list.".into(),
        });
    }
    Ok(Some(WorkspaceSessionState {
        root: persisted.root,
        open_files: persisted.open_files,
        active_file: persisted.active_file,
    }))
}

pub fn save_last_workspace(
    path: impl AsRef<Path>,
    state: &WorkspaceSessionState,
) -> Result<(), WorkspaceError> {
    let path = path.as_ref();
    let parent = path
        .parent()
        .ok_or_else(|| WorkspaceError::InvalidPath(path.to_path_buf()))?;
    fs::create_dir_all(parent).map_err(|error| WorkspaceError::ReadFailed {
        path: parent.to_path_buf(),
        message: error.to_string(),
    })?;

    if !state.root.is_absolute()
        || state.open_files.len() > MAX_RESTORED_OPEN_FILES
        || state
            .open_files
            .iter()
            .any(|path| !is_normal_relative_path(path))
        || state
            .active_file
            .as_ref()
            .is_some_and(|active| !state.open_files.contains(active))
    {
        return Err(WorkspaceError::InvalidPath(path.to_path_buf()));
    }
    let persisted = LastWorkspace {
        version: LAST_WORKSPACE_VERSION,
        root: state.root.clone(),
        open_files: state.open_files.clone(),
        active_file: state.active_file.clone(),
    };
    let contents = serde_json::to_vec(&persisted).map_err(|error| WorkspaceError::ReadFailed {
        path: path.to_path_buf(),
        message: format!("Could not encode the saved workspace state: {error}"),
    })?;
    let temp_path = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temp_path, contents).map_err(|error| WorkspaceError::ReadFailed {
        path: temp_path.clone(),
        message: error.to_string(),
    })?;
    if let Err(rename_error) = fs::rename(&temp_path, path) {
        let cleanup_error = fs::remove_file(&temp_path).err();
        let message = match cleanup_error {
            Some(cleanup_error) => {
                format!("{rename_error}; temporary-file cleanup also failed: {cleanup_error}")
            }
            None => rename_error.to_string(),
        };
        return Err(WorkspaceError::ReadFailed {
            path: path.to_path_buf(),
            message,
        });
    }
    Ok(())
}

fn is_normal_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

pub fn discover(root: impl AsRef<Path>) -> Result<Workspace, WorkspaceError> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Err(WorkspaceError::InvalidRoot(root.to_path_buf()));
    }
    let root = fs::canonicalize(root).map_err(|error| WorkspaceError::ReadFailed {
        path: root.to_path_buf(),
        message: error.to_string(),
    })?;
    let entries = read_entries(&root)?;
    Ok(Workspace { root, entries })
}

pub fn read_directory(
    root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
    let root = canonical_root(root.as_ref())?;
    let directory = resolve_inside_root(&root, relative_path.as_ref())?;
    if !directory.is_dir() {
        return Err(WorkspaceError::InvalidPath(directory));
    }
    read_entries(&directory)
}

pub fn resolve_file(
    root: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> Result<PathBuf, WorkspaceError> {
    let root = canonical_root(root.as_ref())?;
    let path = resolve_inside_root(&root, relative_path.as_ref())?;
    if !path.is_file() {
        return Err(WorkspaceError::InvalidPath(path));
    }
    Ok(path)
}

pub fn search_workspace(
    root: impl AsRef<Path>,
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    open_documents: Vec<SearchDocumentInput>,
    search_id: &str,
    cancelled: &AtomicBool,
) -> Result<WorkspaceSearchResponse, WorkspaceError> {
    let root = canonical_root(root.as_ref())?;
    let workspace_root = root.to_string_lossy().into_owned();
    let mut overrides = HashMap::new();
    for document in open_documents {
        if let Some(path) = document
            .path
            .as_ref()
            .filter(|path| path.starts_with(&root))
        {
            overrides.insert(path.clone(), document);
        }
    }
    let mut response = WorkspaceSearchResponse {
        search_id: search_id.to_owned(),
        workspace_root: Some(workspace_root.clone()),
        hits: Vec::new(),
        cancelled: false,
        skipped_files: 0,
        warnings: Vec::new(),
    };
    let walker = WalkBuilder::new(&root)
        .hidden(false)
        .git_ignore(false)
        .git_exclude(false)
        .git_global(false)
        .ignore(false)
        .require_git(false)
        .follow_links(false)
        .build();

    for result in walker {
        if cancelled.load(Ordering::Relaxed) {
            response.cancelled = true;
            break;
        }
        let entry = match result {
            Ok(entry) => entry,
            Err(error) => {
                response.skipped_files += 1;
                record_search_warning(&mut response, error.to_string());
                continue;
            }
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let path = entry.path();
        let relative_path = relative_path_string(&root, path)?;
        let (text, source_document, source_fingerprint) = if let Some((_, document)) = overrides
            .iter()
            .find(|(override_path, _)| same_path(override_path, path))
        {
            (document.text.clone(), Some(document), None)
        } else {
            match read_searchable_text(path) {
                Ok(Some(text)) => {
                    let fingerprint = search_fingerprint(&text);
                    (text, None, Some(fingerprint))
                }
                Ok(None) => {
                    response.skipped_files += 1;
                    record_search_warning(
                        &mut response,
                        format!("Skipped non-text file `{relative_path}`."),
                    );
                    continue;
                }
                Err(error) => {
                    response.skipped_files += 1;
                    record_search_warning(&mut response, error.to_string());
                    continue;
                }
            }
        };
        append_search_hits(
            &mut response,
            &relative_path,
            &text,
            query,
            case_sensitive,
            whole_word,
            source_document,
            source_fingerprint,
            Some(workspace_root.clone()),
            cancelled,
        );
        if response.cancelled {
            return Ok(response);
        }
    }

    Ok(response)
}

pub fn search_open_documents(
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    documents: Vec<SearchDocumentInput>,
    search_id: &str,
    cancelled: &AtomicBool,
) -> WorkspaceSearchResponse {
    let mut response = WorkspaceSearchResponse {
        search_id: search_id.to_owned(),
        workspace_root: None,
        hits: Vec::new(),
        cancelled: false,
        skipped_files: 0,
        warnings: Vec::new(),
    };
    for document in documents {
        if cancelled.load(Ordering::Relaxed) {
            response.cancelled = true;
            break;
        }
        append_search_hits(
            &mut response,
            &document.title,
            &document.text,
            query,
            case_sensitive,
            whole_word,
            Some(&document),
            None,
            None,
            cancelled,
        );
        if response.cancelled {
            break;
        }
    }
    response
}

fn record_search_warning(response: &mut WorkspaceSearchResponse, warning: String) {
    if response.warnings.len() < MAX_SEARCH_WARNINGS {
        response.warnings.push(warning);
    }
}

fn read_searchable_text(path: &Path) -> Result<Option<String>, WorkspaceError> {
    let bytes = fs::read(path).map_err(|error| WorkspaceError::ReadFailed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;
    if bytes.contains(&0) {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok().map(|text| {
        text.strip_prefix('\u{feff}')
            .unwrap_or(&text)
            .replace("\r\n", "\n")
            .replace('\r', "\n")
    }))
}

fn append_search_hits(
    response: &mut WorkspaceSearchResponse,
    relative_path: &str,
    text: &str,
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    source_document: Option<&SearchDocumentInput>,
    source_fingerprint: Option<String>,
    workspace_root: Option<String>,
    cancelled: &AtomicBool,
) {
    let mut line_offset = 0usize;
    for (line_index, line) in text.split('\n').enumerate() {
        if cancelled.load(Ordering::Relaxed) {
            response.cancelled = true;
            return;
        }
        for (start, end) in find_matches(line, query, case_sensitive, whole_word) {
            response.hits.push(WorkspaceSearchHit {
                relative_path: relative_path.to_owned(),
                document_id: source_document.map(|document| document.document_id),
                revision: source_document.map(|document| document.revision),
                line_number: u32::try_from(line_index + 1).unwrap_or(u32::MAX),
                column: u32::try_from(line[..start].encode_utf16().count() + 1).unwrap_or(u32::MAX),
                match_length: u32::try_from(line[start..end].encode_utf16().count())
                    .unwrap_or(u32::MAX),
                range_offset: u32::try_from(line_offset + line[..start].encode_utf16().count())
                    .unwrap_or(u32::MAX),
                range_length: u32::try_from(line[start..end].encode_utf16().count())
                    .unwrap_or(u32::MAX),
                matched_text: line[start..end].to_owned(),
                source_fingerprint: source_fingerprint.clone(),
                workspace_root: workspace_root.clone(),
                preview: search_preview(line, start, end),
            });
        }
        line_offset += line.encode_utf16().count() + 1;
    }
}

pub fn search_fingerprint(text: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in text.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub fn replace_unopened_search_files(
    root: impl AsRef<Path>,
    request: ReplaceWorkspaceSearchResultsRequest,
) -> Result<ReplaceWorkspaceSearchResultsResponse, BridgeError> {
    if request.files.is_empty() {
        return Err(BridgeError::invalid_request(
            "Replace All did not include any unopened workspace files.",
        ));
    }
    if request.replacement.len() > crate::protocol::MAX_INSERTED_TEXT_BYTES {
        return Err(BridgeError::invalid_request(
            "Replacement text exceeds the 1 MiB limit.",
        ));
    }
    let root = canonical_root(root.as_ref())
        .map_err(|error| workspace_replace_file_error(error.to_string()))?;
    if request.workspace_root != root.to_string_lossy() {
        return Err(stale_workspace_search_result(
            "The search result is stale because the workspace changed.",
        ));
    }

    let mut prepared = Vec::with_capacity(request.files.len());
    let mut seen_paths = std::collections::HashSet::new();
    let mut replaced = 0;
    for file in request.files {
        let path = resolve_file(&root, PathBuf::from(&file.relative_path))
            .map_err(|error| workspace_replace_file_error(error.to_string()))?;
        if !seen_paths.insert(path.clone()) {
            return Err(BridgeError::invalid_request(
                "Replace All contains duplicate workspace file paths.",
            ));
        }
        if file.matches.is_empty() {
            return Err(BridgeError::invalid_request(
                "Replace All contains a workspace file without any matches.",
            ));
        }
        let loaded = file_io::load(&path).map_err(workspace_replace_load_error)?;
        if search_fingerprint(&loaded.text) != file.source_fingerprint {
            return Err(stale_workspace_search_result(format!(
                "The search result for `{}` is stale because the file changed after the search.",
                file.relative_path
            )));
        }

        let mut edits = Vec::with_capacity(file.matches.len());
        for matched in file.matches {
            if matched.matched_text.is_empty()
                || matched.matched_text.encode_utf16().count() != matched.range_length as usize
            {
                return Err(BridgeError::invalid_request(
                    "Replace All contains an invalid search match.",
                ));
            }
            let (start, end) = utf16_range_to_byte_offsets(
                &loaded.text,
                matched.range_offset,
                matched.range_length,
            )
            .ok_or_else(|| {
                stale_workspace_search_result(format!(
                    "The search result for `{}` has an invalid text range.",
                    file.relative_path
                ))
            })?;
            if loaded.text.get(start..end) != Some(matched.matched_text.as_str()) {
                return Err(stale_workspace_search_result(format!(
                    "The search result for `{}` no longer matches its source text.",
                    file.relative_path
                )));
            }
            edits.push((start, end));
        }
        edits.sort_unstable_by_key(|(start, _)| *start);
        if edits.windows(2).any(|pair| pair[0].1 > pair[1].0) {
            return Err(BridgeError::invalid_request(
                "Replace All contains overlapping search matches.",
            ));
        }

        let mut updated_text = loaded.text.clone();
        for (start, end) in edits.iter().rev() {
            updated_text.replace_range(*start..*end, &request.replacement);
        }
        replaced += edits.len();
        if updated_text != loaded.text {
            prepared.push((loaded.path, loaded.stamp, updated_text));
        }
    }

    let mut files_written = 0;
    for (path, stamp, updated_text) in prepared {
        if let Err(error) = file_io::save_if_unchanged(&path, &updated_text, Some(&stamp)) {
            let error = workspace_replace_save_error(error);
            let message = if files_written == 0 {
                error.message
            } else {
                format!(
                    "{} {files_written} earlier workspace file(s) were already updated.",
                    error.message
                )
            };
            return Err(BridgeError::new(error.code, message));
        }
        files_written += 1;
    }

    Ok(ReplaceWorkspaceSearchResultsResponse {
        replaced,
        files_written,
    })
}

fn utf16_range_to_byte_offsets(
    text: &str,
    range_offset: u32,
    range_length: u32,
) -> Option<(usize, usize)> {
    let end_offset = range_offset.checked_add(range_length)?;
    Some((
        utf16_offset_to_byte_offset(text, range_offset as usize)?,
        utf16_offset_to_byte_offset(text, end_offset as usize)?,
    ))
}

fn utf16_offset_to_byte_offset(text: &str, target: usize) -> Option<usize> {
    let mut utf16_offset = 0;
    for (byte_offset, character) in text.char_indices() {
        if utf16_offset == target {
            return Some(byte_offset);
        }
        utf16_offset += character.len_utf16();
        if utf16_offset > target {
            return None;
        }
    }
    (utf16_offset == target).then_some(text.len())
}

fn stale_workspace_search_result(message: impl Into<String>) -> BridgeError {
    BridgeError::new(BridgeErrorCode::StaleRevision, message)
}

fn workspace_replace_file_error(message: String) -> BridgeError {
    BridgeError::new(BridgeErrorCode::FileIo, message)
}

fn workspace_replace_load_error(error: FileError) -> BridgeError {
    match error {
        FileError::InvalidUtf8 { path } => stale_workspace_search_result(format!(
            "The file `{}` is no longer searchable text.",
            path.display()
        )),
        FileError::Read { path, message } => BridgeError::new(
            BridgeErrorCode::FileIo,
            format!("Could not read {}: {message}", path.display()),
        ),
        FileError::Modified { path } => stale_workspace_search_result(format!(
            "The file `{}` changed while Replace All was running.",
            path.display()
        )),
        FileError::Write { path, message } => BridgeError::new(
            BridgeErrorCode::FileIo,
            format!("Could not update {}: {message}", path.display()),
        ),
    }
}

fn workspace_replace_save_error(error: FileError) -> BridgeError {
    match error {
        FileError::Modified { path } => stale_workspace_search_result(format!(
            "The file `{}` changed while Replace All was running.",
            path.display()
        )),
        error => workspace_replace_load_error(error),
    }
}

fn find_matches(
    line: &str,
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
) -> Vec<(usize, usize)> {
    let matches = if case_sensitive {
        line.match_indices(query)
            .map(|(start, matched)| (start, start + matched.len()))
            .collect::<Vec<_>>()
    } else {
        let query: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
        if query.is_empty() {
            return Vec::new();
        }
        let mut folded = Vec::new();
        for (start, character) in line.char_indices() {
            let end = start + character.len_utf8();
            folded.extend(
                character
                    .to_lowercase()
                    .map(|folded_character| (folded_character, start, end)),
            );
        }

        let mut matches = Vec::new();
        let mut index = 0;
        while index + query.len() <= folded.len() {
            if folded[index..index + query.len()]
                .iter()
                .map(|(character, _, _)| *character)
                .eq(query.iter().copied())
            {
                matches.push((folded[index].1, folded[index + query.len() - 1].2));
                index += query.len();
            } else {
                index += 1;
            }
        }
        matches
    };

    if whole_word {
        matches
            .into_iter()
            .filter(|(start, end)| {
                !line[..*start]
                    .chars()
                    .next_back()
                    .is_some_and(is_word_character)
                    && !line[*end..].chars().next().is_some_and(is_word_character)
            })
            .collect()
    } else {
        matches
    }
}

fn is_word_character(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

fn search_preview(line: &str, start: usize, end: usize) -> String {
    let characters: Vec<char> = line.chars().collect();
    let start_character = line[..start].chars().count();
    let end_character = start_character + line[start..end].chars().count();
    let context = MAX_PREVIEW_CHARS / 2;
    let preview_start = start_character.saturating_sub(context);
    let preview_end = (end_character + context).min(characters.len());
    let mut preview = String::new();
    if preview_start > 0 {
        preview.push('…');
    }
    preview.extend(characters[preview_start..preview_end].iter());
    if preview_end < characters.len() {
        preview.push('…');
    }
    preview
}

fn relative_path_string(root: &Path, path: &Path) -> Result<String, WorkspaceError> {
    path.strip_prefix(root)
        .map_err(|_| WorkspaceError::InvalidPath(path.to_path_buf()))
        .map(|relative| {
            relative
                .components()
                .map(|component| component.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/")
        })
}

fn same_path(left: &Path, right: &Path) -> bool {
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

fn canonical_root(root: &Path) -> Result<PathBuf, WorkspaceError> {
    if !root.is_dir() {
        return Err(WorkspaceError::InvalidRoot(root.to_path_buf()));
    }
    fs::canonicalize(root).map_err(|error| WorkspaceError::ReadFailed {
        path: root.to_path_buf(),
        message: error.to_string(),
    })
}

fn resolve_inside_root(root: &Path, relative_path: &Path) -> Result<PathBuf, WorkspaceError> {
    if relative_path.as_os_str().is_empty()
        || relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(WorkspaceError::InvalidPath(relative_path.to_path_buf()));
    }

    let path = root.join(relative_path);
    let canonical = fs::canonicalize(&path).map_err(|error| WorkspaceError::ReadFailed {
        path: path.clone(),
        message: error.to_string(),
    })?;
    if !canonical.starts_with(root) {
        return Err(WorkspaceError::InvalidPath(relative_path.to_path_buf()));
    }
    Ok(canonical)
}

fn read_entries(directory: &Path) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|error| WorkspaceError::ReadFailed {
        path: directory.to_path_buf(),
        message: error.to_string(),
    })? {
        let entry = entry.map_err(|error| WorkspaceError::ReadFailed {
            path: directory.to_path_buf(),
            message: error.to_string(),
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| WorkspaceError::ReadFailed {
                path: path.clone(),
                message: error.to_string(),
            })?;
        if file_type.is_symlink() {
            continue;
        }
        let kind = if file_type.is_dir() {
            WorkspaceEntryKind::Directory
        } else {
            WorkspaceEntryKind::File
        };
        entries.push(WorkspaceEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            path,
            kind,
            children: None,
        });
    }

    entries.sort_by(|left, right| {
        let left_kind = matches!(left.kind, WorkspaceEntryKind::File);
        let right_kind = matches!(right.kind, WorkspaceEntryKind::File);
        left_kind
            .cmp(&right_kind)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{
        LAST_WORKSPACE_VERSION, SearchDocumentInput, WorkspaceSessionState, discover,
        load_last_workspace, read_directory, replace_unopened_search_files, resolve_file,
        save_last_workspace, search_open_documents, search_workspace,
    };
    use crate::protocol::{
        ReplaceWorkspaceSearchResultsRequest, WorkspaceSearchFileReplacement,
        WorkspaceSearchMatchReplacement,
    };
    use functor_core::workspace::{Workspace, WorkspaceEntry, WorkspaceEntryKind, WorkspaceError};
    use std::fs;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn deterministic_fixture_has_stable_domain_data() {
        let fixture = Workspace {
            root: "fixture".into(),
            entries: vec![
                WorkspaceEntry {
                    name: "Cargo.toml".into(),
                    path: "fixture/Cargo.toml".into(),
                    kind: WorkspaceEntryKind::File,
                    children: None,
                },
                WorkspaceEntry {
                    name: "src".into(),
                    path: "fixture/src".into(),
                    kind: WorkspaceEntryKind::Directory,
                    children: None,
                },
            ],
        };

        assert_eq!(fixture.root, std::path::PathBuf::from("fixture"));
        assert_eq!(fixture.entries[0].kind, WorkspaceEntryKind::File);
        assert_eq!(fixture.entries[1].kind, WorkspaceEntryKind::Directory);
        assert_eq!(fixture.entries[1].children, None);
    }

    #[test]
    fn discovers_sorted_root_entries_without_eagerly_reading_subdirectories() {
        let root = unique_test_directory("workspace");
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("README.md"), "# Functor").unwrap();
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();

        let workspace = discover(&root).unwrap();
        let canonical_root = fs::canonicalize(&root).unwrap();

        assert_eq!(workspace.root, canonical_root);
        assert_eq!(
            workspace
                .entries
                .iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            vec!["src", "Cargo.toml", "README.md"]
        );
        assert_eq!(workspace.entries[0].kind, WorkspaceEntryKind::Directory);
        assert!(workspace.entries[0].children.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reads_a_nested_directory_on_demand() {
        let root = unique_test_directory("workspace-nested");
        fs::create_dir(root.join("src")).unwrap();
        fs::write(root.join("src").join("main.rs"), "fn main() {}").unwrap();

        let entries = read_directory(&root, "src").unwrap();
        let canonical_root = fs::canonicalize(&root).unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "main.rs");
        assert_eq!(entries[0].kind, WorkspaceEntryKind::File);
        assert_eq!(entries[0].path, canonical_root.join("src").join("main.rs"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resolves_only_files_inside_the_workspace() {
        let root = unique_test_directory("workspace-resolve");
        fs::write(root.join("README.md"), "# Functor").unwrap();
        let canonical_root = fs::canonicalize(&root).unwrap();
        assert_eq!(
            resolve_file(&root, "README.md").unwrap(),
            canonical_root.join("README.md")
        );
        assert!(matches!(
            resolve_file(&root, "../outside.md"),
            Err(WorkspaceError::InvalidPath(_))
        ));
        assert!(matches!(
            resolve_file(&root, "missing.md"),
            Err(WorkspaceError::ReadFailed { .. })
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn persists_and_restores_the_last_workspace_root() {
        let app_data = unique_test_directory("workspace-persistence");
        let state_path = app_data.join("state").join("last-workspace.json");
        let first_root = unique_test_directory("workspace-first");
        let second_root = unique_test_directory("workspace-second");
        let first_state = WorkspaceSessionState {
            root: first_root.clone(),
            open_files: vec!["src/main.rs".into(), "README.md".into()],
            active_file: Some("README.md".into()),
        };
        let second_state = WorkspaceSessionState {
            root: second_root.clone(),
            open_files: Vec::new(),
            active_file: None,
        };

        assert_eq!(load_last_workspace(&state_path).unwrap(), None);
        save_last_workspace(&state_path, &first_state).unwrap();
        assert_eq!(load_last_workspace(&state_path).unwrap(), Some(first_state));
        save_last_workspace(&state_path, &second_state).unwrap();
        assert_eq!(
            load_last_workspace(&state_path).unwrap(),
            Some(second_state)
        );

        fs::remove_dir_all(app_data).unwrap();
        fs::remove_dir_all(first_root).unwrap();
        fs::remove_dir_all(second_root).unwrap();
    }

    #[test]
    fn rejects_an_unsupported_saved_workspace_version() {
        let app_data = unique_test_directory("workspace-version");
        let state_path = app_data.join("last-workspace.json");
        let saved = super::LastWorkspace {
            version: LAST_WORKSPACE_VERSION + 1,
            root: app_data.clone(),
            open_files: Vec::new(),
            active_file: None,
        };
        fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();

        assert!(matches!(
            load_last_workspace(&state_path),
            Err(WorkspaceError::ReadFailed { .. })
        ));
        fs::remove_dir_all(app_data).unwrap();
    }

    #[test]
    fn restores_version_one_root_only_state() {
        let app_data = unique_test_directory("workspace-version-one");
        let state_path = app_data.join("last-workspace.json");
        let root = unique_test_directory("workspace-version-one-root");
        let saved = serde_json::json!({
            "version": 1,
            "root": root,
        });
        fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();

        assert_eq!(
            load_last_workspace(&state_path).unwrap(),
            Some(WorkspaceSessionState {
                root: root.clone(),
                open_files: Vec::new(),
                active_file: None,
            })
        );

        fs::remove_dir_all(app_data).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_saved_open_file_paths_that_escape_the_workspace() {
        let app_data = unique_test_directory("workspace-invalid-session");
        let state_path = app_data.join("last-workspace.json");
        let root = unique_test_directory("workspace-invalid-session-root");
        let saved = super::LastWorkspace {
            version: LAST_WORKSPACE_VERSION,
            root: root.clone(),
            open_files: vec!["../outside.rs".into()],
            active_file: None,
        };
        fs::write(&state_path, serde_json::to_vec(&saved).unwrap()).unwrap();

        assert!(matches!(
            load_last_workspace(&state_path),
            Err(WorkspaceError::ReadFailed { .. })
        ));

        fs::remove_dir_all(app_data).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_a_non_directory_root() {
        let root =
            std::env::temp_dir().join(format!("functors-workspace-file-{}", std::process::id()));
        fs::write(&root, "file").unwrap();

        assert_eq!(
            discover(&root),
            Err(WorkspaceError::InvalidRoot(root.clone()))
        );
        fs::remove_file(root).unwrap();
    }

    #[test]
    fn searches_all_workspace_text_case_insensitively_including_gitignored_files() {
        let root = unique_test_directory("workspace-search");
        fs::create_dir(root.join("src")).unwrap();
        fs::create_dir(root.join("ignored")).unwrap();
        fs::write(root.join(".gitignore"), "ignored/\n").unwrap();
        fs::write(
            root.join("src").join("main.rs"),
            "fn main() { println!(\"Hello\"); }\n",
        )
        .unwrap();
        fs::write(root.join("ignored").join("vendor.rs"), "hello\n").unwrap();

        let result = search_workspace_test(&root, "hello", false, Vec::new());

        assert_eq!(result.hits.len(), 2, "{:?}", result.hits);
        assert!(
            result
                .hits
                .iter()
                .any(|hit| hit.relative_path == "src/main.rs")
        );
        assert!(
            result
                .hits
                .iter()
                .any(|hit| hit.relative_path == "ignored/vendor.rs")
        );
        assert_eq!(result.hits[0].line_number, 1);
        assert!(result.hits[0].source_fingerprint.is_some());
        assert!(result.hits[0].workspace_root.is_some());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn searches_open_buffer_overrides_and_returns_monaco_utf16_columns() {
        let root = unique_test_directory("workspace-search-buffer");
        let path = root.join("main.rs");
        fs::write(&path, "the saved file has no match\n").unwrap();
        let canonical_path = fs::canonicalize(&path).unwrap();
        let result = search_workspace_test(
            &root,
            "hello",
            false,
            vec![SearchDocumentInput {
                document_id: 7,
                revision: 3,
                title: "main.rs".into(),
                path: Some(canonical_path),
                text: "x 🦀 Hello\n".into(),
            }],
        );

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].column, 6);
        assert_eq!(result.hits[0].match_length, 5);
        assert_eq!(result.hits[0].document_id, Some(7));
        assert_eq!(result.hits[0].revision, Some(3));
        assert_eq!(result.hits[0].range_offset, 5);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn workspace_search_includes_open_buffers_under_ignored_directories() {
        let root = unique_test_directory("workspace-search-ignored-buffer");
        let ignored_directory = root.join("node_modules");
        fs::create_dir(&ignored_directory).unwrap();
        let path = ignored_directory.join("main.rs");
        fs::write(&path, "saved text").unwrap();
        let result = search_workspace_test(
            &root,
            "hello",
            true,
            vec![SearchDocumentInput {
                document_id: 7,
                revision: 3,
                title: "main.rs".into(),
                path: Some(fs::canonicalize(path).unwrap()),
                text: "hello from open buffer".into(),
            }],
        );

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].relative_path, "node_modules/main.rs");
        assert_eq!(result.hits[0].document_id, Some(7));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn returns_all_workspace_search_matches() {
        let root = unique_test_directory("workspace-search-limit");
        fs::write(root.join("many.txt"), "hit\n".repeat(1_001)).unwrap();

        let result = search_workspace_test(&root, "hit", true, Vec::new());

        assert_eq!(result.hits.len(), 1_001);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replaces_unopened_workspace_matches_without_loading_editor_documents() {
        let root = unique_test_directory("workspace-search-replace-closed");
        let file_path = root.join("closed.txt");
        fs::write(&file_path, "🦀 hit hit\r\n").unwrap();
        let search = search_workspace_test(&root, "hit", true, Vec::new());
        let source_fingerprint = search.hits[0].source_fingerprint.clone().unwrap();
        let request = ReplaceWorkspaceSearchResultsRequest {
            workspace_root: fs::canonicalize(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            replacement: "ok".into(),
            files: vec![WorkspaceSearchFileReplacement {
                relative_path: "closed.txt".into(),
                source_fingerprint,
                matches: search
                    .hits
                    .iter()
                    .map(|hit| WorkspaceSearchMatchReplacement {
                        range_offset: hit.range_offset,
                        range_length: hit.range_length,
                        matched_text: hit.matched_text.clone(),
                    })
                    .collect(),
            }],
        };

        let response = replace_unopened_search_files(&root, request).unwrap();

        assert_eq!(response.replaced, 2);
        assert_eq!(response.files_written, 1);
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "🦀 ok ok\n");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stale_unopened_match_preflight_leaves_other_files_unchanged() {
        let root = unique_test_directory("workspace-search-replace-stale");
        let first_path = root.join("first.txt");
        let second_path = root.join("second.txt");
        fs::write(&first_path, "hit first").unwrap();
        fs::write(&second_path, "hit second").unwrap();
        let search = search_workspace_test(&root, "hit", true, Vec::new());
        let workspace_root = fs::canonicalize(&root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let files = search
            .hits
            .iter()
            .map(|hit| WorkspaceSearchFileReplacement {
                relative_path: hit.relative_path.clone(),
                source_fingerprint: hit.source_fingerprint.clone().unwrap(),
                matches: vec![WorkspaceSearchMatchReplacement {
                    range_offset: hit.range_offset,
                    range_length: hit.range_length,
                    matched_text: hit.matched_text.clone(),
                }],
            })
            .collect();
        fs::write(&second_path, "changed second").unwrap();

        let result = replace_unopened_search_files(
            &root,
            ReplaceWorkspaceSearchResultsRequest {
                workspace_root,
                replacement: "done".into(),
                files,
            },
        );

        assert_eq!(
            result.unwrap_err().code,
            crate::protocol::BridgeErrorCode::StaleRevision
        );
        assert_eq!(fs::read_to_string(first_path).unwrap(), "hit first");
        assert_eq!(fs::read_to_string(second_path).unwrap(), "changed second");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn replacing_with_identical_text_does_not_write_a_closed_file() {
        let root = unique_test_directory("workspace-search-replace-noop");
        let file_path = root.join("closed.txt");
        fs::write(&file_path, "hit").unwrap();
        let search = search_workspace_test(&root, "hit", true, Vec::new());
        let request = ReplaceWorkspaceSearchResultsRequest {
            workspace_root: fs::canonicalize(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            replacement: "hit".into(),
            files: vec![WorkspaceSearchFileReplacement {
                relative_path: "closed.txt".into(),
                source_fingerprint: search.hits[0].source_fingerprint.clone().unwrap(),
                matches: vec![WorkspaceSearchMatchReplacement {
                    range_offset: search.hits[0].range_offset,
                    range_length: search.hits[0].range_length,
                    matched_text: search.hits[0].matched_text.clone(),
                }],
            }],
        };

        let response = replace_unopened_search_files(&root, request).unwrap();

        assert_eq!(response.replaced, 1);
        assert_eq!(response.files_written, 0);
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "hit");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn searches_all_open_documents_without_a_workspace() {
        let cancelled = AtomicBool::new(false);
        let result = search_open_documents(
            "hit",
            true,
            false,
            vec![
                SearchDocumentInput {
                    document_id: 4,
                    revision: 8,
                    title: "main.rs".into(),
                    path: None,
                    text: "🦀 hit\n".into(),
                },
                SearchDocumentInput {
                    document_id: 5,
                    revision: 2,
                    title: "notes.md".into(),
                    path: None,
                    text: "hit\n".into(),
                },
            ],
            "search-test",
            &cancelled,
        );

        assert_eq!(result.hits.len(), 2);
        assert_eq!(result.hits[0].document_id, Some(4));
        assert_eq!(result.hits[0].range_offset, 3);
        assert_eq!(result.hits[1].document_id, Some(5));
        assert!(!result.cancelled);
    }

    #[test]
    fn open_document_search_can_be_cancelled_before_scanning() {
        let cancelled = AtomicBool::new(true);
        let result = search_open_documents(
            "hit",
            true,
            false,
            vec![SearchDocumentInput {
                document_id: 4,
                revision: 8,
                title: "main.rs".into(),
                path: None,
                text: "hit".into(),
            }],
            "cancelled-search",
            &cancelled,
        );

        assert!(result.cancelled);
        assert!(result.hits.is_empty());
    }

    #[test]
    fn open_document_search_returns_an_empty_result_when_nothing_matches() {
        let cancelled = AtomicBool::new(false);
        let result = search_open_documents(
            "missing",
            true,
            false,
            vec![SearchDocumentInput {
                document_id: 4,
                revision: 8,
                title: "main.rs".into(),
                path: None,
                text: "present".into(),
            }],
            "no-match-search",
            &cancelled,
        );

        assert!(result.hits.is_empty());
        assert!(!result.cancelled);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn whole_word_search_respects_case_and_unicode_word_boundaries() {
        let text = "Cat cat_1 scatter _cat cat. café caféx écafé";
        let document = SearchDocumentInput {
            document_id: 4,
            revision: 8,
            title: "main.rs".into(),
            path: None,
            text: text.into(),
        };
        let cancelled = AtomicBool::new(false);

        let insensitive = search_open_documents(
            "cat",
            false,
            true,
            vec![document.clone()],
            "whole-word-insensitive",
            &cancelled,
        );
        assert_eq!(
            insensitive
                .hits
                .iter()
                .map(|hit| hit.matched_text.as_str())
                .collect::<Vec<_>>(),
            ["Cat", "cat"]
        );

        let sensitive = search_open_documents(
            "cat",
            true,
            true,
            vec![document],
            "whole-word-sensitive",
            &cancelled,
        );
        assert_eq!(
            sensitive
                .hits
                .iter()
                .map(|hit| hit.matched_text.as_str())
                .collect::<Vec<_>>(),
            ["cat"]
        );

        let unicode = search_open_documents(
            "café",
            false,
            true,
            vec![SearchDocumentInput {
                document_id: 4,
                revision: 8,
                title: "main.rs".into(),
                path: None,
                text: text.into(),
            }],
            "whole-word-unicode",
            &cancelled,
        );
        assert_eq!(
            unicode
                .hits
                .iter()
                .map(|hit| hit.matched_text.as_str())
                .collect::<Vec<_>>(),
            ["café"]
        );
    }

    fn search_workspace_test(
        root: &std::path::Path,
        query: &str,
        case_sensitive: bool,
        open_documents: Vec<SearchDocumentInput>,
    ) -> crate::protocol::WorkspaceSearchResponse {
        search_workspace(
            root,
            query,
            case_sensitive,
            false,
            open_documents,
            "workspace-search-test",
            &AtomicBool::new(false),
        )
        .unwrap()
    }

    fn unique_test_directory(label: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("functors-{label}-{}-{suffix}", std::process::id()));
        fs::create_dir(&root).unwrap();
        root
    }
}
