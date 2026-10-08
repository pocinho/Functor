use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
    pub entries: Vec<WorkspaceEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: WorkspaceEntryKind,
    pub children: Option<Vec<WorkspaceEntry>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkspaceError {
    InvalidRoot(PathBuf),
    InvalidPath(PathBuf),
    ReadFailed { path: PathBuf, message: String },
}

impl Workspace {
    pub fn contains_directory(&self, path: &std::path::Path) -> bool {
        fn contains(entries: &[WorkspaceEntry], path: &std::path::Path) -> bool {
            entries.iter().any(|entry| {
                (entry.path == path && entry.kind == WorkspaceEntryKind::Directory)
                    || entry
                        .children
                        .as_deref()
                        .is_some_and(|children| contains(children, path))
            })
        }

        contains(&self.entries, path)
    }

    pub fn contains_file(&self, path: &std::path::Path) -> bool {
        fn contains(entries: &[WorkspaceEntry], path: &std::path::Path) -> bool {
            entries.iter().any(|entry| {
                (entry.path == path && entry.kind == WorkspaceEntryKind::File)
                    || entry
                        .children
                        .as_deref()
                        .is_some_and(|children| contains(children, path))
            })
        }

        contains(&self.entries, path)
    }

    pub fn set_directory_entries(
        &mut self,
        directory: &std::path::Path,
        entries: Vec<WorkspaceEntry>,
    ) -> bool {
        if directory == self.root {
            self.entries = entries;
            return true;
        }

        fn set_entries(
            current: &mut [WorkspaceEntry],
            directory: &std::path::Path,
            entries: &mut Option<Vec<WorkspaceEntry>>,
        ) -> bool {
            for entry in current {
                if entry.path == directory && entry.kind == WorkspaceEntryKind::Directory {
                    entry.children = entries.take();
                    return true;
                }
                if let Some(children) = &mut entry.children
                    && set_entries(children, directory, entries)
                {
                    return true;
                }
            }
            false
        }

        let mut entries = Some(entries);
        set_entries(&mut self.entries, directory, &mut entries)
    }
}

impl std::fmt::Display for WorkspaceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRoot(path) => {
                write!(
                    formatter,
                    "The selected workspace is not a directory: {}",
                    path.display()
                )
            }
            Self::InvalidPath(path) => {
                write!(
                    formatter,
                    "The path is not a workspace file or directory: {}",
                    path.display()
                )
            }
            Self::ReadFailed { path, message } => {
                write!(formatter, "Could not read {}: {message}", path.display())
            }
        }
    }
}
