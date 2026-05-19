use std::ffi::OsString;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Dir,
    File,
    Symlink { broken: bool, points_to_dir: bool },
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: OsString,
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size: Option<u64>,
    pub mtime: Option<SystemTime>,
    pub is_hidden: bool,
    pub symlink_target: Option<PathBuf>,
}

impl Entry {
    pub fn display_name(&self) -> String {
        self.name.to_string_lossy().into_owned()
    }

    pub fn is_dir_like(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
            || matches!(
                self.kind,
                EntryKind::Symlink {
                    broken: false,
                    points_to_dir: true
                }
            )
    }

    pub fn is_broken_symlink(&self) -> bool {
        matches!(self.kind, EntryKind::Symlink { broken: true, .. })
    }
}
