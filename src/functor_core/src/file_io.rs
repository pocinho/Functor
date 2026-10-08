use std::path::PathBuf;
use std::time::SystemTime;

use crate::syntax::SyntaxLanguage;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedFile {
    pub path: PathBuf,
    pub identity_path: PathBuf,
    pub text: String,
    pub language: SyntaxLanguage,
    pub stamp: FileStamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileStamp {
    pub length: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedFile {
    pub path: PathBuf,
    pub identity_path: PathBuf,
    pub stamp: FileStamp,
}

pub fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    Read { path: PathBuf, message: String },
    InvalidUtf8 { path: PathBuf },
    Modified { path: PathBuf },
    Write { path: PathBuf, message: String },
}
