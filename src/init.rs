//! `necro init`: write a starter `necro.yaml` (spec: CLI).

use std::path::Path;

/// The languages `init` can detect, in priority order.
const LANGUAGES: &[(&str, &[&str], &str)] = &[
    ("python", &["py"], PYTHON_RULE),
    ("rust", &["rs"], RUST_RULE),
    ("gdscript", &["gd"], GDSCRIPT_RULE),
    ("go", &["go"], GO_RULE),
    ("javascript", &["js", "ts", "jsx", "tsx"], JS_RULE),
];

const PYTHON_RULE: &str = "  no-print:
    command: necro grep --skip-comments '#' '\\bprint\\(' -- '*.py'
    message: print call in library code
    why: Library code reports through logging, so the caller decides where output goes.
    fix: Log with logging.getLogger(__name__) at the right level.
";

const RUST_RULE: &str = "  no-todo:
    command: necro grep '\\btodo!\\(' -- '*.rs'
    message: todo! left in code
    why: A todo! panics at runtime; it's a placeholder, not a plan.
    fix: Implement the case, or return an error.
";

const GDSCRIPT_RULE: &str = "  no-print:
    command: necro grep --skip-comments '#' '\\bprint\\(' -- '*.gd'
    message: print call in game code
    why: Print output bypasses the structured game log.
    fix: Log through your game's log system, or push_error for real failures.
";

const GO_RULE: &str = "  no-println:
    command: necro grep '\\bfmt\\.Println\\(' -- '*.go'
    message: fmt.Println in library code
    why: Library code should return errors, not print to stdout.
    fix: Return an error, or use the log package.
";

const JS_RULE: &str = "  no-console-log:
    command: necro grep '\\bconsole\\.log\\(' -- '*.js' '*.ts' '*.jsx' '*.tsx'
    message: console.log left in code
    why: Console output is for debugging, not production.
    fix: Remove it, or use a proper logger.
";

const GENERIC_RULE: &str = "  # Replace this with a rule for your project. A rule needs:
  #   command: a command that prints path[:line[:col]] for each violation
  #   message: what's wrong (one line)
  #   why:     why the rule exists
  #   fix:     what to do instead
  # Example:
  # no-fixme:
  #   command: necro grep '\\bFIXME\\b' -- '*'
  #   message: FIXME left in code
  #   why: FIXMEs are reminders, not plans; they rot.
  #   fix: Fix it or file an issue and delete the comment.
";

const HEADER: &str = "\
# House rules: each rule is a command that finds violations, plus the words
# that teach whoever trips it what's wrong and what to do instead.
# Docs: https://github.com/WahahaYes/necronomicon/blob/main/docs/writing-rules.md
version: 0
";

const PACK_COMMENT: &str = "\
# Uncomment to add built-in rules for your language:
";

/// Write a starter `necro.yaml` in `dir`. Returns a message to print on
/// success, or an error string on failure.
pub fn run(dir: &Path) -> Result<String, String> {
    let target = dir.join("necro.yaml");
    if target.exists() {
        return Err(format!(
            "necro.yaml already exists in {}; nothing to do",
            dir.display()
        ));
    }

    let (lang, rule, pack_line) = detect_language(dir);

    let mut content = String::from(HEADER);
    if let Some(pack) = pack_line {
        content.push_str(PACK_COMMENT);
        content.push_str(&format!("# include: [necro:{pack}]\n"));
    }
    content.push_str("rules:\n");
    content.push_str(rule);

    std::fs::write(&target, &content).map_err(|e| format!("can't write necro.yaml: {e}"))?;

    let mut msg = String::from("necro: wrote necro.yaml");
    if let Some(name) = lang {
        msg.push_str(&format!(" (detected {name})"));
    }
    msg.push_str(
        "\n\nNext steps:\
         \n  1. Run necro to see it check your code\
         \n  2. Edit necro.yaml: change the starter rule or write your own\
         \n  3. Set up the pre-commit hook (see the README)\
         \n\nDocs: https://github.com/WahahaYes/necronomicon/blob/main/docs/writing-rules.md",
    );
    Ok(msg)
}

/// Pick a language from tracked file extensions. Returns (language name, rule
/// text, pack name for the include comment).
fn detect_language(dir: &Path) -> (Option<&'static str>, &'static str, Option<&'static str>) {
    let extensions = tracked_extensions(dir);
    for &(lang, exts, rule) in LANGUAGES {
        if exts.iter().any(|e| extensions.contains(&e.to_string())) {
            let pack = match lang {
                "python" => Some("python"),
                "rust" => Some("rust"),
                "gdscript" => Some("gdscript"),
                _ => None,
            };
            return (Some(lang), rule, pack);
        }
    }
    (None, GENERIC_RULE, None)
}

/// The file extensions of tracked files, via `git ls-files`.
fn tracked_extensions(dir: &Path) -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["ls-files", "--cached", "--others", "--exclude-standard"])
        .current_dir(dir)
        .output();
    if let Ok(output) = output
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut exts: Vec<String> = text
            .lines()
            .filter_map(|line| {
                Path::new(line)
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase())
            })
            .collect();
        exts.sort();
        exts.dedup();
        if !exts.is_empty() {
            return exts;
        }
    }
    // Fallback if not a git repo or no files found by git: inspect files in dir and src/
    let mut exts = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    exts.push(ext.to_lowercase());
                }
            } else if path.is_dir()
                && path.file_name().is_some_and(|n| n == "src")
                && let Ok(src_entries) = std::fs::read_dir(&path)
            {
                for src_entry in src_entries.flatten() {
                    if let Some(ext) = src_entry.path().extension().and_then(|e| e.to_str()) {
                        exts.push(ext.to_lowercase());
                    }
                }
            }
        }
    }
    exts.sort();
    exts.dedup();
    exts
}
