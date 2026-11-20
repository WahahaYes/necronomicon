//! `necro godot` (spec: Finders): Godot's GDScript analyzer, through the
//! repo's own `godot`, loading the project's scripts headless and turning
//! the errors and warnings it prints into hits.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use regex::Regex;

use super::grep::read_list;
use super::known;
use crate::args::{self, Spec};
use crate::command::Hit;

const USAGE: &str = "necro godot [--files-from <file>] (compile|warnings) [-- <pathspec>…]";

/// The checks `godot` knows.
const CHECKS: &[&str] = &["compile", "warnings"];

/// The SceneTree script Godot runs: it loads each `res://` path listed in
/// the file named after `--`, so Godot prints what it finds in each.
const CHECKER: &str = r#"extends SceneTree
## necro's Godot check: load each res:// path listed in the file named
## after `--`, so Godot reports its errors and warnings.


func _initialize() -> void:
	var args: PackedStringArray = OS.get_cmdline_user_args()
	if args.is_empty():
		quit(2)
		return
	var list: String = FileAccess.get_file_as_string(args[0])
	for path: String in list.split("\n", false):
		ResourceLoader.load(path, "", ResourceLoader.CACHE_MODE_IGNORE)
	quit(0)
"#;

/// Errors that mean the class cache is missing or out of date.
const STALE: &[&str] = &[
    "Could not find base class",
    "Identifier not found",
    "Could not find type",
    "Could not resolve class",
];

/// The class cache an import writes, which the analyzer needs.
const CACHE: &str = ".godot/global_script_class_cache.cfg";

/// What a dependency's failure says on the scripts that load it.
const DEPENDED: &str = "Failed to compile depended scripts";

/// One message Godot printed about a script.
struct Message {
    error: bool,
    res: String,
    line: usize,
    text: String,
}

/// The `godot` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--files-from"],
            flags: &[],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    let check = args::check(&parsed.positional, CHECKS, USAGE)?;
    let only = parsed.value("--files-from").map(read_list).transpose()?;
    let here = std::env::current_dir()
        .and_then(|d| d.canonicalize())
        .map_err(|e| format!("can't read the working folder: {e}"))?;
    let projects = projects(&here)?;
    if projects.is_empty() {
        return Ok(vec![]);
    }
    let specs = if parsed.pathspecs.is_empty() {
        vec!["*.gd".to_string()]
    } else {
        parsed.pathspecs.clone()
    };
    let scripts: Vec<String> = known(&specs, None)?
        .into_iter()
        .filter(|p| p.ends_with(".gd"))
        .collect();
    let mut hits = Vec::new();
    let mut version: Option<(u32, u32)> = None;
    for project in &projects {
        let listed = only
            .as_ref()
            .is_some_and(|o| o.contains(&rel(&here, &project.join("project.godot"))));
        let mut files: Vec<String> = scripts
            .iter()
            .filter(|f| owner(&here, f, &projects) == Some(project))
            .filter(|f| only.is_none() || listed || only.as_ref().is_some_and(|o| o.contains(*f)))
            .filter(|f| !ignored(&here.join(f), project))
            .cloned()
            .collect();
        files.sort();
        if files.is_empty() {
            continue;
        }
        if version.is_none() {
            version = Some(godot_version()?);
        }
        if let Some(have) = version {
            same_version(&here, project, have)?;
        }
        let wanted: HashSet<&String> = files.iter().collect();
        let res: Vec<String> = files
            .iter()
            .filter_map(|f| here.join(f).strip_prefix(project).ok().map(res_path))
            .collect();
        let mut seen = BTreeSet::new();
        for m in messages(project, &res)? {
            if m.error != (check == "compile") {
                continue;
            }
            let path = rel(&here, &project.join(&m.res));
            if !wanted.contains(&path) {
                continue;
            }
            let key = if m.error {
                (path.clone(), m.line, String::new())
            } else {
                (path.clone(), m.line, m.text.clone())
            };
            if !seen.insert(key) {
                continue;
            }
            let label = if m.text.starts_with(DEPENDED) {
                "a script it depends on fails to compile".to_string()
            } else {
                m.text
            };
            hits.push(Hit {
                path,
                line: m.line,
                col: 0,
                endcol: None,
                label,
            });
        }
    }
    Ok(hits)
}

/// Godot projects: the folder enclosing the working folder, if any, and
/// each tracked `project.godot`'s folder below it, deepest first.
fn projects(here: &Path) -> Result<Vec<PathBuf>, String> {
    let mut found: Vec<PathBuf> = known(&["*project.godot".to_string()], None)?
        .into_iter()
        .filter(|p| {
            Path::new(p)
                .file_name()
                .is_some_and(|n| n == "project.godot")
        })
        .filter_map(|p| here.join(p).parent().map(Path::to_path_buf))
        .collect();
    if let Some(up) = here
        .ancestors()
        .skip(1)
        .find(|d| d.join("project.godot").is_file())
    {
        found.push(up.to_path_buf());
    }
    found.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    found.dedup();
    Ok(found)
}

/// The deepest project holding a script.
fn owner<'a>(here: &Path, file: &str, projects: &'a [PathBuf]) -> Option<&'a PathBuf> {
    let abs = here.join(file);
    projects.iter().find(|p| abs.starts_with(p))
}

/// Whether Godot ignores a script: a folder named with a leading `.`, or
/// one holding a `.gdignore`, between it and the project.
fn ignored(abs: &Path, project: &Path) -> bool {
    let Ok(inner) = abs.strip_prefix(project) else {
        return true;
    };
    let hidden = inner.parent().is_some_and(|d| {
        d.components()
            .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
    });
    hidden
        || abs
            .ancestors()
            .skip(1)
            .take_while(|d| d.starts_with(project) && *d != project)
            .any(|d| d.join(".gdignore").exists())
}

/// A path inside a project as Godot names it.
fn res_path(inner: &Path) -> String {
    format!("res://{}", inner.to_string_lossy().replace('\\', "/"))
}

/// A path relative to the working folder.
fn rel(here: &Path, abs: &Path) -> String {
    abs.strip_prefix(here)
        .unwrap_or(abs)
        .to_string_lossy()
        .into_owned()
}

/// What Godot says while loading the scripts, importing the project first
/// when it has no class cache, or again when an error reads as a stale one.
fn messages(project: &Path, res: &[String]) -> Result<Vec<Message>, String> {
    let mut imported = false;
    if !project.join(CACHE).is_file() {
        import(project)?;
        imported = true;
    }
    let found = load(project, res)?;
    let stale = found
        .iter()
        .any(|m| m.error && STALE.iter().any(|s| m.text.contains(s)));
    if stale && !imported {
        import(project)?;
        return load(project, res);
    }
    Ok(found)
}

/// The Godot to run: `$GODOT` when set, else `godot` on PATH.
fn program() -> Option<String> {
    std::env::var("GODOT").ok().filter(|g| !g.is_empty())
}

/// Start Godot with stdin closed, so its debugger never waits for input.
fn godot(args: &[&str]) -> Result<std::process::Output, String> {
    let named = program();
    let run = Command::new(named.as_deref().unwrap_or("godot"))
        .args(args)
        .stdin(Stdio::null())
        .output();
    match (run, named) {
        (Ok(o), _) => Ok(o),
        (Err(e), Some(g)) => Err(format!("can't run $GODOT ({g}): {e}")),
        (Err(e), None) if e.kind() == std::io::ErrorKind::NotFound => Err(
            "godot isn't on PATH; install the Godot version project.godot names (or set $GODOT to it), or disable gdscript/compile and gdscript/warnings"
                .to_string(),
        ),
        (Err(e), None) => Err(format!("can't run godot: {e}")),
    }
}

/// The oldest Godot whose analyzer prints warnings when a script loads.
const FLOOR: (u32, u32) = (4, 5);

/// The major.minor of a version string such as `4.7.2.stable.official`.
fn major_minor(text: &str) -> Option<(u32, u32)> {
    let mut parts = text.trim().split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    Some((major, minor))
}

/// `godot --version`'s major.minor, at least the floor.
fn godot_version() -> Result<(u32, u32), String> {
    let out = godot(&["--version"])?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().rev().find(|l| major_minor(l).is_some());
    let Some(have) = line.and_then(major_minor) else {
        return Err(format!(
            "godot --version printed no version: {}",
            text.trim()
        ));
    };
    if have < FLOOR {
        return Err(format!(
            "godot is Godot {}.{}; necro needs {}.{} or later, which prints GDScript warnings",
            have.0, have.1, FLOOR.0, FLOOR.1
        ));
    }
    Ok(have)
}

/// That `project.godot` names the Godot that's running, by major.minor.
fn same_version(here: &Path, project: &Path, have: (u32, u32)) -> Result<(), String> {
    let file = project.join("project.godot");
    let name = rel(here, &file);
    let text = std::fs::read_to_string(&file).map_err(|e| format!("can't read {name}: {e}"))?;
    let named = Regex::new(r#"config/features\s*=\s*PackedStringArray\(\s*"([0-9]+\.[0-9]+)"#)
        .ok()
        .and_then(|r| r.captures(&text).map(|c| c[1].to_string()));
    let Some(want) = named.as_deref().and_then(major_minor) else {
        return Err(format!(
            "{name} names no Godot version (config/features); save it once in the Godot editor"
        ));
    };
    if want != have {
        return Err(format!(
            "godot is Godot {}.{}, but {name} is edited with {}.{}; run that version, as `godot` on PATH or $GODOT",
            have.0, have.1, want.0, want.1
        ));
    }
    Ok(())
}

/// `godot --headless --import`, which writes the class cache under `.godot/`.
fn import(project: &Path) -> Result<(), String> {
    let path = project.to_string_lossy();
    let out = godot(&["--headless", "--path", &path, "--import"])?;
    // Godot's .NET builds exit nonzero importing a project without C#, yet
    // write the cache: the cache, not the exit code, says it worked.
    if project.join(CACHE).is_file() {
        return Ok(());
    }
    let text = String::from_utf8_lossy(&out.stderr);
    let tail: Vec<&str> = text.lines().rev().take(5).collect();
    Err(format!(
        "godot --import failed in {path}: {}",
        tail.into_iter().rev().collect::<Vec<_>>().join(" / ")
    ))
}

/// Load the scripts in one headless Godot and read what it prints.
fn load(project: &Path, res: &[String]) -> Result<Vec<Message>, String> {
    let dir = std::env::temp_dir().join(format!(
        "necro-godot-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("can't write a temp folder: {e}"))?;
    let checker = dir.join("necro-godot-check.gd");
    let list = dir.join("scripts.txt");
    let written = std::fs::write(&checker, CHECKER)
        .and_then(|()| std::fs::write(&list, res.join("\n") + "\n"))
        .map_err(|e| format!("can't write a temp file: {e}"));
    let out = written.and_then(|()| {
        godot(&[
            "--headless",
            "--debug",
            "--path",
            &project.to_string_lossy(),
            "-s",
            &checker.to_string_lossy(),
            "--",
            &list.to_string_lossy(),
        ])
    });
    // necro-allow rust/swallowed-error: a leftover temp folder is harmless; the result matters more
    let _ = std::fs::remove_dir_all(&dir);
    let out = out?;
    if out.status.code().is_none() {
        return Err(format!(
            "godot stopped without an exit code in {}",
            project.display()
        ));
    }
    let mut found = parse(&String::from_utf8_lossy(&out.stdout));
    found.extend(parse(&String::from_utf8_lossy(&out.stderr)));
    Ok(found)
}

/// `SCRIPT ERROR: <kind> Error: <text>` or `WARNING: <text>`, each followed
/// by `at: GDScript::reload (res://<path>:<line>)`. Anything else is chatter.
fn parse(text: &str) -> Vec<Message> {
    let (Ok(at), Ok(kind)) = (
        Regex::new(r"^at: GDScript::reload \(res://(.+):(\d+)\)$"),
        Regex::new(r"^[A-Za-z ]+ Error: "),
    ) else {
        return vec![];
    };
    let mut found = Vec::new();
    let mut open: Option<(bool, String)> = None;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("SCRIPT ERROR: ") {
            open = Some((true, kind.replace(rest, "").into_owned()));
            continue;
        }
        if let Some(rest) = line.strip_prefix("WARNING: ") {
            open = Some((false, rest.to_string()));
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.starts_with("at: ") {
            continue;
        }
        if let Some((error, text)) = open.take()
            && let Some(c) = at.captures(trimmed)
        {
            found.push(Message {
                error,
                res: c[1].to_string(),
                line: c[2].parse().unwrap_or(0),
                text,
            });
        }
    }
    found
}
