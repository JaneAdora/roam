//! File-type icons. Three styles: Nerd Font glyphs (default, the eza/lsd set),
//! emoji, or plain ASCII. A narrow layout forces ASCII regardless of style so a
//! missing glyph never smears the list. Each returned str carries a trailing
//! space for column alignment. Nerd glyphs are spelled as `\u{...}` escapes so
//! the source is ASCII-clean and unambiguous.
use crate::model::{Entry, EntryKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum IconStyle {
    /// Nerd Font file-type glyphs (needs a patched font; what most terminals use).
    #[default]
    Nerd,
    /// Emoji fallback (folder/link only); works without a Nerd Font.
    Emoji,
    /// Single-letter ASCII; works in any terminal.
    Ascii,
}

impl IconStyle {
    /// Cycle Nerd -> Emoji -> Ascii -> Nerd (bound to the `I` key).
    pub fn next(self) -> Self {
        match self {
            IconStyle::Nerd => IconStyle::Emoji,
            IconStyle::Emoji => IconStyle::Ascii,
            IconStyle::Ascii => IconStyle::Nerd,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            IconStyle::Nerd => "nerd",
            IconStyle::Emoji => "emoji",
            IconStyle::Ascii => "ascii",
        }
    }
}

/// Icon (with trailing space) for `entry` under `style`. `compact` (a narrow
/// terminal) always falls back to ASCII.
pub fn icon_for(entry: &Entry, style: IconStyle, compact: bool) -> &'static str {
    if compact || style == IconStyle::Ascii {
        return ascii(entry);
    }
    match style {
        IconStyle::Emoji => emoji(entry),
        _ => nerd(entry),
    }
}

fn ascii(entry: &Entry) -> &'static str {
    match entry.kind {
        EntryKind::Dir => "d ",
        EntryKind::Symlink { broken: true, .. } => "x ",
        EntryKind::Symlink { .. } => "l ",
        EntryKind::File => "  ",
    }
}

fn emoji(entry: &Entry) -> &'static str {
    match entry.kind {
        EntryKind::Dir => "\u{1F4C1} ",                          // 📁
        EntryKind::Symlink { broken: true, .. } => "\u{26A0} ",  // ⚠
        EntryKind::Symlink { .. } => "\u{1F517} ",               // 🔗
        EntryKind::File => "  ",
    }
}

fn nerd(entry: &Entry) -> &'static str {
    match entry.kind {
        EntryKind::Dir => "\u{F024B} ",                          // nf-md-folder
        EntryKind::Symlink { broken: true, .. } => "\u{F071} ",  // warning triangle
        EntryKind::Symlink { .. } => "\u{F0C1} ",                // link
        EntryKind::File => nerd_file(&entry.display_name()),
    }
}

/// Pick a Nerd Font glyph from a filename's extension.
fn nerd_file(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        // images
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "webp" | "ico" | "tif" | "tiff"
        | "heic" | "heif" | "avif" | "jfif" | "tga" => "\u{F1C5} ", // file-image-o
        "svg" => "\u{F0721} ",                                      // nf-md-svg
        // documents
        "pdf" => "\u{F1C1} ",                                       // file-pdf-o
        "doc" | "docx" | "odt" | "rtf" => "\u{F1C2} ",              // file-word-o
        "xls" | "xlsx" | "ods" | "csv" | "tsv" => "\u{F1C3} ",      // file-excel-o
        "ppt" | "pptx" | "odp" => "\u{F1C4} ",                      // file-powerpoint-o
        "txt" | "text" | "log" => "\u{F0F6} ",                      // file-text-o
        "md" | "markdown" | "mdx" => "\u{F0354} ",                  // nf-md-language-markdown
        // archives
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "zst" | "7z" | "rar"
        | "lz" | "lzma" | "lz4" => "\u{F1C6} ",                     // file-archive-o
        // audio / video
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" | "wma" => "\u{F1C7} ",
        "mp4" | "mkv" | "mov" | "avi" | "webm" | "wmv" | "flv" | "m4v" => "\u{F1C8} ",
        // languages we have confident glyphs for
        "rs" => "\u{E7A8} ",                                        // rust
        "py" | "pyw" => "\u{E73C} ",                               // python
        "js" | "mjs" | "cjs" | "jsx" => "\u{E74E} ",              // javascript
        "ts" | "tsx" => "\u{E628} ",                              // typescript
        "html" | "htm" | "xhtml" => "\u{E736} ",                  // html5
        "css" | "scss" | "sass" | "less" => "\u{E749} ",         // css3
        "json" | "jsonc" => "\u{E60B} ",                          // nf-seti-json
        "toml" | "yaml" | "yml" | "ini" | "conf" | "cfg" | "env" => "\u{F013} ", // cog
        "sh" | "bash" | "zsh" | "fish" => "\u{F120} ",            // terminal
        // other source files -> generic code badge
        "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "go" | "rb" | "gem" | "java"
        | "kt" | "lua" | "php" | "swift" | "vim" | "el" | "clj" | "hs" | "scala"
        | "dart" | "pl" | "r" | "sql" => "\u{F1C9} ",             // file-code-o
        "lock" => "\u{F023} ",                                     // lock
        _ => "\u{F016} ",                                          // generic file-o
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Entry, EntryKind};
    use std::path::PathBuf;

    fn file(name: &str) -> Entry {
        Entry {
            name: name.into(),
            path: PathBuf::from(name),
            kind: EntryKind::File,
            size: None,
            mtime: None,
            is_hidden: false,
            symlink_target: None,
        }
    }

    #[test]
    fn style_cycles() {
        assert_eq!(IconStyle::Nerd.next(), IconStyle::Emoji);
        assert_eq!(IconStyle::Emoji.next(), IconStyle::Ascii);
        assert_eq!(IconStyle::Ascii.next(), IconStyle::Nerd);
    }

    #[test]
    fn compact_forces_ascii() {
        assert_eq!(icon_for(&file("a.png"), IconStyle::Nerd, true), "  ");
    }

    #[test]
    fn known_extensions_map_distinctly() {
        let png = icon_for(&file("a.png"), IconStyle::Nerd, false);
        let pdf = icon_for(&file("a.pdf"), IconStyle::Nerd, false);
        let svg = icon_for(&file("a.svg"), IconStyle::Nerd, false);
        let other = icon_for(&file("a.unknownext"), IconStyle::Nerd, false);
        assert_ne!(png, pdf);
        assert_ne!(png, svg);
        assert_ne!(pdf, svg);
        assert_eq!(other, "\u{F016} ");
        // case-insensitive
        assert_eq!(icon_for(&file("A.PNG"), IconStyle::Nerd, false), png);
    }
}
