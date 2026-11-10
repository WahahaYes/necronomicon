//! `necro`: checks a repo against its house rules (spec/v0.md).

mod allows;
mod args;
#[expect(dead_code, reason = "entries() serves the status lane; no caller yet")]
mod baseline;
mod command;
mod config;
mod files;
mod finders;
mod packs;
mod render;
mod run;
mod skills;
mod yaml;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Default)]
struct Args {
    dir: Option<PathBuf>,
    format: Option<String>,
    rule: Option<String>,
    explain: Option<String>,
    list: bool,
    fix: bool,
    write_baseline: bool,
    clean_cache: bool,
    /// Files for a narrowed run, as given (spec: Narrowed runs).
    files: Vec<String>,
}

fn parse(argv: Vec<String>) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        let mut value = |name: &str, inline: Option<&str>| -> Result<String, String> {
            inline
                .map(String::from)
                .or_else(|| it.next())
                .ok_or(format!("{name} needs a value"))
        };
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if arg.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        match flag.as_str() {
            "-C" => a.dir = Some(PathBuf::from(value("-C", None)?)),
            "--format" => a.format = Some(value("--format", inline.as_deref())?),
            "--rule" => a.rule = Some(value("--rule", inline.as_deref())?),
            "--explain" => a.explain = Some(value("--explain", inline.as_deref())?),
            "--list" => a.list = true,
            "--fix" => a.fix = true,
            "--write-baseline" => a.write_baseline = true,
            "--clean-cache" => a.clean_cache = true,
            f if f.starts_with('-') && f.len() > 1 => {
                return Err(format!("unknown option {f} (see necro --help)"));
            }
            _ => a.files.push(arg),
        }
    }
    if a.fix {
        let refuses = [
            (a.write_baseline, "--write-baseline"),
            (a.explain.is_some(), "--explain"),
            (a.list, "--list"),
            (a.clean_cache, "--clean-cache"),
        ];
        if let Some((_, other)) = refuses.iter().find(|(set, _)| *set) {
            return Err(format!("{other} doesn't fix; drop --fix"));
        }
    }
    if !a.files.is_empty() {
        let refuses = [
            (a.write_baseline, "--write-baseline covers every file"),
            (a.explain.is_some(), "--explain takes no files"),
            (a.list, "--list takes no files"),
            (a.clean_cache, "--clean-cache takes no files"),
        ];
        if let Some((_, why)) = refuses.iter().find(|(set, _)| *set) {
            return Err(format!("{why}; drop the file arguments"));
        }
    }
    match a.format.as_deref() {
        None | Some("full") | Some("terse") => Ok(a),
        Some(f) => Err(format!("unknown format {f}; use full or terse")),
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(code) = finders::dispatch(&argv) {
        return ExitCode::from(code);
    }
    if argv.first().is_some_and(|a| a == "skills") {
        return match skills::run(&argv[1..]) {
            Ok(lines) => {
                render::lines(&lines);
                ExitCode::SUCCESS
            }
            Err(message) => {
                render::usage(&format!("skills: {message}"));
                ExitCode::from(2)
            }
        };
    }
    if argv.iter().any(|a| a == "--version") {
        render::line(&format!("necro {}", env!("CARGO_PKG_VERSION")));
        return ExitCode::SUCCESS;
    }
    if argv.iter().any(|a| a == "--help" || a == "-h") {
        render::line(render::HELP);
        return ExitCode::SUCCESS;
    }
    match parse(argv).and_then(main_with) {
        Ok(code) => ExitCode::from(code),
        Err(message) => {
            render::usage(&message);
            ExitCode::from(2)
        }
    }
}

fn main_with(args: Args) -> Result<u8, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("can't read the current folder: {e}"))?;
    let start = match &args.dir {
        Some(d) => files::absolute(&cwd, d),
        None => cwd.clone(),
    };
    let root = files::find_root(&start).ok_or(format!(
        "no necro.yaml in {} or any folder above it",
        start.display()
    ))?;
    if !files::in_git(&root) {
        render::setup_error(&format!("{} isn't inside a git work tree", root.display()));
        return Ok(2);
    }
    if args.clean_cache {
        return clean_cache(&root);
    }
    let config = match config::load(&root, &files::list(&root)) {
        Ok(c) => c,
        Err(errors) => {
            render::config_errors(&errors);
            return Ok(2);
        }
    };
    let unknown = |name: &str| format!("no rule `{name}`; rules: {}", config.names().join(", "));

    if args.list {
        render::text(&render::list(&config));
        return Ok(0);
    }
    if let Some(name) = &args.explain {
        let builtins = config::builtins();
        let rule = config
            .rules
            .iter()
            .chain(builtins.iter())
            .find(|r| &r.name == name)
            .ok_or_else(|| unknown(name))?;
        render::text(&render::explain::render(rule));
        return Ok(0);
    }
    if let Some(name) = &args.rule
        && !config.rules.iter().any(|r| &r.name == name)
    {
        return Err(unknown(name));
    }
    if args.write_baseline {
        return write_baseline(&config, &root, args.rule.as_deref());
    }
    // Given files: a narrowed run over the ones that exist and the top-level
    // exclude keeps, root-relative.
    let given: Option<Vec<String>> = (!args.files.is_empty()).then(|| {
        let mut given: Vec<String> = args
            .files
            .iter()
            .filter_map(|f| files::anchor(&cwd, &root, Path::new(f)))
            .filter(|f| !config.excludes(f))
            .collect();
        given.sort();
        given.dedup();
        given
    });
    let terse = args.format.as_deref() == Some("terse");
    if args.fix {
        let done = run::fix(&config, &root, args.rule.as_deref(), given.as_deref());
        let report = &done.report;
        render::fixed(&done.fixed, done.changed, report.diagnostics.len());
        if terse {
            render::terse(report);
        } else {
            render::full(report);
        }
        render::problems(report);
        render::stale_warning(report);
        let code = report.exit_code();
        return Ok(if done.changed > 0 { code.max(1) } else { code });
    }
    let report = run::run(&config, &root, args.rule.as_deref(), given.as_deref());
    if terse {
        render::terse(&report);
    } else {
        render::full(&report);
        render::fixable(
            run::fixable(&config, &report.diagnostics),
            report.diagnostics.len(),
        );
    }
    render::problems(&report);
    render::stale_warning(&report);
    Ok(report.exit_code())
}

/// `--write-baseline`: snapshot post-allow hits and confirm (spec: Regen).
fn write_baseline(config: &config::Config, root: &Path, only: Option<&str>) -> Result<u8, String> {
    let report = run::run_observed(config, root, only, None);
    if !report.problems.is_empty() {
        render::problems(&report);
        return Ok(2);
    }
    let path = config.baseline_path(root);
    let fresh = baseline::Baseline::from_diagnostics(&report.diagnostics);
    let baseline = match only {
        Some(rule) => baseline::Baseline::load(&path)
            .unwrap_or_default()
            .merge(rule, &fresh),
        None => fresh,
    };
    if let Err(message) = baseline.save(&path) {
        render::setup_error(&message);
        return Ok(2);
    }
    let shown = path.strip_prefix(root).unwrap_or(&path);
    let n = baseline.len();
    render::line(&format!(
        "necro: wrote {} ({} {})",
        shown.display(),
        n,
        if n == 1 { "entry" } else { "entries" }
    ));
    let fixable = run::fixable(config, &report.diagnostics);
    if fixable > 0 {
        render::line(&format!(
            "necro: {fixable} of them can be fixed automatically; to keep them out of the \
             baseline, run necro --fix first, then --write-baseline again"
        ));
    }
    Ok(0)
}

/// `--clean-cache`: remove the Cargo finders' build folders (spec: CLI).
fn clean_cache(root: &Path) -> Result<u8, String> {
    std::env::set_current_dir(root).map_err(|e| format!("can't enter {}: {e}", root.display()))?;
    let removed = match finders::clean_cache() {
        Ok(removed) => removed,
        Err(message) => {
            render::setup_error(&message);
            return Ok(2);
        }
    };
    if removed.is_empty() {
        render::line("necro: no cache to clean");
    }
    for (folder, size) in removed {
        render::line(&format!("necro: removed {folder} ({size})"));
    }
    Ok(0)
}
