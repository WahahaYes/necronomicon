//! Everything necro prints (spec: Messages, CLI). Diagnostics and the summary
//! go to stdout; config, check and usage errors to stderr.

use std::io::Write;

pub mod explain;
pub mod full;

use crate::config::ConfigError;
use crate::run::Report;

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

pub fn check_errors(report: &Report) {
    let mut err = std::io::stderr().lock();
    for c in &report.check_errors {
        let _ = writeln!(
            err,
            "necro: check error: {} ({}): {}",
            c.group, c.command, c.failure.message
        );
        for line in &c.failure.stderr {
            let _ = writeln!(err, "  | {line}");
        }
    }
}

pub fn usage(message: &str) {
    let _ = writeln!(std::io::stderr(), "necro: {message}");
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
        let _ = writeln!(
            out,
            "{}:{}:{}: {} {}",
            d.file, d.line, d.col, d.code, d.summary
        );
    }
}

pub const HELP: &str = "\
necro: checks a repo against its house rules (necro.yaml).

usage: necro [-C <dir>] [files…] [--format full|terse] [--rule <code>]
       necro [-C <dir>] --explain <code|group>
       necro [-C <dir>] test [--update]
       necro --version

Exit 0 clean, 1 on an error hit, 2 on a config error, check failure or usage error.";
