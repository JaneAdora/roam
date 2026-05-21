//! Tiny, dependency-free syntax highlighter for the preview pane and modal.
//! Not a real parser: a lexical pass that colors comments, strings, numbers,
//! keywords and call-sites for C-family/front-end code, and tags/attributes for
//! markup. Good enough to skim code on the go; degrades to readable plain text.
use crate::ui::theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Code,
    Markup,
}

const KEYWORD: Color = theme::MAGENTA;
const STRING: Color = Color::Rgb(0x9e, 0xce, 0x6a);
const NUMBER: Color = Color::Rgb(0xe5, 0xc0, 0x7b);
const FUNC: Color = Color::Rgb(0x61, 0xaf, 0xef);
const TAG: Color = Color::Rgb(0xe0, 0x6c, 0x75);
const ATTR: Color = Color::Rgb(0xd1, 0x9a, 0x66);

fn comment_style() -> Style {
    Style::default()
        .fg(theme::LAVENDER)
        .add_modifier(Modifier::DIM | Modifier::ITALIC)
}
fn plain() -> Style {
    Style::default()
}
fn punct() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

const KEYWORDS: &[&str] = &[
    "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue",
    "debugger", "declare", "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "from", "function", "get", "if", "implements", "import", "in", "infer",
    "instanceof", "interface", "keyof", "let", "namespace", "new", "null", "of", "private",
    "protected", "public", "readonly", "return", "satisfies", "set", "static", "super", "switch",
    "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
    // rust-ish extras, harmless elsewhere
    "fn", "impl", "let", "mut", "pub", "struct", "trait", "use", "match", "move", "where", "mod",
    "dyn", "crate", "Self",
];

pub fn language_for(name: &str) -> Option<Lang> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "json" | "json5" | "css" | "scss" | "sass"
        | "less" | "rs" | "go" | "c" | "h" | "cpp" | "hpp" | "java" | "kt" | "swift" | "php" => {
            Some(Lang::Code)
        }
        "html" | "htm" | "xml" | "svg" | "vue" | "svelte" => Some(Lang::Markup),
        _ => None,
    }
}

pub fn to_lines(text: &str, lang: Lang) -> Vec<Line<'static>> {
    match lang {
        Lang::Code => code_lines(text),
        Lang::Markup => markup_lines(text),
    }
}

fn push_buf(spans: &mut Vec<Span<'static>>, buf: &mut String, style: Style) {
    if !buf.is_empty() {
        spans.push(Span::styled(std::mem::take(buf), style));
    }
}

fn find2(chars: &[char], from: usize, a: char, b: char) -> Option<usize> {
    (from..chars.len().saturating_sub(1)).find(|&i| chars[i] == a && chars[i + 1] == b)
}
fn find3(chars: &[char], from: usize, a: char, b: char, c: char) -> Option<usize> {
    (from..chars.len().saturating_sub(2)).find(|&i| chars[i] == a && chars[i + 1] == b && chars[i + 2] == c)
}
fn next_nonspace(chars: &[char], from: usize) -> Option<char> {
    chars[from..].iter().copied().find(|c| !c.is_whitespace())
}

fn code_lines(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_block = false;
    for raw in text.lines() {
        let (line, still) = code_line(raw, in_block);
        in_block = still;
        out.push(line);
    }
    out
}

fn code_line(raw: &str, mut in_block: bool) -> (Line<'static>, bool) {
    let chars: Vec<char> = raw.chars().collect();
    let n = chars.len();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < n {
        if in_block {
            if let Some(end) = find2(&chars, i, '*', '/') {
                spans.push(Span::styled(chars[i..end + 2].iter().collect::<String>(), comment_style()));
                i = end + 2;
                in_block = false;
            } else {
                spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
                i = n;
            }
            continue;
        }
        let c = chars[i];
        // line comment
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            push_buf(&mut spans, &mut buf, plain());
            spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
            break;
        }
        // block comment
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            push_buf(&mut spans, &mut buf, plain());
            if let Some(end) = find2(&chars, i + 2, '*', '/') {
                spans.push(Span::styled(chars[i..end + 2].iter().collect::<String>(), comment_style()));
                i = end + 2;
            } else {
                spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
                in_block = true;
                i = n;
            }
            continue;
        }
        // string (closed on this line only, so Rust lifetimes don't run away)
        if c == '"' || c == '\'' || c == '`' {
            let mut j = i + 1;
            let mut found = false;
            while j < n {
                if chars[j] == '\\' {
                    j += 2;
                    continue;
                }
                if chars[j] == c {
                    found = true;
                    break;
                }
                j += 1;
            }
            if found {
                push_buf(&mut spans, &mut buf, plain());
                spans.push(Span::styled(chars[i..=j].iter().collect::<String>(), Style::default().fg(STRING)));
                i = j + 1;
                continue;
            }
            buf.push(c);
            i += 1;
            continue;
        }
        // number
        if c.is_ascii_digit() {
            push_buf(&mut spans, &mut buf, plain());
            let mut j = i;
            while j < n && (chars[j].is_ascii_alphanumeric() || chars[j] == '.' || chars[j] == '_') {
                j += 1;
            }
            spans.push(Span::styled(chars[i..j].iter().collect::<String>(), Style::default().fg(NUMBER)));
            i = j;
            continue;
        }
        // identifier / keyword / call
        if c.is_alphabetic() || c == '_' || c == '$' {
            let mut j = i;
            while j < n && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '$') {
                j += 1;
            }
            push_buf(&mut spans, &mut buf, plain());
            let word: String = chars[i..j].iter().collect();
            let style = if KEYWORDS.contains(&word.as_str()) {
                Style::default().fg(KEYWORD)
            } else if next_nonspace(&chars, j) == Some('(') {
                Style::default().fg(FUNC)
            } else {
                plain()
            };
            spans.push(Span::styled(word, style));
            i = j;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    push_buf(&mut spans, &mut buf, plain());
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), plain()));
    }
    (Line::from(spans), in_block)
}

fn markup_lines(text: &str) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut in_comment = false;
    let mut in_tag = false;
    for raw in text.lines() {
        let (line, c, t) = markup_line(raw, in_comment, in_tag);
        in_comment = c;
        in_tag = t;
        out.push(line);
    }
    out
}

fn markup_line(raw: &str, mut in_comment: bool, mut in_tag: bool) -> (Line<'static>, bool, bool) {
    let chars: Vec<char> = raw.chars().collect();
    let n = chars.len();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    let mut name_pending = false;
    while i < n {
        if in_comment {
            if let Some(end) = find3(&chars, i, '-', '-', '>') {
                push_buf(&mut spans, &mut buf, plain());
                spans.push(Span::styled(chars[i..end + 3].iter().collect::<String>(), comment_style()));
                i = end + 3;
                in_comment = false;
            } else {
                spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
                i = n;
            }
            continue;
        }
        let c = chars[i];
        if in_tag {
            if c == '>' {
                push_buf(&mut spans, &mut buf, punct());
                spans.push(Span::styled(">".to_string(), punct()));
                i += 1;
                in_tag = false;
                name_pending = false;
                continue;
            }
            if c == '"' || c == '\'' {
                push_buf(&mut spans, &mut buf, punct());
                let mut j = i + 1;
                let mut found = false;
                while j < n {
                    if chars[j] == c {
                        found = true;
                        break;
                    }
                    j += 1;
                }
                if found {
                    spans.push(Span::styled(chars[i..=j].iter().collect::<String>(), Style::default().fg(STRING)));
                    i = j + 1;
                } else {
                    buf.push(c);
                    i += 1;
                }
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let mut j = i;
                while j < n
                    && (chars[j].is_alphanumeric() || matches!(chars[j], '_' | ':' | '-' | '.'))
                {
                    j += 1;
                }
                push_buf(&mut spans, &mut buf, punct());
                let style = if name_pending {
                    Style::default().fg(TAG).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(ATTR)
                };
                spans.push(Span::styled(chars[i..j].iter().collect::<String>(), style));
                name_pending = false;
                i = j;
                continue;
            }
            buf.push(c);
            i += 1;
            continue;
        }
        if c == '<' {
            if i + 3 < n && chars[i + 1] == '!' && chars[i + 2] == '-' && chars[i + 3] == '-' {
                push_buf(&mut spans, &mut buf, plain());
                if let Some(end) = find3(&chars, i + 4, '-', '-', '>') {
                    spans.push(Span::styled(chars[i..end + 3].iter().collect::<String>(), comment_style()));
                    i = end + 3;
                } else {
                    spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
                    in_comment = true;
                    i = n;
                }
                continue;
            }
            push_buf(&mut spans, &mut buf, plain());
            let mut k = i + 1;
            let bracket = if k < n && chars[k] == '/' {
                k += 1;
                "</".to_string()
            } else {
                "<".to_string()
            };
            spans.push(Span::styled(bracket, punct()));
            i = k;
            in_tag = true;
            name_pending = true;
            continue;
        }
        buf.push(c);
        i += 1;
    }
    push_buf(&mut spans, &mut buf, plain());
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), plain()));
    }
    (Line::from(spans), in_comment, in_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg_of<'a>(line: &'a Line, text: &str) -> Option<Color> {
        line.spans.iter().find(|s| s.content.as_ref() == text).and_then(|s| s.style.fg)
    }

    #[test]
    fn lang_detection() {
        assert_eq!(language_for("a.ts"), Some(Lang::Code));
        assert_eq!(language_for("a.css"), Some(Lang::Code));
        assert_eq!(language_for("a.html"), Some(Lang::Markup));
        assert_eq!(language_for("a.vue"), Some(Lang::Markup));
        assert_eq!(language_for("a.txt"), None);
    }

    #[test]
    fn code_keyword_string_number_comment() {
        let lines = code_lines("const x = \"hi\"; // note");
        let l = &lines[0];
        assert_eq!(fg_of(l, "const"), Some(KEYWORD));
        assert!(l.spans.iter().any(|s| s.content.as_ref() == "\"hi\"" && s.style.fg == Some(STRING)));
        assert!(l.spans.iter().any(|s| s.content.contains("// note")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
        let nums = code_lines("let n = 42;");
        assert!(nums[0].spans.iter().any(|s| s.content.as_ref() == "42" && s.style.fg == Some(NUMBER)));
    }

    #[test]
    fn code_function_call() {
        let lines = code_lines("foo(1)");
        assert_eq!(fg_of(&lines[0], "foo"), Some(FUNC));
    }

    #[test]
    fn code_block_comment_across_lines() {
        let lines = code_lines("a /* c1\nc2 */ b");
        assert!(lines[1].spans.iter().any(|s| s.content.contains("c2")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
        assert!(lines[1].spans.iter().any(|s| s.content.contains('b') && s.style.fg.is_none()));
    }

    #[test]
    fn code_unterminated_quote_is_not_runaway() {
        // a Rust lifetime-ish apostrophe must not color the rest of the line
        let lines = code_lines("let a: &'static str = x;");
        assert!(lines[0].spans.iter().all(|s| s.style.fg != Some(STRING)));
    }

    #[test]
    fn markup_tag_attr_string() {
        let lines = markup_lines("<div class=\"x\">hi</div>");
        let l = &lines[0];
        assert_eq!(fg_of(l, "div"), Some(TAG));
        assert_eq!(fg_of(l, "class"), Some(ATTR));
        assert!(l.spans.iter().any(|s| s.content.as_ref() == "\"x\"" && s.style.fg == Some(STRING)));
    }

    #[test]
    fn markup_comment() {
        let lines = markup_lines("<!-- hi -->");
        assert!(lines[0].spans.iter().any(|s| s.content.contains("hi")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
    }
}
