//! The language table (spec: Languages) and the one pass over a file that
//! gives the code view (comments blanked, strings kept) and its comments.

use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Quote {
    pub open: String,
    pub close: String,
    pub multiline: bool,
}

/// What may come before a file's header comment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Preamble {
    None,
    Shebang,
    Python,
    Gd,
    Fence,
    FrontMatter,
    YamlStart,
}

#[derive(Clone, Debug)]
pub struct Lang {
    pub line: Vec<String>,
    pub block: Vec<(String, String)>,
    pub quotes: Vec<Quote>,
    /// Python: a triple-quoted string that opens a line is a comment block.
    pub docstrings: bool,
    /// Rust: raw strings and char literals.
    pub rust: bool,
    pub preamble: Preamble,
}

fn q(open: &str, multiline: bool) -> Quote {
    Quote {
        open: open.into(),
        close: open.into(),
        multiline,
    }
}

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

fn lang(line: &[&str], block: &[(&str, &str)], quotes: Vec<Quote>, preamble: Preamble) -> Lang {
    Lang {
        line: strs(line),
        block: block
            .iter()
            .map(|(o, c)| (o.to_string(), c.to_string()))
            .collect(),
        quotes,
        docstrings: false,
        rust: false,
        preamble,
    }
}

/// The built-in table, keyed by extension with its dot.
pub fn builtin() -> BTreeMap<String, Lang> {
    let mut table = BTreeMap::new();
    let ts = lang(
        &["//"],
        &[("/*", "*/")],
        vec![q("\"", false), q("'", false), q("`", true)],
        Preamble::Shebang,
    );
    for ext in [".ts", ".mts", ".cts", ".tsx", ".js", ".mjs", ".cjs", ".jsx"] {
        table.insert(ext.to_string(), ts.clone());
    }
    let astro = lang(
        &["//"],
        &[("/*", "*/"), ("<!--", "-->")],
        vec![q("\"", false), q("'", false), q("`", true)],
        Preamble::Fence,
    );
    table.insert(".astro".into(), astro);
    let css = lang(
        &[],
        &[("/*", "*/")],
        vec![q("\"", false), q("'", false)],
        Preamble::None,
    );
    table.insert(".css".into(), css);
    let mut py = lang(
        &["#"],
        &[],
        vec![
            q("\"\"\"", true),
            q("'''", true),
            q("\"", false),
            q("'", false),
        ],
        Preamble::Python,
    );
    py.docstrings = true;
    table.insert(".py".into(), py);
    let mut rs = lang(
        &["///", "//!", "//"],
        &[("/*", "*/")],
        vec![q("\"", true)],
        Preamble::None,
    );
    rs.rust = true;
    table.insert(".rs".into(), rs);
    let gd = lang(
        &["##", "#"],
        &[],
        vec![q("\"\"\"", true), q("\"", false), q("'", false)],
        Preamble::Gd,
    );
    table.insert(".gd".into(), gd);
    let md = lang(&[], &[("<!--", "-->")], vec![], Preamble::FrontMatter);
    table.insert(".md".into(), md);
    let yaml = lang(
        &["#"],
        &[],
        vec![q("\"", false), q("'", false)],
        Preamble::YamlStart,
    );
    table.insert(".yaml".into(), yaml.clone());
    table.insert(".yml".into(), yaml);
    table
}

/// A repo's own entry: `{ line, block, strings }`, preamble-free.
pub fn custom(line: Vec<String>, block: Vec<(String, String)>, strings: Vec<String>) -> Lang {
    let quotes = strings
        .into_iter()
        .map(|s| {
            let multiline = s.chars().count() == 3 || s == "`";
            Quote {
                open: s.clone(),
                close: s,
                multiline,
            }
        })
        .collect();
    let mut line = line;
    // Longest marker first, so `##` wins over `#`.
    line.sort_by_key(|m| std::cmp::Reverse(m.len()));
    Lang {
        line,
        block,
        quotes,
        docstrings: false,
        rust: false,
        preamble: Preamble::None,
    }
}

/// The extension a path is looked up by: `.gd`, or "" for none.
pub fn extension(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[i..].to_string(),
        _ => String::new(),
    }
}

/// One comment, or one line of a multi-line block comment or docstring.
#[derive(Clone, Debug)]
pub struct Comment {
    /// 0-based line and code-point column of the marker (or the line start
    /// for a block's later lines).
    pub line: usize,
    pub col: usize,
    pub marker: String,
    /// The text after the marker, up to the closer or the line end.
    pub text: String,
    /// Code-point column where `text` starts.
    pub text_col: usize,
    /// Set for block comments and docstrings: lines of one block share it.
    pub block: Option<usize>,
    pub docstring: bool,
    /// Nothing but whitespace before it on its line.
    pub own_line: bool,
}

pub struct Source {
    pub lines: Vec<String>,
    /// `lines` with comments blanked to spaces, columns unchanged.
    pub code: Vec<String>,
    pub comments: Vec<Comment>,
}

enum State {
    Normal,
    Block {
        close: String,
    },
    Str {
        close: String,
        multiline: bool,
        raw: bool,
        doc: bool,
    },
}

/// The block comment or docstring being read: where its text starts on this line.
struct Open {
    col: usize,
    text_col: usize,
    id: usize,
    marker: String,
    docstring: bool,
}

fn at(chars: &[char], i: usize, s: &str) -> bool {
    s.chars()
        .enumerate()
        .all(|(k, c)| chars.get(i + k) == Some(&c))
}

/// Split into lines (dropping `\r`) and scan with `lang`, or raw when `None`.
pub fn scan(text: &str, lang: Option<&Lang>) -> Source {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect();
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }
    let Some(lang) = lang else {
        return Source {
            code: lines.clone(),
            lines,
            comments: Vec::new(),
        };
    };
    let mut code = Vec::with_capacity(lines.len());
    let mut comments: Vec<Comment> = Vec::new();
    let mut state = State::Normal;
    let mut open: Option<Open> = None;
    let mut blocks = 0;
    for (n, line) in lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let mut out = chars.clone();
        let first = chars
            .iter()
            .position(|c| !c.is_whitespace())
            .unwrap_or(chars.len());
        if let Some(o) = open.as_mut() {
            o.col = first;
            o.text_col = first;
        }
        let mut i = 0;
        while i < chars.len() {
            match &state {
                State::Normal => {
                    if let Some(m) = lang.line.iter().find(|m| at(&chars, i, m)) {
                        let len = m.chars().count();
                        comments.push(Comment {
                            line: n,
                            col: i,
                            marker: m.clone(),
                            text: chars[i + len..].iter().collect(),
                            text_col: i + len,
                            block: None,
                            docstring: false,
                            own_line: i == first,
                        });
                        out[i..].fill(' ');
                        i = chars.len();
                    } else if let Some((o, c)) = lang.block.iter().find(|(o, _)| at(&chars, i, o)) {
                        let len = o.chars().count();
                        out[i..i + len].fill(' ');
                        blocks += 1;
                        open = Some(Open {
                            col: i,
                            text_col: i + len,
                            id: blocks,
                            marker: o.clone(),
                            docstring: false,
                        });
                        state = State::Block { close: c.clone() };
                        i += len;
                    } else if let Some(quote) = lang.quotes.iter().find(|q| at(&chars, i, &q.open))
                    {
                        let len = quote.open.chars().count();
                        let doc = lang.docstrings && len == 3 && i == first;
                        if doc {
                            blocks += 1;
                            open = Some(Open {
                                col: i,
                                text_col: i + len,
                                id: blocks,
                                marker: quote.open.clone(),
                                docstring: true,
                            });
                        }
                        state = State::Str {
                            close: quote.close.clone(),
                            multiline: quote.multiline,
                            raw: false,
                            doc,
                        };
                        i += len;
                    } else if let (true, 'r', Some(hashes)) =
                        (lang.rust, chars[i], rust_raw(&chars, i))
                    {
                        state = State::Str {
                            close: format!("\"{}", "#".repeat(hashes)),
                            multiline: true,
                            raw: true,
                            doc: false,
                        };
                        i += hashes + 2;
                    } else if lang.rust && chars[i] == '\'' {
                        i += rust_char_len(&chars, i);
                    } else {
                        i += 1;
                    }
                }
                State::Block { close } => {
                    if at(&chars, i, close) {
                        let len = close.chars().count();
                        out[i..i + len].fill(' ');
                        push_open(&mut comments, open.take(), &chars, n, first, i);
                        state = State::Normal;
                        i += len;
                    } else {
                        out[i] = ' ';
                        i += 1;
                    }
                }
                State::Str {
                    close, raw, doc, ..
                } => {
                    if !*raw && chars[i] == '\\' {
                        i += 2;
                    } else if at(&chars, i, close) {
                        let len = close.chars().count();
                        if *doc {
                            push_open(&mut comments, open.take(), &chars, n, first, i);
                        }
                        state = State::Normal;
                        i += len;
                    } else {
                        i += 1;
                    }
                }
            }
        }
        // Whatever is still open runs to the line end.
        match &state {
            State::Block { .. } | State::Str { doc: true, .. } => {
                let o = open.as_ref().map(|o| Open {
                    marker: o.marker.clone(),
                    ..*o
                });
                push_open(&mut comments, o, &chars, n, first, chars.len());
            }
            State::Str {
                multiline: false, ..
            } => state = State::Normal,
            _ => {}
        }
        code.push(out.into_iter().collect());
    }
    Source {
        lines,
        code,
        comments,
    }
}

fn push_open(
    comments: &mut Vec<Comment>,
    open: Option<Open>,
    chars: &[char],
    n: usize,
    first: usize,
    end: usize,
) {
    let Some(o) = open else { return };
    let from = o.text_col.min(end);
    comments.push(Comment {
        line: n,
        col: o.col,
        marker: o.marker,
        text: chars[from..end].iter().collect(),
        text_col: from,
        block: Some(o.id),
        docstring: o.docstring,
        own_line: o.col == first,
    });
}

/// `r"`, `r#"`, … : the number of hashes.
fn rust_raw(chars: &[char], i: usize) -> Option<usize> {
    if i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_') {
        return None;
    }
    let mut k = i + 1;
    while chars.get(k) == Some(&'#') {
        k += 1;
    }
    (chars.get(k) == Some(&'"')).then_some(k - i - 1)
}

/// A char literal (`'x'`, `'\n'`) is skipped whole; a lifetime is one char.
fn rust_char_len(chars: &[char], i: usize) -> usize {
    if chars.get(i + 1) == Some(&'\\') {
        let close = chars[i + 2..].iter().position(|&c| c == '\'');
        return close.map_or(1, |p| p + 3);
    }
    if chars.get(i + 2) == Some(&'\'') {
        return 3;
    }
    1
}

/// How many leading lines are blank or what the language allows before the header.
pub fn preamble_len(lang: &Lang, lines: &[String]) -> usize {
    let mut n = 0;
    let mut fence_open = false;
    while n < lines.len() {
        let t = lines[n].trim();
        let fence = matches!(lang.preamble, Preamble::Fence | Preamble::FrontMatter);
        if fence && (fence_open || (n == 0 && t == "---")) {
            fence_open = !(n > 0 && t == "---");
            n += 1;
            continue;
        }
        let allowed = t.is_empty()
            || (n == 0 && t.starts_with("#!") && lang.preamble != Preamble::None)
            || (lang.preamble == Preamble::Python && t.starts_with('#') && t.contains("coding"))
            || (lang.preamble == Preamble::Gd
                && ["@tool", "@icon", "class_name ", "extends "]
                    .iter()
                    .any(|p| t.starts_with(p)))
            || (lang.preamble == Preamble::YamlStart && t == "---");
        if !allowed {
            break;
        }
        n += 1;
    }
    n
}
