//! `necro`: checks a repo against its house rules (spec/v0.md).

mod checks;
mod config;
mod files;
mod fixtures;
mod lang;
mod render;
mod run;
mod script;
mod yaml;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use script::Process;

#[derive(Default)]
struct Args {
    dir: Option<PathBuf>,
    files: Vec<String>,
    format: Option<String>,
    rule: Option<String>,
    explain: Option<String>,
    test: bool,
    update: bool,
}

fn parse(argv: Vec<String>) -> Result<Args, String> {
    let mut a = Args::default();
    let mut it = argv.into_iter();
    let mut options = true;
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
            "--" if options => options = false,
            "-C" if options => a.dir = Some(PathBuf::from(value("-C", None)?)),
            "--format" if options => a.format = Some(value("--format", inline.as_deref())?),
            "--rule" if options => a.rule = Some(value("--rule", inline.as_deref())?),
            "--explain" if options => a.explain = Some(value("--explain", inline.as_deref())?),
            "--update" if options && a.test => a.update = true,
            "test" if options && !a.test && a.files.is_empty() => a.test = true,
            f if options && f.starts_with('-') && f.len() > 1 => {
                return Err(format!("unknown option {f} (see necro --help)"));
            }
            _ => a.files.push(arg),
        }
    }
    Ok(a)
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
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
    let cwd = match &args.dir {
        Some(d) => files::absolute(&cwd, d),
        None => cwd,
    };
    let root = files::find_root(&cwd).ok_or(format!(
        "no necro.yaml in {} or any folder above it",
        cwd.display()
    ))?;
    match args.format.as_deref() {
        None | Some("full") | Some("terse") => {}
        Some("json") => return Err("--format json arrives in a later 0.x".into()),
        Some(f) => return Err(format!("unknown format {f}; use full or terse")),
    }

    let mut scripts: BTreeMap<String, Process> = BTreeMap::new();
    let loaded = {
        let mut describe = |group: &config::Group| {
            let mut p = Process::spawn(&root, &root, group).map_err(|f| f.message)?;
            let decls = p.describe().map_err(|f| f.message)?;
            scripts.insert(group.name.clone(), p);
            Ok(decls)
        };
        config::load(&root, &mut describe)
    };
    let loaded = match loaded {
        Ok(l) => l,
        Err(errors) => {
            render::config_errors(&errors);
            return Ok(2);
        }
    };
    let cfg = &loaded.config;

    if args.test {
        drop(scripts);
        let outcomes = fixtures::test(&root, &loaded, args.update);
        render::text(&render::test::render(&outcomes));
        return Ok(if outcomes.iter().all(|o| o.problems.is_empty()) {
            0
        } else {
            1
        });
    }
    if let Some(name) = &args.explain {
        render::text(&render::explain::render(cfg, name)?);
        return Ok(0);
    }
    let all = files::list(&root);
    let given = if args.files.is_empty() {
        None
    } else {
        Some(given_files(&cwd, &root, &args.files)?)
    };
    if let Some(code) = &args.rule
        && !cfg.rules.iter().any(|r| &r.code == code)
    {
        return Err(format!("no rule {code}"));
    }
    let overlaps = config::where_overlaps(cfg, &all);
    if !overlaps.is_empty() {
        render::config_errors(&overlaps);
        return Ok(2);
    }
    let opts = run::Options {
        given: given.as_deref(),
        rule: args.rule.as_deref(),
    };
    let report = run::run(cfg, &loaded.broken, &mut scripts, &root, &all, &opts);
    if !report.config_errors.is_empty() {
        render::config_errors(&report.config_errors);
        return Ok(2);
    }
    match args.format.as_deref() {
        Some("terse") => render::terse(&report),
        _ => render::full(&report),
    }
    render::check_errors(&report);
    Ok(report.exit_code())
}

/// Files named on the command line, relative to the root; outside it they're skipped.
fn given_files(cwd: &Path, root: &Path, names: &[String]) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for name in names {
        let abs = files::absolute(cwd, Path::new(name));
        if !abs.is_file() {
            return Err(format!("{name}: no such file"));
        }
        if let Some(rel) = files::rel(root, &abs) {
            out.push(rel);
        }
    }
    Ok(out)
}
