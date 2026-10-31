//! A snapshot of known hits (the baseline ratchet): flat YAML with a
//! top-level version plus an `entries:` list of rule/path/fingerprint/line.
//! Matching is a multiset per (rule, path, fingerprint) bucket; the stored
//! line is only a hint for humans and never part of the key.

use std::collections::BTreeMap;
use std::path::Path;

use crate::run::Diagnostic;
use crate::yaml::{self, Value};

/// The snapshot layout this necro reads and writes.
pub const VERSION: i64 = 1;

const ALLOW: &str = "necro-allow";

/// One known hit. `line` is a hint for humans; matching ignores it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub rule: String,
    pub path: String,
    pub fingerprint: String,
    pub line: usize,
}

/// A snapshot with entries sorted by (rule, path, fingerprint).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Baseline {
    entries: Vec<Entry>,
}

/// Why a snapshot file couldn't be read. The caller maps every case to
/// exit 2; a missing file is `Ok(empty)`, never this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    Read(String),
    Parse(String),
    Shape(String),
    Version(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Read(message)
            | LoadError::Parse(message)
            | LoadError::Shape(message)
            | LoadError::Version(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for LoadError {}

impl Baseline {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Read a snapshot. A missing file is an empty baseline; anything
    /// unreadable, unparseable, misshapen or versioned otherwise errors.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(e) => {
                return Err(LoadError::Read(format!(
                    "can't read {}: {e}",
                    path.display()
                )));
            }
        };
        let node = yaml::parse(&text).map_err(|(pos, message)| {
            LoadError::Parse(format!(
                "{}:{}:{}: {message}",
                path.display(),
                pos.line,
                pos.col
            ))
        })?;
        Self::from_node(&path.display().to_string(), &node)
    }

    fn from_node(file: &str, node: &yaml::Node) -> Result<Self, LoadError> {
        let Value::Map(items) = &node.value else {
            return Err(LoadError::Shape(format!(
                "{file}: expected a mapping, got {}",
                node.kind()
            )));
        };
        let mut version: Option<&yaml::Node> = None;
        let mut entries_node: Option<&yaml::Node> = None;
        for (key, value) in items {
            match key.name.as_str() {
                "version" => version = Some(value),
                "entries" => entries_node = Some(value),
                other => {
                    return Err(LoadError::Shape(format!(
                        "{file}: unknown key `{other}`; expected version and entries"
                    )));
                }
            }
        }
        let version = version
            .ok_or_else(|| LoadError::Shape(format!("{file}: missing required key `version`")))?;
        match version.as_number() {
            Some(n) if n == VERSION as f64 => {}
            Some(n) => {
                return Err(LoadError::Version(format!(
                    "{file}: baseline version {n} isn't {VERSION}; delete the snapshot and re-run to rebuild it"
                )));
            }
            None => {
                return Err(LoadError::Shape(format!(
                    "{file}: version is a whole number; this necro reads version {VERSION}"
                )));
            }
        }
        let entries_node = entries_node
            .ok_or_else(|| LoadError::Shape(format!("{file}: missing required key `entries`")))?;
        let items = match &entries_node.value {
            Value::Seq(items) => items.as_slice(),
            Value::Null => &[],
            _ => {
                return Err(LoadError::Shape(format!(
                    "{file}: expected a list of entries, got {}",
                    entries_node.kind()
                )));
            }
        };
        let mut entries = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            entries.push(read_entry(file, i, item)?);
        }
        sort_entries(&mut entries);
        Ok(Self { entries })
    }

    /// Snapshot the given post-allow diagnostics.
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut entries: Vec<Entry> = diagnostics
            .iter()
            .map(|d| Entry {
                rule: d.rule.clone(),
                path: d.file.clone(),
                fingerprint: fingerprint_for(d),
                line: d.line,
            })
            .collect();
        sort_entries(&mut entries);
        Self { entries }
    }

    /// The file text: `version:` plus an `entries:` list sorted by
    /// (rule, path, fingerprint). Every run over the same hits writes
    /// byte-identical text.
    pub fn render(&self) -> String {
        let mut sorted: Vec<&Entry> = self.entries.iter().collect();
        sorted.sort_by(|a, b| {
            (&a.rule, &a.path, &a.fingerprint, a.line).cmp(&(
                &b.rule,
                &b.path,
                &b.fingerprint,
                b.line,
            ))
        });
        let mut out = format!("version: {VERSION}\n");
        if sorted.is_empty() {
            out.push_str("entries: []\n");
            return out;
        }
        out.push_str("entries:\n");
        for e in sorted {
            out.push_str(&format!(
                "  - rule: {}\n    path: {}\n    fingerprint: {}\n    line: {}\n",
                yaml_string(&e.rule),
                yaml_string(&e.path),
                yaml_string(&e.fingerprint),
                e.line
            ));
        }
        out
    }

    /// Write the snapshot to `path`.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        std::fs::write(path, self.render())
            .map_err(|e| format!("can't write {}: {e}", path.display()))
    }

    /// Split post-allow diagnostics into the hits the snapshot doesn't
    /// cover (kept, in input order) plus how many snapshot entries no
    /// observed hit consumed (stale). Each observed hit consumes one
    /// entry from its (rule, path, fingerprint) bucket.
    pub fn match_hits(&self, diagnostics: Vec<Diagnostic>) -> (Vec<Diagnostic>, usize) {
        let mut counts: BTreeMap<(String, String, String), usize> = BTreeMap::new();
        for e in &self.entries {
            *counts
                .entry((e.rule.clone(), e.path.clone(), e.fingerprint.clone()))
                .or_insert(0) += 1;
        }
        let mut kept = Vec::new();
        for d in diagnostics {
            let key = (d.rule.clone(), d.file.clone(), fingerprint_for(&d));
            match counts.get_mut(&key) {
                Some(n) if *n > 0 => *n -= 1,
                _ => kept.push(d),
            }
        }
        (kept, counts.values().sum())
    }
}

fn sort_entries(entries: &mut [Entry]) {
    entries.sort_by(|a, b| {
        (&a.rule, &a.path, &a.fingerprint, a.line).cmp(&(&b.rule, &b.path, &b.fingerprint, b.line))
    });
}

fn read_entry(file: &str, i: usize, node: &yaml::Node) -> Result<Entry, LoadError> {
    let at = format!("{file}: entries[{i}]");
    let Value::Map(items) = &node.value else {
        return Err(LoadError::Shape(format!(
            "{at}: expected a mapping, got {}",
            node.kind()
        )));
    };
    let mut rule: Option<&yaml::Node> = None;
    let mut path: Option<&yaml::Node> = None;
    let mut fingerprint: Option<&yaml::Node> = None;
    let mut line: Option<&yaml::Node> = None;
    for (key, value) in items {
        match key.name.as_str() {
            "rule" => rule = Some(value),
            "path" => path = Some(value),
            "fingerprint" => fingerprint = Some(value),
            "line" => line = Some(value),
            other => {
                return Err(LoadError::Shape(format!("{at}: unknown key `{other}`")));
            }
        }
    }
    let field = |name: &str, node: Option<&yaml::Node>| -> Result<String, LoadError> {
        let node = node.ok_or_else(|| LoadError::Shape(format!("{at}: missing key `{name}`")))?;
        node.as_str().map(String::from).ok_or_else(|| {
            LoadError::Shape(format!(
                "{at}.{name}: expected a string, got {}",
                node.kind()
            ))
        })
    };
    let rule = field("rule", rule)?;
    let path = field("path", path)?;
    let fingerprint = field("fingerprint", fingerprint)?;
    let line = line.ok_or_else(|| LoadError::Shape(format!("{at}: missing key `line`")))?;
    let line = match line.as_number() {
        Some(n) if n.fract() == 0.0 && n >= 0.0 => n as usize,
        _ => {
            return Err(LoadError::Shape(format!(
                "{at}.line: expected a whole line number, got {}",
                line.kind()
            )));
        }
    };
    Ok(Entry {
        rule,
        path,
        fingerprint,
        line,
    })
}

/// The fingerprint of a source line: the text before the first valid
/// allow, trimmed. Validity reuses the allows rule (whitespace after
/// `necro-allow`, then a rule in `[a-z0-9-/]`), so multi-allow lines cut
/// at the first valid one. No tab expansion, no closer handling.
pub fn fingerprint(source: Option<&str>) -> String {
    match source {
        None => String::new(),
        Some(line) => match first_valid_allow_start(line) {
            Some(i) => line[..i].trim().to_string(),
            None => line.trim().to_string(),
        },
    }
}

/// The fingerprint of a diagnostic: whole-file hits (line 0) are always
/// the empty string.
pub fn fingerprint_for(d: &Diagnostic) -> String {
    if d.line == 0 {
        String::new()
    } else {
        fingerprint(d.source.as_deref())
    }
}

fn first_valid_allow_start(line: &str) -> Option<usize> {
    let mut from = 0;
    while let Some(rel) = line[from..].find(ALLOW) {
        let start = from + rel;
        let body = &line[start + ALLOW.len()..];
        if body.starts_with(char::is_whitespace) {
            let rule: String = body
                .trim_start()
                .chars()
                .take_while(|c| {
                    c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '/'
                })
                .collect();
            if !rule.is_empty() {
                return Some(start);
            }
        }
        from = start + ALLOW.len();
    }
    None
}

/// A double-quoted YAML string with the standard escapes.
fn yaml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // src/ is necro's own root, so literal fake allows here would trip the
    // self-hosted unused-allow rule. Build the marker without spelling it.
    const MARK: &str = concat!("necro-", "allow");

    fn hit(rule: &str, file: &str, line: usize, source: Option<&str>) -> Diagnostic {
        Diagnostic {
            rule: rule.into(),
            file: file.into(),
            line,
            col: 1,
            length: 1,
            label: String::new(),
            message: String::new(),
            why: String::new(),
            fix: String::new(),
            source: source.map(String::from),
        }
    }

    #[test]
    fn normalize_strips_allow_and_trims() {
        assert_eq!(
            fingerprint(Some(&format!("let x = 1; // {MARK} foo: some reason"))),
            "let x = 1; //"
        );
        assert_eq!(fingerprint(Some("  let x = 1;  ")), "let x = 1;");
        assert_eq!(fingerprint(None), "");
        // No whitespace after the marker: not a valid allow, no cut.
        assert_eq!(
            fingerprint(Some("x necro-allowfoo: nope")),
            "x necro-allowfoo: nope"
        );
        // Marker with no rule: not valid, no cut.
        assert_eq!(
            fingerprint(Some("x // necro-allow: nope")),
            "x // necro-allow: nope"
        );
    }

    #[test]
    fn multi_allow_cuts_at_first_valid() {
        assert_eq!(
            fingerprint(Some(&format!("code // {MARK} foo: a {MARK} bar: b"))),
            "code //"
        );
        // First marker is invalid (no whitespace), so the cut is at the second.
        assert_eq!(
            fingerprint(Some(&format!("x necro-allowfoo: nope // {MARK} bar: yes"))),
            "x necro-allowfoo: nope //"
        );
    }

    #[test]
    fn whole_file_hits_fingerprint_empty() {
        assert_eq!(
            fingerprint_for(&hit("foo", "a.rs", 0, None)),
            "".to_string()
        );
        assert_eq!(
            fingerprint_for(&hit("foo", "a.rs", 0, Some("whatever is here"))),
            "".to_string()
        );
    }

    #[test]
    fn multiset_consumes_one_entry_each() {
        let baseline = Baseline::from_diagnostics(&[
            hit("foo", "a.rs", 3, Some("let x = 1;")),
            hit("foo", "a.rs", 9, Some("let x = 1;")),
            hit("bar", "b.rs", 1, Some("y")),
        ]);
        // Two hits in the (foo, a.rs, "let x = 1;") bucket, one in bar's:
        // the third foo hit is excess and stays.
        let (kept, stale) = baseline.match_hits(vec![
            hit("foo", "a.rs", 4, Some("let x = 1;")),
            hit("foo", "a.rs", 5, Some("let x = 1;")),
            hit("foo", "a.rs", 6, Some("let x = 1;")),
            hit("bar", "b.rs", 2, Some("y")),
        ]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].rule, "foo");
        assert_eq!(stale, 0);

        // One hit consumes one of the two foo entries; the rest go stale.
        let (kept, stale) = baseline.match_hits(vec![hit("foo", "a.rs", 4, Some("let x = 1;"))]);
        assert!(kept.is_empty());
        assert_eq!(stale, 2);
    }

    #[test]
    fn line_hint_excluded_from_key() {
        let baseline = Baseline::from_diagnostics(&[hit("foo", "a.rs", 10, Some("let x = 1;"))]);
        let (kept, stale) =
            baseline.match_hits(vec![hit("foo", "a.rs", 99, Some("  let x = 1;  "))]);
        assert!(kept.is_empty());
        assert_eq!(stale, 0);
    }

    #[test]
    fn write_is_sorted_and_round_trips() {
        let diagnostics = vec![
            hit("b-rule", "z.rs", 2, Some("second")),
            hit("a-rule", "z.rs", 1, Some("first")),
            hit("a-rule", "a.rs", 7, Some("quoted \" \\ end")),
            hit("a-rule", "a.rs", 7, Some("quoted \" \\ end")),
        ];
        let baseline = Baseline::from_diagnostics(&diagnostics);
        let first = baseline.render();
        let second = Baseline::from_diagnostics(&diagnostics).render();
        assert_eq!(first, second, "render must be deterministic");

        let a_a = first.find("a-rule").expect("a-rule sorts first");
        let b = first.find("b-rule").expect("b-rule sorts after");
        assert!(a_a < b);

        let path = std::env::temp_dir().join(format!(
            "necro-baseline-roundtrip-{}.yaml",
            std::process::id()
        ));
        baseline.save(&path).expect("save writes the snapshot");
        let back = Baseline::load(&path).expect("load reads it back");
        let _ = std::fs::remove_file(&path);
        assert_eq!(back, baseline);
        assert_eq!(back.entries().len(), 4);
        assert_eq!(back.len(), 4);
        assert!(!back.is_empty());
        assert!(Baseline::empty().is_empty());
    }

    #[test]
    fn load_missing_is_empty_and_bad_files_error() {
        let missing =
            std::env::temp_dir().join(format!("necro-baseline-nope-{}.yaml", std::process::id()));
        let _ = std::fs::remove_file(&missing);
        assert_eq!(
            Baseline::load(&missing).expect("missing is empty"),
            Baseline::empty()
        );

        let dir = std::env::temp_dir();
        let bad_version = dir.join(format!(
            "necro-baseline-version-{}.yaml",
            std::process::id()
        ));
        std::fs::write(&bad_version, "version: 99\nentries: []\n").expect("write fixture");
        assert!(matches!(
            Baseline::load(&bad_version),
            Err(LoadError::Version(_))
        ));
        let _ = std::fs::remove_file(&bad_version);

        let corrupt = dir.join(format!(
            "necro-baseline-corrupt-{}.yaml",
            std::process::id()
        ));
        std::fs::write(&corrupt, "version: [oops\n").expect("write fixture");
        assert!(matches!(Baseline::load(&corrupt), Err(LoadError::Parse(_))));
        let _ = std::fs::remove_file(&corrupt);
    }
}
