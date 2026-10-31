//! `necro`: checks a repo against its house rules (spec/v0.md).

mod allows;
// Lane 1's snapshot API: entries() serves later lanes (spec + status), so it
// has no caller yet.
#[allow(dead_code)]
mod baseline;
mod command;
mod config;
mod files;
mod finders;
mod render;
mod run;
mod yaml;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Default)]
struct Args {
    dir: Option<PathBuf>,
    format: Option<String>,
    rule: Option<String>,
    explain: Option<String>,
    write_baseline: bool,
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
            "--write-baseline" => a.write_baseline = true,
            f if f.starts_with('-') && f.len() > 1 => {
                return Err(format!("unknown option {f} (see necro --help)"));
            }
            _ => {
                return Err(format!(
                    "unexpected argument {arg} (commands choose their own files; see necro --help)"
                ));
            }
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
    let config = match config::load(&root, &files::list(&root)) {
        Ok(c) => c,
        Err(errors) => {
            render::config_errors(&errors);
            return Ok(2);
        }
    };
    let unknown = |name: &str| format!("no rule `{name}`; rules: {}", config.names().join(", "));

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
    let report = run::run(&config, &root, args.rule.as_deref());
    match args.format.as_deref() {
        Some("terse") => render::terse(&report),
        _ => render::full(&report),
    }
    render::problems(&report);
    render::stale_warning(&report);
    Ok(report.exit_code())
}

/// `--write-baseline`: snapshot the observed post-allow hits and confirm.
/// A full regen replaces the snapshot; a scoped one replaces only that
/// rule's entries and carries the rest over. Snapshot errors never block
/// regen (a missing snapshot is empty, a corrupt one is discarded and
/// rebuilt); problems fail first: a partial pipeline must not become the
/// snapshot.
fn write_baseline(config: &config::Config, root: &Path, only: Option<&str>) -> Result<u8, String> {
    let report = run::run_observed(config, root, only);
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
    Ok(0)
}
