//! Resolving the program a workflow names (ADR-0012).
//!
//! A program is a bare name, looked up on the caller's `PATH`, or a repository-relative path that
//! must stay inside the repository. The resolved absolute path is what the user grants and what is
//! recorded as provenance, together with the SHA-256 of the file.

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use functor_core::{FunctorError, Result};
use functor_model::ContentDigest;
use sha2::{Digest, Sha256};

/// Whether the program names a path (it has a separator) rather than a command on `PATH`.
fn is_path(program: &str) -> bool {
    program.contains('/') || program.contains('\\')
}

fn is_file(path: &Path) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.is_file())
}

/// Drops the Windows verbatim prefix so paths read normally and compare with user-supplied ones.
pub(crate) fn plain(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

fn canonical(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok().map(plain)
}

/// Whether two paths name the same file. Windows paths compare case-insensitively.
pub(crate) fn same_file(a: &Path, b: &Path) -> bool {
    let (Some(a), Some(b)) = (canonical(a), canonical(b)) else {
        return false;
    };
    if cfg!(windows) {
        a.to_string_lossy().to_lowercase() == b.to_string_lossy().to_lowercase()
    } else {
        a == b
    }
}

/// Candidate file names for a bare command: on Windows, the name itself when it has an extension,
/// otherwise the name with each `PATHEXT` extension.
fn candidates(name: &str) -> Vec<String> {
    if !cfg!(windows) {
        return vec![name.to_owned()];
    }
    if Path::new(name).extension().is_some() {
        return vec![name.to_owned()];
    }
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    extensions
        .split(';')
        .filter(|extension| !extension.is_empty())
        .map(|extension| format!("{name}{extension}"))
        .collect()
}

/// Resolves `program` to an absolute path, or `None` when it does not exist.
///
/// # Errors
/// `PermissionDenied` when a repository-relative path leaves the repository (through `..` or a
/// link).
pub(crate) fn resolve(
    root: &Path,
    program: &str,
    path_var: Option<&OsStr>,
) -> Result<Option<PathBuf>> {
    if is_path(program) {
        let mut candidate = root.to_path_buf();
        candidate.extend(program.split(['/', '\\']));
        let Some(real) = canonical(&candidate) else {
            return Ok(None);
        };
        let inside = canonical(root).is_some_and(|root| real.starts_with(root));
        if !inside {
            return Err(FunctorError::permission_denied(
                "execute",
                format!("the program `{program}` leaves the repository"),
            ));
        }
        return Ok(is_file(&real).then_some(real));
    }

    let search = match path_var {
        Some(paths) => paths.to_os_string(),
        None => std::env::var_os("PATH").unwrap_or_default(),
    };
    for dir in std::env::split_paths(&search) {
        for name in candidates(program) {
            let candidate = dir.join(&name);
            if is_file(&candidate)
                && let Some(real) = canonical(&candidate)
            {
                return Ok(Some(real));
            }
        }
    }
    Ok(None)
}

/// The SHA-256 of a file, streamed; `None` when it cannot be read.
pub(crate) fn file_digest(path: &Path) -> Option<ContentDigest> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return None,
        }
    }
    Some(ContentDigest::from_bytes(hasher.finalize().into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exe(name: &str) -> String {
        format!("{name}{}", std::env::consts::EXE_SUFFIX)
    }

    #[test]
    fn a_relative_path_resolves_inside_the_repository() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("tools")).unwrap();
        let tool = dir.path().join("tools").join(exe("tool"));
        fs::write(&tool, b"x").unwrap();

        let found = resolve(dir.path(), &format!("tools/{}", exe("tool")), None)
            .unwrap()
            .unwrap();
        assert!(same_file(&found, &tool));
        assert_eq!(
            resolve(dir.path(), "tools\\missing", None).unwrap(),
            None,
            "a missing file is not an error"
        );
        assert_eq!(
            resolve(dir.path(), "tools", None).unwrap(),
            None,
            "a directory is not a program"
        );
    }

    #[test]
    fn a_relative_path_cannot_leave_the_repository() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("repo");
        fs::create_dir_all(&root).unwrap();
        fs::write(outer.path().join(exe("outside")), b"x").unwrap();
        for escape in [
            format!("../{}", exe("outside")),
            format!("tools/../../{}", exe("outside")),
        ] {
            let error = resolve(&root, &escape, None).unwrap_err();
            assert_eq!(
                error.kind(),
                functor_core::ErrorKind::PermissionDenied,
                "{escape}"
            );
        }
    }

    #[test]
    fn bare_names_are_looked_up_on_the_given_path_only() {
        let dir = tempfile::tempdir().unwrap();
        let tool = dir.path().join(exe("tool"));
        fs::write(&tool, b"x").unwrap();
        let path = std::env::join_paths([dir.path()]).unwrap();

        let found = resolve(dir.path(), "tool", Some(&path)).unwrap().unwrap();
        assert!(same_file(&found, &tool));
        let empty = std::env::join_paths(Vec::<PathBuf>::new()).unwrap();
        assert_eq!(resolve(dir.path(), "tool", Some(&empty)).unwrap(), None);
        assert_eq!(
            resolve(dir.path(), "no-such-tool", Some(&path)).unwrap(),
            None
        );
    }

    #[test]
    fn digests_stream_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("f");
        fs::write(&file, b"abc").unwrap();
        assert_eq!(
            file_digest(&file).unwrap().as_str(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(file_digest(&dir.path().join("missing")), None);
    }

    #[test]
    fn plain_strips_only_drive_verbatim_prefixes() {
        assert_eq!(
            plain(PathBuf::from(r"\\?\C:\a\b")),
            PathBuf::from(r"C:\a\b")
        );
        assert_eq!(
            plain(PathBuf::from(r"\\?\UNC\host\share")),
            PathBuf::from(r"\\?\UNC\host\share")
        );
        assert_eq!(plain(PathBuf::from("/a/b")), PathBuf::from("/a/b"));
    }
}
