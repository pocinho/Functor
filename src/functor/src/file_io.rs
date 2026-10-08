use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use functor_core::file_io::{FileError, FileStamp, LoadedFile, SavedFile, normalize_newlines};
use functor_core::syntax::SyntaxLanguage;

static NEXT_TEMP_FILE_ID: AtomicU64 = AtomicU64::new(1);

pub fn load(path: impl AsRef<Path>) -> Result<LoadedFile, FileError> {
    let requested_path = path.as_ref().to_path_buf();
    let bytes = fs::read(&requested_path).map_err(|error| FileError::Read {
        path: requested_path.clone(),
        message: error.to_string(),
    })?;
    let text = String::from_utf8(bytes)
        .map(|text| normalize_newlines(text.strip_prefix('\u{feff}').unwrap_or(&text)))
        .map_err(|_| FileError::InvalidUtf8 {
            path: requested_path.clone(),
        })?;
    let identity_path = fs::canonicalize(&requested_path).map_err(|error| FileError::Read {
        path: requested_path.clone(),
        message: error.to_string(),
    })?;
    let language = SyntaxLanguage::from_path(&identity_path);
    Ok(LoadedFile {
        stamp: file_stamp(&identity_path).map_err(|error| FileError::Read {
            path: identity_path.clone(),
            message: error.to_string(),
        })?,
        path: requested_path,
        identity_path,
        text,
        language,
    })
}

pub fn save_if_unchanged(
    path: impl AsRef<Path>,
    text: &str,
    expected: Option<&FileStamp>,
) -> Result<SavedFile, FileError> {
    let path = path.as_ref().to_path_buf();
    if let Some(expected) = expected {
        let current = file_stamp(&path).map_err(|error| FileError::Write {
            path: path.clone(),
            message: error.to_string(),
        })?;
        if &current != expected {
            return Err(FileError::Modified { path });
        }
    }
    let identity_path = canonical_identity_path(&path).map_err(|error| FileError::Write {
        path: path.clone(),
        message: error.to_string(),
    })?;
    let temporary_path =
        write_temporary_file(&path, &normalize_newlines(text)).map_err(|error| {
            FileError::Write {
                path: path.clone(),
                message: error.to_string(),
            }
        })?;

    if let Err(error) = replace_file(&temporary_path, &path) {
        let cleanup_error = fs::remove_file(&temporary_path).err();
        return Err(FileError::Write {
            path: path.clone(),
            message: cleanup_error.map_or_else(
                || error.to_string(),
                |cleanup| format!("{error}; temporary-file cleanup also failed: {cleanup}"),
            ),
        });
    }

    let stamp = file_stamp(&path).map_err(|error| FileError::Write {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok(SavedFile {
        path,
        identity_path,
        stamp,
    })
}

pub fn canonical_identity_path(path: &Path) -> io::Result<std::path::PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "The save destination has no file name.",
        )
    })?;
    Ok(fs::canonicalize(parent)?.join(file_name))
}

fn write_temporary_file(path: &Path, text: &str) -> io::Result<std::path::PathBuf> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    path.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "The save destination has no file name.",
        )
    })?;

    loop {
        let id = NEXT_TEMP_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let temporary_path = parent.join(format!(".functor-save-{}-{id}.tmp", std::process::id(),));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(text.as_bytes())
                    .and_then(|()| file.sync_all());
                if let Err(error) = result {
                    let cleanup_error = fs::remove_file(&temporary_path).err();
                    let message = cleanup_error.map_or_else(
                        || error.to_string(),
                        |cleanup| format!("{error}; temporary-file cleanup also failed: {cleanup}"),
                    );
                    return Err(io::Error::new(error.kind(), message));
                }
                return Ok(temporary_path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
}

#[cfg(windows)]
fn replace_file(temporary_path: &Path, path: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_WRITE_THROUGH, MoveFileExW, REPLACEFILE_WRITE_THROUGH, ReplaceFileW,
    };

    let target_exists = path.exists();
    let temporary_path = temporary_path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let path = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    if target_exists {
        let succeeded = unsafe {
            ReplaceFileW(
                path.as_ptr(),
                temporary_path.as_ptr(),
                std::ptr::null(),
                REPLACEFILE_WRITE_THROUGH,
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if succeeded != 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::NotFound {
            return Err(error);
        }
    }

    let succeeded = unsafe {
        MoveFileExW(
            temporary_path.as_ptr(),
            path.as_ptr(),
            MOVEFILE_WRITE_THROUGH,
        )
    };
    if succeeded == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(temporary_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temporary_path, path)
}

fn file_stamp(path: &Path) -> std::io::Result<FileStamp> {
    let metadata = fs::metadata(path)?;
    Ok(FileStamp {
        length: metadata.len(),
        modified: metadata.modified().ok(),
    })
}

#[cfg(test)]
mod tests {
    use super::{FileError, load, save_if_unchanged};
    use functor_core::syntax::SyntaxLanguage;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn loads_utf8_text_and_selects_language_from_extension() {
        let path = test_path("source.rs");
        fs::write(&path, "\u{feff}fn main() {}\r\n// second line\r").unwrap();

        let loaded = load(&path).unwrap();

        assert_eq!(loaded.path, path);
        assert_eq!(loaded.identity_path, fs::canonicalize(&path).unwrap());
        assert_eq!(loaded.text, "fn main() {}\n// second line\n");
        assert_eq!(loaded.language, SyntaxLanguage::Rust);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn saves_by_replacing_the_target_file() {
        let path = test_path("notes.md");
        fs::write(&path, "# Original\n").unwrap();
        save_if_unchanged(&path, "# Updated\r\n", None).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), "# Updated\n");
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn resolves_a_canonical_identity_for_a_new_destination() {
        let directory =
            std::env::temp_dir().join(format!("functors-file-identity-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("new.rs");

        assert_eq!(
            super::canonical_identity_path(&path).unwrap(),
            fs::canonicalize(&directory).unwrap().join("new.rs")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn reports_invalid_utf8_without_returning_partial_text() {
        let path = test_path("binary.txt");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        assert_eq!(
            load(&path),
            Err(FileError::InvalidUtf8 { path: path.clone() })
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejects_save_when_file_changed_after_load() {
        let path = test_path("changed.rs");
        fs::write(&path, "fn main() {}\n").unwrap();
        let loaded = load(&path).unwrap();
        fs::write(&path, "fn main() { println!(\"external\"); }\n").unwrap();

        assert_eq!(
            save_if_unchanged(&path, &loaded.text, Some(&loaded.stamp)),
            Err(FileError::Modified { path: path.clone() })
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "fn main() { println!(\"external\"); }\n"
        );
        fs::remove_file(path).unwrap();
    }

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("functors-file-{}-{name}", std::process::id()))
    }
}
