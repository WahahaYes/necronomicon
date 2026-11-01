//! Everything necro prints (spec: Messages, CLI). Diagnostics and the summary
//! go to stdout; config, check and usage errors to stderr.

use std::io::Write;

pub mod explain;
pub mod full;

use crate::config::ConfigError;
use crate::run::{Problem, Report};

/// Config errors to stderr, one per line.
pub fn config_errors(errors: &[ConfigError]) {
    let mut err = std::io::stderr().lock();
    for e in errors {
        let path = if e.path.is_empty() {
            String::new()
        } else {
            format!("{}: ", e.path)
        };
        let _ = writeln!(
            err,
            "necro: config error: {}:{}:{}: {path}{}",
            e.file, e.pos.line, e.pos.col, e.message
        );
    }
}

/// Setup and check errors, after the hits.
pub fn problems(report: &Report) {
    let mut err = std::io::stderr().lock();
    for p in &report.problems {
        match p {
            Problem::Setup(message) => {
                let _ = writeln!(err, "necro: setup error: {message}");
            }
            Problem::Check {
                rule,
                message,
                stderr,
            } => {
                let _ = writeln!(err, "necro: check error: {rule}: {message}");
                for line in stderr {
                    let _ = writeln!(err, "  | {line}");
                }
            }
        }
    }
}

/// One setup error to stderr.
pub fn setup_error(message: &str) {
    let _ = writeln!(std::io::stderr(), "necro: setup error: {message}");
}

/// Snapshot entries nothing consumed, after the hits: a warning, never a failure.
pub fn stale_warning(report: &Report) {
    if report.stale_baseline > 0 {
        let n = report.stale_baseline;
        let _ = writeln!(
            std::io::stderr(),
            "necro: warning: {n} baseline {} no longer {}; refresh with necro --write-baseline",
            if n == 1 { "entry" } else { "entries" },
            if n == 1 { "matches" } else { "match" }
        );
    }
}

/// The rules whose commands are still running, one line each, to stderr.
pub fn still_running(rules: &[&str], secs: u64) {
    let mut err = std::io::stderr().lock();
    for rule in rules {
        let _ = writeln!(err, "necro: still running after {secs} s: {rule}");
    }
}

/// One usage error to stderr.
pub fn usage(message: &str) {
    let _ = writeln!(std::io::stderr(), "necro: {message}");
}

/// Finder hits to stdout, one per line.
pub fn lines(lines: &[String]) {
    let mut out = std::io::stdout().lock();
    for l in lines {
        let _ = writeln!(out, "{l}");
    }
}

/// One line to stdout.
pub fn line(text: &str) {
    let _ = writeln!(std::io::stdout(), "{text}");
}

/// One report in the `full` format to stdout.
pub fn full(report: &Report) {
    let _ = write!(std::io::stdout(), "{}", full::render(report));
}

/// Pre-rendered text to stdout.
pub fn text(text: &str) {
    let _ = write!(std::io::stdout(), "{text}");
}

/// One report in the `terse` format to stdout.
pub fn terse(report: &Report) {
    let mut out = std::io::stdout().lock();
    for d in &report.diagnostics {
        let at = if d.line == 0 {
            d.file.clone()
        } else {
            format!("{}:{}:{}", d.file, d.line, d.col)
        };
        let _ = writeln!(out, "{at}: {} {}", d.rule, d.message);
    }
}

/// The `--help` text: usage, finders and exit codes.
pub const HELP: &str = "\
necro: checks a repo against its house rules (necro.yaml).

usage: necro [-C <dir>] [--format full|terse] [--rule <name>]
       necro [-C <dir>] --explain <rule>
       necro [-C <dir>] [--rule <name>] --write-baseline
       necro grep [--skip-comments <marker>]… <regex> [-- <pathspec>…]
       necro --version

Each rule's command chooses its own files.

Exit 0 clean, 1 on an error hit or a broken allow, 2 on a config, setup, check
or usage error.";
