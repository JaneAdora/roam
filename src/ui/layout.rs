#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Columns {
    pub show_size: bool,
    pub show_mtime: bool,
    pub show_pinned: bool,
    pub show_preview: bool,
    pub name_max: u16,
    pub compact_icons: bool,
    pub too_narrow: bool,
}

pub fn choose_columns(width: u16) -> Columns {
    if width < 30 {
        Columns {
            show_size: false,
            show_mtime: false,
            show_pinned: false,
            show_preview: false,
            name_max: 8,
            compact_icons: true,
            too_narrow: true,
        }
    } else if width < 40 {
        Columns {
            show_size: false,
            show_mtime: false,
            show_pinned: false,
            show_preview: false,
            name_max: 16,
            compact_icons: true,
            too_narrow: false,
        }
    } else if width < 60 {
        Columns {
            show_size: true,
            show_mtime: false,
            show_pinned: true,
            show_preview: false,
            name_max: 22,
            compact_icons: false,
            too_narrow: false,
        }
    } else if width < 80 {
        Columns {
            show_size: true,
            show_mtime: true,
            show_pinned: true,
            show_preview: false,
            name_max: 28,
            compact_icons: false,
            too_narrow: false,
        }
    } else {
        Columns {
            show_size: true,
            show_mtime: true,
            show_pinned: true,
            show_preview: true,
            name_max: 36,
            compact_icons: false,
            too_narrow: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ultra_narrow_flags_warning() {
        let c = choose_columns(20);
        assert!(c.too_narrow);
        assert!(!c.show_preview);
        assert!(!c.show_pinned);
    }

    #[test]
    fn narrow_no_preview_no_pinned() {
        let c = choose_columns(35);
        assert!(!c.show_preview);
        assert!(!c.show_pinned);
    }

    #[test]
    fn medium_shows_pinned_no_preview() {
        let c = choose_columns(50);
        assert!(c.show_pinned);
        assert!(!c.show_preview);
    }

    #[test]
    fn wide_shows_preview() {
        let c = choose_columns(100);
        assert!(c.show_preview);
        assert!(c.show_pinned);
    }
}
