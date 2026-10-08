use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, UNIX_EPOCH};

use functor_core::file_io::FileStamp;
use serde::{Deserialize, Serialize};

use crate::protocol::{DocumentLanguage, MonacoPosition, MonacoSelection, RecoveryCandidate};

const RECOVERY_VERSION: u32 = 1;
const MAX_RECOVERY_FILES: usize = 100;
const MAX_RECOVERY_TEXT_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECOVERY_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECOVERY_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RECOVERY_WARNINGS: usize = 20;
static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoveryFileStamp {
    length: u64,
    modified: Option<(u64, u32)>,
}

impl From<&FileStamp> for RecoveryFileStamp {
    fn from(stamp: &FileStamp) -> Self {
        Self {
            length: stamp.length,
            modified: stamp.modified.and_then(|modified| {
                modified
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .map(|duration| (duration.as_secs(), duration.subsec_nanos()))
            }),
        }
    }
}

impl RecoveryFileStamp {
    pub fn into_file_stamp(self) -> FileStamp {
        FileStamp {
            length: self.length,
            modified: self
                .modified
                .map(|(seconds, nanoseconds)| UNIX_EPOCH + Duration::new(seconds, nanoseconds)),
        }
    }

    fn is_valid(&self) -> bool {
        self.modified.is_none_or(|(seconds, nanoseconds)| {
            nanoseconds < 1_000_000_000
                && UNIX_EPOCH
                    .checked_add(Duration::new(seconds, nanoseconds))
                    .is_some()
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecoverySnapshot {
    version: u32,
    pub recovery_id: String,
    session_id: String,
    generation: u64,
    pub title: String,
    pub path: Option<PathBuf>,
    pub identity_path: Option<PathBuf>,
    pub text: String,
    pub language: DocumentLanguage,
    pub revision: u64,
    pub cursor: MonacoPosition,
    pub selection: Option<MonacoSelection>,
    pub file_stamp: Option<RecoveryFileStamp>,
    pub tab_index: usize,
    pub is_active: bool,
}

impl RecoverySnapshot {
    pub fn new(
        recovery_id: String,
        session_id: String,
        generation: u64,
        title: String,
        path: Option<PathBuf>,
        identity_path: Option<PathBuf>,
        text: String,
        language: DocumentLanguage,
        revision: u64,
        cursor: MonacoPosition,
        selection: Option<MonacoSelection>,
        file_stamp: Option<RecoveryFileStamp>,
        tab_index: usize,
        is_active: bool,
    ) -> Self {
        Self {
            version: RECOVERY_VERSION,
            recovery_id,
            session_id,
            generation,
            title,
            path,
            identity_path,
            text,
            language,
            revision,
            cursor,
            selection,
            file_stamp,
            tab_index,
            is_active,
        }
    }

    pub fn file_stamp(&self) -> Option<FileStamp> {
        self.file_stamp
            .clone()
            .map(RecoveryFileStamp::into_file_stamp)
    }
}

#[derive(Debug)]
pub struct RecoveryError(String);

impl fmt::Display for RecoveryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub fn save_snapshot(
    directory: impl AsRef<Path>,
    snapshot: &RecoverySnapshot,
) -> Result<(), RecoveryError> {
    validate_snapshot(snapshot)?;
    let directory = directory.as_ref();
    fs::create_dir_all(directory).map_err(|error| {
        recovery_error(format!(
            "Could not create recovery directory {}: {error}",
            directory.display()
        ))
    })?;

    let path = snapshot_path(directory, &snapshot.recovery_id)?;
    let contents = serde_json::to_vec(snapshot)
        .map_err(|error| recovery_error(format!("Could not encode recovery data: {error}")))?;
    if contents.len() as u64 > MAX_RECOVERY_FILE_BYTES {
        return Err(recovery_error(format!(
            "Recovery data for {} exceeds the per-document storage limit.",
            snapshot.title
        )));
    }
    ensure_storage_capacity(directory, &path, contents.len() as u64)?;

    let temporary_path = directory.join(format!(
        ".{}.tmp-{}-{}",
        snapshot.recovery_id,
        std::process::id(),
        NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temporary_path, contents).map_err(|error| {
        recovery_error(format!(
            "Could not write recovery data for {}: {error}",
            snapshot.title
        ))
    })?;
    if let Err(rename_error) = fs::rename(&temporary_path, &path) {
        let cleanup_error = fs::remove_file(&temporary_path).err();
        let message = cleanup_error.map_or_else(
            || rename_error.to_string(),
            |error| format!("{rename_error}; temporary-file cleanup also failed: {error}"),
        );
        return Err(recovery_error(format!(
            "Could not commit recovery data for {}: {message}",
            snapshot.title
        )));
    }
    Ok(())
}

pub fn list_snapshots(
    directory: impl AsRef<Path>,
) -> Result<(Vec<RecoverySnapshot>, Vec<String>), RecoveryError> {
    let directory = directory.as_ref();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()));
        }
        Err(error) => {
            return Err(recovery_error(format!(
                "Could not read recovery directory {}: {error}",
                directory.display()
            )));
        }
    };

    let mut paths = Vec::new();
    let mut warnings = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) if entry.file_type().is_ok_and(|kind| kind.is_file()) => {
                let path = entry.path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "json")
                {
                    paths.push(path);
                }
            }
            Ok(_) => {}
            Err(error) => record_warning(
                &mut warnings,
                format!("Could not inspect recovery directory entry: {error}"),
            ),
        }
    }
    paths.sort();

    let mut snapshots = Vec::new();
    let mut total_bytes = 0u64;
    for path in paths {
        if snapshots.len() >= MAX_RECOVERY_FILES {
            record_warning(
                &mut warnings,
                "Additional recovery snapshots were not loaded because the count limit was reached."
                    .to_owned(),
            );
            break;
        }
        let metadata = match fs::metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                record_warning(
                    &mut warnings,
                    format!(
                        "Could not inspect recovery file {}: {error}",
                        path.display()
                    ),
                );
                continue;
            }
        };
        let next_total = total_bytes.saturating_add(metadata.len());
        if metadata.len() > MAX_RECOVERY_FILE_BYTES || next_total > MAX_RECOVERY_TOTAL_BYTES {
            record_warning(
                &mut warnings,
                format!(
                    "Recovery file {} exceeds configured storage limits.",
                    path.display()
                ),
            );
            continue;
        }
        match read_snapshot(&path) {
            Ok(snapshot) => {
                total_bytes = next_total;
                snapshots.push(snapshot);
            }
            Err(error) => record_warning(
                &mut warnings,
                format!("Recovery file {} was ignored: {error}", path.display()),
            ),
        }
    }
    snapshots.sort_by_key(|snapshot| (snapshot.tab_index, snapshot.title.clone()));
    Ok((snapshots, warnings))
}

pub fn load_snapshot(
    directory: impl AsRef<Path>,
    recovery_id: &str,
) -> Result<RecoverySnapshot, RecoveryError> {
    let path = snapshot_path(directory.as_ref(), recovery_id)?;
    read_snapshot(&path)
}

pub fn delete_snapshot(
    directory: impl AsRef<Path>,
    recovery_id: &str,
) -> Result<(), RecoveryError> {
    let path = snapshot_path(directory.as_ref(), recovery_id)?;
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(recovery_error(format!(
            "Could not remove recovery file {}: {error}",
            path.display()
        ))),
    }
}

pub fn delete_all(directory: impl AsRef<Path>) -> Result<(), RecoveryError> {
    let directory = directory.as_ref();
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(recovery_error(format!(
                "Could not read recovery directory {}: {error}",
                directory.display()
            )));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            recovery_error(format!("Could not inspect recovery directory: {error}"))
        })?;
        if entry.file_type().is_ok_and(|kind| kind.is_file())
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        {
            fs::remove_file(entry.path()).map_err(|error| {
                recovery_error(format!(
                    "Could not remove recovery file {}: {error}",
                    entry.path().display()
                ))
            })?;
        }
    }
    Ok(())
}

pub fn candidate(snapshot: &RecoverySnapshot) -> RecoveryCandidate {
    RecoveryCandidate {
        recovery_id: snapshot.recovery_id.clone(),
        title: snapshot.title.clone(),
        path: snapshot
            .path
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned()),
    }
}

fn read_snapshot(path: &Path) -> Result<RecoverySnapshot, RecoveryError> {
    let metadata = fs::metadata(path)
        .map_err(|error| recovery_error(format!("Could not inspect recovery file: {error}")))?;
    if metadata.len() > MAX_RECOVERY_FILE_BYTES {
        return Err(recovery_error(
            "The recovery file exceeds the per-document storage limit.".to_owned(),
        ));
    }
    let contents = fs::read(path)
        .map_err(|error| recovery_error(format!("Could not read recovery file: {error}")))?;
    let snapshot: RecoverySnapshot = serde_json::from_slice(&contents)
        .map_err(|error| recovery_error(format!("The recovery data is invalid: {error}")))?;
    validate_snapshot(&snapshot)?;
    let expected_id = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| recovery_error("The recovery file name is invalid.".to_owned()))?;
    if snapshot.recovery_id != expected_id {
        return Err(recovery_error(
            "The recovery ID does not match its file name.".to_owned(),
        ));
    }
    Ok(snapshot)
}

fn validate_snapshot(snapshot: &RecoverySnapshot) -> Result<(), RecoveryError> {
    if snapshot.version != RECOVERY_VERSION
        || !is_valid_id(&snapshot.recovery_id)
        || !is_valid_id(&snapshot.session_id)
        || snapshot.text.len() > MAX_RECOVERY_TEXT_BYTES
        || snapshot
            .path
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        || snapshot
            .identity_path
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
        || snapshot.path.is_some() != snapshot.identity_path.is_some()
        || snapshot.path.is_some() != snapshot.file_stamp.is_some()
        || snapshot
            .file_stamp
            .as_ref()
            .is_some_and(|stamp| !stamp.is_valid())
    {
        return Err(recovery_error(
            "The recovery data failed validation or exceeded configured limits.".to_owned(),
        ));
    }
    Ok(())
}

fn ensure_storage_capacity(
    directory: &Path,
    target: &Path,
    new_file_bytes: u64,
) -> Result<(), RecoveryError> {
    let entries = fs::read_dir(directory).map_err(|error| {
        recovery_error(format!(
            "Could not inspect recovery storage {}: {error}",
            directory.display()
        ))
    })?;
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    for entry in entries {
        let entry = entry.map_err(|error| {
            recovery_error(format!("Could not inspect recovery storage: {error}"))
        })?;
        if entry.file_type().is_ok_and(|kind| kind.is_file())
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "json")
        {
            if entry.path() == target {
                continue;
            }
            file_count += 1;
            total_bytes = total_bytes.saturating_add(
                entry
                    .metadata()
                    .map_err(|error| {
                        recovery_error(format!("Could not inspect recovery file: {error}"))
                    })?
                    .len(),
            );
        }
    }
    if file_count >= MAX_RECOVERY_FILES
        || total_bytes.saturating_add(new_file_bytes) > MAX_RECOVERY_TOTAL_BYTES
    {
        return Err(recovery_error(
            "Recovery storage is full; discard older recovery data before continuing.".to_owned(),
        ));
    }
    Ok(())
}

fn snapshot_path(directory: &Path, recovery_id: &str) -> Result<PathBuf, RecoveryError> {
    if !is_valid_id(recovery_id) {
        return Err(recovery_error("The recovery ID is invalid.".to_owned()));
    }
    Ok(directory.join(format!("{recovery_id}.json")))
}

fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn record_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_RECOVERY_WARNINGS {
        warnings.push(warning);
    }
}

fn recovery_error(message: String) -> RecoveryError {
    RecoveryError(message)
}

#[cfg(test)]
mod tests {
    use super::{
        RecoveryFileStamp, RecoverySnapshot, candidate, delete_all, delete_snapshot,
        list_snapshots, load_snapshot, save_snapshot,
    };
    use crate::protocol::{DocumentLanguage, MonacoPosition};
    use functor_core::file_io::FileStamp;
    use std::fs;
    use std::path::PathBuf;
    use std::time::SystemTime;

    #[test]
    fn recovery_snapshot_round_trips_and_can_be_deleted() {
        let directory = test_directory("roundtrip");
        let snapshot = test_snapshot("untitled-1-2", "fn main() {}\n");

        save_snapshot(&directory, &snapshot).unwrap();

        let (snapshots, warnings) = list_snapshots(&directory).unwrap();
        assert!(warnings.is_empty());
        assert_eq!(snapshots, vec![snapshot.clone()]);
        assert_eq!(
            load_snapshot(&directory, &snapshot.recovery_id).unwrap(),
            snapshot
        );
        assert_eq!(candidate(&snapshots[0]).title, "notes.rs");

        delete_snapshot(&directory, &snapshots[0].recovery_id).unwrap();
        assert!(list_snapshots(&directory).unwrap().0.is_empty());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_file_replacement_keeps_only_the_latest_contents() {
        let directory = test_directory("replace");
        let first = test_snapshot("untitled-1-3", "first");
        let second = test_snapshot("untitled-1-3", "second");

        save_snapshot(&directory, &first).unwrap();
        save_snapshot(&directory, &second).unwrap();

        assert_eq!(
            load_snapshot(&directory, &second.recovery_id).unwrap().text,
            "second"
        );
        delete_all(&directory).unwrap();
        assert!(list_snapshots(&directory).unwrap().0.is_empty());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovery_ids_cannot_escape_the_storage_directory() {
        let directory = test_directory("path-check");
        assert!(load_snapshot(&directory, "../other").is_err());
        assert!(delete_snapshot(&directory, "..").is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_inconsistent_saved_file_metadata() {
        let directory = test_directory("invalid");
        let mut snapshot = test_snapshot("file-1", "text");
        snapshot.path = Some(directory.join("main.rs"));
        snapshot.file_stamp = None;

        assert!(save_snapshot(&directory, &snapshot).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn rejects_unrepresentable_recovery_timestamps() {
        let directory = test_directory("timestamp");
        let mut snapshot = test_snapshot("file-2", "text");
        snapshot.path = Some(directory.join("main.rs"));
        snapshot.identity_path = snapshot.path.clone();
        snapshot.file_stamp = Some(RecoveryFileStamp {
            length: 4,
            modified: Some((u64::MAX, 0)),
        });

        assert!(save_snapshot(&directory, &snapshot).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preserves_file_stamp_precision_for_external_change_checks() {
        let stamp = FileStamp {
            length: 42,
            modified: Some(SystemTime::now()),
        };

        assert_eq!(RecoveryFileStamp::from(&stamp).into_file_stamp(), stamp);
    }

    fn test_snapshot(recovery_id: &str, text: &str) -> RecoverySnapshot {
        RecoverySnapshot::new(
            recovery_id.into(),
            "session-1".into(),
            1,
            "notes.rs".into(),
            None,
            None,
            text.into(),
            DocumentLanguage::Rust,
            1,
            MonacoPosition {
                line_number: 1,
                column: 1,
            },
            None,
            None,
            0,
            true,
        )
    }

    fn test_directory(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("functors-recovery-{name}-{}", std::process::id()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
