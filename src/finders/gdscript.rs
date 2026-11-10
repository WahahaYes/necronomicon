//! The GDScript line scanner (spec: Finders): one pass over a file labels each
//! line's code, string and comment spans, the bracket depth at its start, and
//! whether it starts inside a `"""` string. The finders read those labels
//! instead of tracking quotes and brackets themselves.

/// What a span of a line holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Code,
    String,
    Comment,
}

/// A byte range on a line and what it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Span {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) kind: Kind,
}

/// One comment: its line and column (0-based, in chars), and whether it's a
/// doc comment and the first thing on its line.
pub(super) struct Comment {
    pub(super) line: usize,
    pub(super) col: usize,
    pub(super) doc: bool,
    pub(super) own_line: bool,
}

/// A file's lines, the same lines with comments blanked, their spans and
/// comments, and where each line starts.
pub(super) struct Source {
    pub(super) lines: Vec<String>,
    /// Comments blanked, strings kept.
    pub(super) code: Vec<String>,
    pub(super) spans: Vec<Vec<Span>>,
    pub(super) comments: Vec<Comment>,
    /// The bracket depth at the start of each line, outside strings and comments.
    pub(super) depth: Vec<usize>,
    /// Lines that start inside a triple-quoted string: text, not code.
    pub(super) in_string: Vec<bool>,
}

impl Source {
    /// The byte offset where a line's code ends: its first comment, or its end.
    pub(super) fn code_end(&self, n: usize) -> usize {
        self.spans[n]
            .iter()
            .find(|span| span.kind == Kind::Comment)
            .map_or(self.lines[n].len(), |span| span.start)
    }
}

/// Read a file into its labeled lines; `None` if it can't be read.
pub(super) fn scan_file(path: &str, markers: &[String]) -> Option<Source> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    Some(scan(&text, markers))
}

/// Scan text, treating each marker as the start of a comment outside a string.
pub(super) fn scan(text: &str, markers: &[String]) -> Source {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let mut code = Vec::with_capacity(lines.len());
    let mut spans = Vec::with_capacity(lines.len());
    let mut comments = Vec::new();
    let mut depth_at = Vec::with_capacity(lines.len());
    let mut in_string = Vec::with_capacity(lines.len());
    let mut depth = 0usize;
    let mut quote: Option<&str> = None;
    for (n, line) in lines.iter().enumerate() {
        in_string.push(quote.is_some());
        depth_at.push(depth);
        let (line_spans, comment_at) =
            scan_line(line, markers, &mut quote, &mut depth, n, &mut comments);
        code.push(blank_comment(line, comment_at));
        spans.push(line_spans);
        // Only a triple-quoted string runs on past its line.
        if quote != Some("\"\"\"") {
            quote = None;
        }
    }
    Source {
        lines,
        code,
        spans,
        comments,
        depth: depth_at,
        in_string,
    }
}

/// Label one line's spans, carrying a string and the bracket depth across
/// lines, and collect the comment that starts on it.
fn scan_line(
    line: &str,
    markers: &[String],
    quote: &mut Option<&'static str>,
    depth: &mut usize,
    n: usize,
    comments: &mut Vec<Comment>,
) -> (Vec<Span>, Option<usize>) {
    let mut spans = Vec::new();
    let mut code_start = 0usize;
    let mut string_start = 0usize;
    let mut comment_at = None;
    let mut i = 0usize;
    while i < line.len() {
        if let Some(q) = *quote {
            if line[i..].starts_with('\\') {
                i += 1;
                if i < line.len() {
                    i += next_char_len(line, i);
                }
            } else if line[i..].starts_with(q) {
                i += q.len();
                spans.push(Span {
                    start: string_start,
                    end: i,
                    kind: Kind::String,
                });
                *quote = None;
                code_start = i;
            } else {
                i += next_char_len(line, i);
            }
            continue;
        }
        if let Some(marker) = markers
            .iter()
            .find(|marker| line[i..].starts_with(marker.as_str()))
        {
            if code_start < i {
                spans.push(Span {
                    start: code_start,
                    end: i,
                    kind: Kind::Code,
                });
            }
            spans.push(Span {
                start: i,
                end: line.len(),
                kind: Kind::Comment,
            });
            comments.push(Comment {
                line: n,
                col: line[..i].chars().count(),
                doc: marker == "#" && line[i..].starts_with("##"),
                own_line: line[..i].chars().all(char::is_whitespace),
            });
            comment_at = Some(i);
            code_start = line.len();
            break;
        }
        if let Some(q) = ["\"\"\"", "\"", "'"]
            .into_iter()
            .find(|q| line[i..].starts_with(q))
        {
            if code_start < i {
                spans.push(Span {
                    start: code_start,
                    end: i,
                    kind: Kind::Code,
                });
            }
            string_start = i;
            *quote = Some(q);
            i += q.len();
            continue;
        }
        let c = line[i..].chars().next().unwrap_or_default();
        if "([{".contains(c) {
            *depth += 1;
        } else if ")]}".contains(c) {
            *depth = depth.saturating_sub(1);
        }
        i += c.len_utf8();
    }
    if let Some(q) = *quote {
        spans.push(Span {
            start: string_start,
            end: line.len(),
            kind: Kind::String,
        });
        if q != "\"\"\"" {
            *quote = None;
        }
        code_start = line.len();
    }
    if code_start < line.len() {
        spans.push(Span {
            start: code_start,
            end: line.len(),
            kind: Kind::Code,
        });
    }
    (spans, comment_at)
}

/// The byte length of the character at `i`.
fn next_char_len(line: &str, i: usize) -> usize {
    line[i..].chars().next().map_or(1, char::len_utf8)
}

/// A line with the comment from `comment_at` on blanked, as `gdscan` reads code.
fn blank_comment(line: &str, comment_at: Option<usize>) -> String {
    match comment_at {
        None => line.to_string(),
        Some(at) => {
            let mut code = String::with_capacity(line.len());
            code.push_str(&line[..at]);
            code.push_str(&" ".repeat(line[at..].chars().count()));
            code
        }
    }
}
