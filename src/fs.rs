use crate::model::{Entry, EntryKind};
use anyhow::Result;
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

/// Compact relative age of a file's mtime, e.g. "5m", "3h", "2d", "4w", "6mo", "1y".
pub fn human_mtime(t: SystemTime) -> String {
    let secs = SystemTime::now()
        .duration_since(t)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if secs < 60 {
        format!("{}s", secs)
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3_600)
    } else if secs < 7 * 86_400 {
        format!("{}d", secs / 86_400)
    } else if secs < 30 * 86_400 {
        format!("{}w", secs / (7 * 86_400))
    } else if secs < 365 * 86_400 {
        format!("{}mo", secs / (30 * 86_400))
    } else {
        format!("{}y", secs / (365 * 86_400))
    }
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

/// Recursively find entries under `root` whose name contains `query`
/// (case-insensitive), to a bounded depth. Hidden entries are skipped unless
/// `show_hidden`; symlinked dirs are not descended into (loop-safe); results are
/// capped. Each result's `name` is set to its path RELATIVE to `root` for
/// display. `max_depth` = how many levels below `root` to search (depth 1 = the
/// direct children of `root`).
pub fn find_recursive(root: &Path, query: &str, show_hidden: bool, max_depth: usize) -> Vec<Entry> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    const CAP: usize = 1000;
    let mut out: Vec<Entry> = Vec::new();
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        let read = match fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for dent in read.flatten() {
            let name = dent.file_name();
            let is_hidden = name.to_string_lossy().starts_with('.');
            if !show_hidden && is_hidden {
                continue;
            }
            let path = dent.path();
            // symlink_metadata does not follow links, so symlinked dirs read as
            // non-dirs here and we never recurse into them (no cycles).
            let is_real_dir = fs::symlink_metadata(&path)
                .map(|m| m.is_dir())
                .unwrap_or(false);
            if name.to_string_lossy().to_lowercase().contains(&q) {
                let mut e = build_entry(name.clone(), path.clone(), is_hidden);
                if let Ok(rel) = path.strip_prefix(root) {
                    e.name = rel.as_os_str().to_os_string();
                }
                out.push(e);
                if out.len() >= CAP {
                    out.sort_by(compare);
                    return out;
                }
            }
            if is_real_dir && depth + 1 < max_depth {
                stack.push((path, depth + 1));
            }
        }
    }
    out.sort_by(compare);
    out
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

    #[test]
    fn find_recursive_matches_and_bounds_depth() {
        use std::fs as f;
        let base = std::env::temp_dir().join(format!("roam_find_test_{}", std::process::id()));
        let _ = f::remove_dir_all(&base);
        f::create_dir_all(base.join("d1/d2/d3")).unwrap();
        f::write(base.join("alpha_match.txt"), b"x").unwrap(); // depth 1
        f::write(base.join("d1/beta_match.txt"), b"x").unwrap(); // depth 2
        f::write(base.join("d1/d2/gamma_match.txt"), b"x").unwrap(); // depth 3
        f::write(base.join("d1/d2/d3/delta_match.txt"), b"x").unwrap(); // depth 4
        let names: Vec<String> = find_recursive(&base, "match", false, 3)
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert!(names.iter().any(|n| n.contains("alpha_match")), "{names:?}");
        assert!(names.iter().any(|n| n == "d1/beta_match.txt"), "{names:?}");
        assert!(names.iter().any(|n| n.ends_with("gamma_match.txt")), "{names:?}");
        assert!(!names.iter().any(|n| n.contains("delta_match")), "depth-4 leaked: {names:?}");
        let _ = f::remove_dir_all(&base);
    }

    #[test]
    fn find_recursive_empty_query_is_empty() {
        assert!(find_recursive(&std::env::temp_dir(), "", false, 3).is_empty());
    }
}
