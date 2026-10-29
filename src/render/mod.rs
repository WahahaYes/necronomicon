//! Everything necro prints (spec: Messages, CLI). Diagnostics and the summary
//! go to stdout; config, check and usage errors to stderr.

use std::io::Write;

pub mod explain;
pub mod full;

use crate::config::ConfigError;
use crate::run::{Problem, Report};

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

pub fn setup_error(message: &str) {
    let _ = writeln!(std::io::stderr(), "necro: setup error: {message}");
}

pub fn usage(message: &str) {
    let _ = writeln!(std::io::stderr(), "necro: {message}");
}

pub fn lines(lines: &[String]) {
    let mut out = std::io::stdout().lock();
    for l in lines {
        let _ = writeln!(out, "{l}");
    }
}

pub fn line(text: &str) {
    let _ = writeln!(std::io::stdout(), "{text}");
}

pub fn full(report: &Report) {
    let _ = write!(
        std::io::stdout(),
        "{}",
        full::render(report, full::necro_on_path())
    );
}

pub fn text(text: &str) {
    let _ = write!(std::io::stdout(), "{text}");
}

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

pub const HELP: &str = "\
necro: checks a repo against its house rules (necro.yaml).

usage: necro [-C <dir>] [--format full|terse] [--rule <name>]
       necro [-C <dir>] --explain <rule>
       necro [-C <dir>] baseline
       necro grep [--skip-comments <marker>]… <regex> [-- <pathspec>…]
       necro --version

Each rule's command chooses its own files. `necro baseline` writes necro.baseline,
the hits a repo accepts while it adopts a rule.

Exit 0 clean, 1 on an error hit or a broken allow, 2 on a config, setup, check
or usage error.";
