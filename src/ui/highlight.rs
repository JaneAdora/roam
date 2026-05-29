//! Tiny, dependency-free syntax highlighter for the preview pane and modal.
//! Not a real parser: a lexical pass that colors comments, strings, numbers,
//! keywords and call-sites for C-family/front-end code and Python, and
//! tags/attributes for markup. Good enough to skim code on the go; degrades to
//! readable plain text.
use crate::ui::theme;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Code,
    Python,
    Markup,
}

fn keyword() -> Color {
    theme::magenta()
}
const STRING: Color = Color::Rgb(0x9e, 0xce, 0x6a);
const NUMBER: Color = Color::Rgb(0xe5, 0xc0, 0x7b);
const FUNC: Color = Color::Rgb(0x61, 0xaf, 0xef);
const TAG: Color = Color::Rgb(0xe0, 0x6c, 0x75);
const ATTR: Color = Color::Rgb(0xd1, 0x9a, 0x66);

fn comment_style() -> Style {
    Style::default()
        .fg(theme::lavender())
        .add_modifier(Modifier::DIM | Modifier::ITALIC)
}
fn plain() -> Style {
    Style::default()
}
fn punct() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

const KEYWORDS_C: &[&str] = &[
    "abstract", "as", "async", "await", "break", "case", "catch", "class", "const", "continue",
    "debugger", "declare", "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "from", "function", "get", "if", "implements", "import", "in", "infer",
    "instanceof", "interface", "keyof", "let", "namespace", "new", "null", "of", "private",
    "protected", "public", "readonly", "return", "satisfies", "set", "static", "super", "switch",
    "this", "throw", "true", "try", "type", "typeof", "undefined", "var", "void", "while", "yield",
    "fn", "impl", "mut", "pub", "struct", "trait", "use", "match", "move", "where", "mod", "dyn",
    "crate", "Self",
];

const KEYWORDS_PY: &[&str] = &[
    "and", "as", "assert", "async", "await", "break", "case", "class", "continue", "def", "del",
    "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import", "in",
    "is", "lambda", "match", "None", "nonlocal", "not", "or", "pass", "raise", "return", "True",
    "try", "while", "with", "yield", "self", "cls",
];

/// Per-language lexer config (used by both `Code` and `Python`).
struct Syntax {
    line_comment: &'static str,
    block_comment: bool,
    triple: bool,
    keywords: &'static [&'static str],
}

const SYNTAX_C: Syntax = Syntax {
    line_comment: "//",
    block_comment: true,
    triple: false,
    keywords: KEYWORDS_C,
};
const SYNTAX_PY: Syntax = Syntax {
    line_comment: "#",
    block_comment: false,
    triple: true,
    keywords: KEYWORDS_PY,
};

/// State carried across lines: an open block comment, or an open triple-quoted
/// string (with its quote char).
#[derive(Clone, Copy)]
enum Carry {
    None,
    Block,
    Triple(char),
}

pub fn language_for(name: &str) -> Option<Lang> {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "json" | "json5" | "css" | "scss" | "sass"
        | "less" | "rs" | "go" | "c" | "h" | "cpp" | "hpp" | "java" | "kt" | "swift" | "php" => {
            Some(Lang::Code)
        }
        "py" | "pyi" | "pyw" => Some(Lang::Python),
        "html" | "htm" | "xml" | "svg" | "vue" | "svelte" => Some(Lang::Markup),
        _ => None,
    }
}

pub fn to_lines(text: &str, lang: Lang) -> Vec<Line<'static>> {
    match lang {
        Lang::Code => code_lines(text, &SYNTAX_C),
        Lang::Python => code_lines(text, &SYNTAX_PY),
        Lang::Markup => markup_lines(text),
    }
}

fn push_buf(spans: &mut Vec<Span<'static>>, buf: &mut String, style: Style) {
    if !buf.is_empty() {
        spans.push(Span::styled(std::mem::take(buf), style));
    }
}

fn line_from(mut spans: Vec<Span<'static>>) -> Line<'static> {
    if spans.is_empty() {
        spans.push(Span::styled(String::new(), plain()));
    }
    Line::from(spans)
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
fn starts_with_at(chars: &[char], i: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    i + p.len() <= chars.len() && (0..p.len()).all(|k| chars[i + k] == p[k])
}

fn code_lines(text: &str, sx: &Syntax) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let mut carry = Carry::None;
    for raw in text.lines() {
        let (line, c) = code_line(raw, carry, sx);
        carry = c;
        out.push(line);
    }
    out
}

fn code_line(raw: &str, carry: Carry, sx: &Syntax) -> (Line<'static>, Carry) {
    let chars: Vec<char> = raw.chars().collect();
    let n = chars.len();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    let mut carry = carry;

    // Resume an open block comment or triple-quoted string from a prior line.
    match carry {
        Carry::Block => {
            if let Some(end) = find2(&chars, 0, '*', '/') {
                spans.push(Span::styled(chars[0..end + 2].iter().collect::<String>(), comment_style()));
                i = end + 2;
                carry = Carry::None;
            } else {
                spans.push(Span::styled(chars[..].iter().collect::<String>(), comment_style()));
                return (line_from(spans), Carry::Block);
            }
        }
        Carry::Triple(q) => {
            if let Some(end) = find3(&chars, 0, q, q, q) {
                spans.push(Span::styled(chars[0..end + 3].iter().collect::<String>(), Style::default().fg(STRING)));
                i = end + 3;
                carry = Carry::None;
            } else {
                spans.push(Span::styled(chars[..].iter().collect::<String>(), Style::default().fg(STRING)));
                return (line_from(spans), Carry::Triple(q));
            }
        }
        Carry::None => {}
    }

    while i < n {
        let c = chars[i];
        // line comment
        if starts_with_at(&chars, i, sx.line_comment) {
            push_buf(&mut spans, &mut buf, plain());
            spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
            break;
        }
        // block comment
        if sx.block_comment && c == '/' && i + 1 < n && chars[i + 1] == '*' {
            push_buf(&mut spans, &mut buf, plain());
            if let Some(end) = find2(&chars, i + 2, '*', '/') {
                spans.push(Span::styled(chars[i..end + 2].iter().collect::<String>(), comment_style()));
                i = end + 2;
            } else {
                spans.push(Span::styled(chars[i..].iter().collect::<String>(), comment_style()));
                return (line_from(spans), Carry::Block);
            }
            continue;
        }
        // triple-quoted string (Python)
        if sx.triple
            && (c == '"' || c == '\'')
            && chars.get(i + 1) == Some(&c)
            && chars.get(i + 2) == Some(&c)
        {
            push_buf(&mut spans, &mut buf, plain());
            if let Some(end) = find3(&chars, i + 3, c, c, c) {
                spans.push(Span::styled(chars[i..end + 3].iter().collect::<String>(), Style::default().fg(STRING)));
                i = end + 3;
            } else {
                spans.push(Span::styled(chars[i..].iter().collect::<String>(), Style::default().fg(STRING)));
                return (line_from(spans), Carry::Triple(c));
            }
            continue;
        }
        // single-line string (closed on this line only)
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
            let style = if sx.keywords.contains(&word.as_str()) {
                Style::default().fg(keyword())
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
    (line_from(spans), carry)
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
                while j < n && (chars[j].is_alphanumeric() || matches!(chars[j], '_' | ':' | '-' | '.')) {
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
    (line_from(spans), in_comment, in_tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fg_of(line: &Line, text: &str) -> Option<Color> {
        line.spans.iter().find(|s| s.content.as_ref() == text).and_then(|s| s.style.fg)
    }

    #[test]
    fn lang_detection() {
        assert_eq!(language_for("a.ts"), Some(Lang::Code));
        assert_eq!(language_for("a.css"), Some(Lang::Code));
        assert_eq!(language_for("a.py"), Some(Lang::Python));
        assert_eq!(language_for("a.html"), Some(Lang::Markup));
        assert_eq!(language_for("a.txt"), None);
    }

    #[test]
    fn code_keyword_string_number_comment() {
        let l = &to_lines("const x = \"hi\"; // note", Lang::Code)[0];
        assert_eq!(fg_of(l, "const"), Some(keyword()));
        assert!(l.spans.iter().any(|s| s.content.as_ref() == "\"hi\"" && s.style.fg == Some(STRING)));
        assert!(l.spans.iter().any(|s| s.content.contains("// note")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
        let nums = to_lines("let n = 42;", Lang::Code);
        assert!(nums[0].spans.iter().any(|s| s.content.as_ref() == "42" && s.style.fg == Some(NUMBER)));
    }

    #[test]
    fn code_function_call() {
        assert_eq!(fg_of(&to_lines("foo(1)", Lang::Code)[0], "foo"), Some(FUNC));
    }

    #[test]
    fn code_block_comment_across_lines() {
        let lines = to_lines("a /* c1\nc2 */ b", Lang::Code);
        assert!(lines[1].spans.iter().any(|s| s.content.contains("c2")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
        assert!(lines[1].spans.iter().any(|s| s.content.contains('b') && s.style.fg.is_none()));
    }

    #[test]
    fn code_unterminated_quote_is_not_runaway() {
        let lines = to_lines("let a: &'static str = x;", Lang::Code);
        assert!(lines[0].spans.iter().all(|s| s.style.fg != Some(STRING)));
    }

    #[test]
    fn python_hash_comment_keyword_call() {
        let lines = to_lines("def f(x):  # doc\n    return x", Lang::Python);
        assert_eq!(fg_of(&lines[0], "def"), Some(keyword()));
        assert_eq!(fg_of(&lines[0], "f"), Some(FUNC));
        assert!(lines[0].spans.iter().any(|s| s.content.contains("# doc")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
        assert_eq!(fg_of(&lines[1], "return"), Some(keyword()));
    }

    #[test]
    fn python_triple_string_spans_lines() {
        let lines = to_lines("x = \"\"\"hi\nthere\"\"\"\ny = 1", Lang::Python);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("\"\"\"hi") && s.style.fg == Some(STRING)));
        assert!(lines[1].spans.iter().any(|s| s.content.contains("there") && s.style.fg == Some(STRING)));
        assert!(lines[2].spans.iter().any(|s| s.content.as_ref() == "1" && s.style.fg == Some(NUMBER)));
    }

    #[test]
    fn css_hash_is_not_a_comment() {
        let lines = to_lines(".a { color: #fff; }", Lang::Code);
        assert!(lines[0].spans.iter().all(|s| !s.style.add_modifier.contains(Modifier::ITALIC)));
    }

    #[test]
    fn markup_tag_attr_string() {
        let l = &to_lines("<div class=\"x\">hi</div>", Lang::Markup)[0];
        assert_eq!(fg_of(l, "div"), Some(TAG));
        assert_eq!(fg_of(l, "class"), Some(ATTR));
        assert!(l.spans.iter().any(|s| s.content.as_ref() == "\"x\"" && s.style.fg == Some(STRING)));
    }

    #[test]
    fn markup_comment() {
        let lines = to_lines("<!-- hi -->", Lang::Markup);
        assert!(lines[0].spans.iter().any(|s| s.content.contains("hi")
            && s.style.add_modifier.contains(Modifier::ITALIC)));
    }
}
