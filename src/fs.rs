use crate::model::{Entry, EntryKind};
use anyhow::Result;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

pub fn list_dir(path: &Path, show_hidden: bool) -> Result<Vec<Entry>> {
    let read = match fs::read_dir(path) {
        Ok(r) => r,
        Err(_) => return Ok(Vec::new()),
    };
    let mut entries: Vec<Entry> = Vec::new();
    for dent in read.flatten() {
        let name = dent.file_name();
        let is_hidden = name.to_string_lossy().starts_with('.');
        if !show_hidden && is_hidden {
            continue;
        }
        let path = dent.path();
        let entry = build_entry(name, path, is_hidden);
        entries.push(entry);
    }
    entries.sort_by(compare);
    Ok(entries)
}

fn build_entry(name: std::ffi::OsString, path: PathBuf, is_hidden: bool) -> Entry {
    let symlink_meta = fs::symlink_metadata(&path).ok();
    let is_symlink = symlink_meta
        .as_ref()
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false);

    if is_symlink {
        let target = fs::read_link(&path).ok();
        let resolved = fs::metadata(&path).ok();
        let kind = match resolved {
            Some(m) => EntryKind::Symlink {
                broken: false,
                points_to_dir: m.is_dir(),
            },
            None => EntryKind::Symlink {
                broken: true,
                points_to_dir: false,
            },
        };
        let (size, mtime) = match symlink_meta {
            Some(m) => (Some(m.len()), m.modified().ok()),
            None => (None, None),
        };
        return Entry {
            name,
            path,
            kind,
            size,
            mtime,
            is_hidden,
            symlink_target: target,
        };
    }

    let (kind, size, mtime) = match symlink_meta {
        Some(m) if m.is_dir() => (EntryKind::Dir, None, m.modified().ok()),
        Some(m) => (EntryKind::File, Some(m.len()), m.modified().ok()),
        None => (EntryKind::File, None, None),
    };

    Entry {
        name,
        path,
        kind,
        size,
        mtime,
        is_hidden,
        symlink_target: None,
    }
}

fn compare(a: &Entry, b: &Entry) -> Ordering {
    let a_dir = a.is_dir_like();
    let b_dir = b.is_dir_like();
    match (a_dir, b_dir) {
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        _ => a
            .name
            .to_string_lossy()
            .to_lowercase()
            .cmp(&b.name.to_string_lossy().to_lowercase()),
    }
}

pub fn parent_of(path: &Path) -> Option<PathBuf> {
    path.parent().map(PathBuf::from)
}

pub fn human_size(n: u64) -> String {
    const UNITS: &[&str] = &["B", "K", "M", "G", "T"];
    let mut size = n as f64;
    let mut i = 0;
    while size >= 1024.0 && i + 1 < UNITS.len() {
        size /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{}{}", n, UNITS[0])
    } else if size >= 100.0 {
        format!("{:.0}{}", size, UNITS[i])
    } else if size >= 10.0 {
        format!("{:.1}{}", size, UNITS[i])
    } else {
        format!("{:.2}{}", size, UNITS[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_size_basics() {
        assert_eq!(human_size(0), "0B");
        assert_eq!(human_size(512), "512B");
        assert_eq!(human_size(1024), "1.00K");
        assert_eq!(human_size(1024 * 1024), "1.00M");
        assert_eq!(human_size(1024 * 1024 * 3 / 2), "1.50M");
    }
}
