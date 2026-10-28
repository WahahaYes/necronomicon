//! `necro test` output: failures with their evidence, then one count line.

use std::fmt::Write as _;

use crate::fixtures::{Outcome, Problem};

/// A line diff (longest common subsequence): `-` expected, `+` actual, `  ` shared.
fn diff(expected: &str, actual: &str) -> String {
    let a: Vec<&str> = expected.lines().collect();
    let b: Vec<&str> = actual.lines().collect();
    let mut lcs = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let (mut i, mut j, mut out) = (0, 0, String::new());
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            let _ = writeln!(out, "    {}", a[i]);
            (i, j) = (i + 1, j + 1);
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            let _ = writeln!(out, "  - {}", a[i]);
            i += 1;
        } else {
            let _ = writeln!(out, "  + {}", b[j]);
            j += 1;
        }
    }
    out
}

pub fn render(outcomes: &[Outcome]) -> String {
    let mut out = String::new();
    for o in outcomes {
        for p in &o.problems {
            match p {
                Problem::Missing(kind) => {
                    let _ = writeln!(out, "FAIL {}: no {kind}/ fixture", o.code);
                }
                Problem::Good(lines) => {
                    let _ = writeln!(out, "FAIL {}: good/ isn't clean:", o.code);
                    for l in lines {
                        let _ = writeln!(out, "  {l}");
                    }
                }
                Problem::BadClean => {
                    let _ = writeln!(out, "FAIL {}: bad/ has no {} hit", o.code, o.code);
                }
                Problem::BadCheckError(lines) => {
                    let _ = writeln!(out, "FAIL {}: bad/ didn't run:", o.code);
                    for l in lines {
                        let _ = writeln!(out, "  {l}");
                    }
                }
                Problem::Snapshot { expected: None, .. } => {
                    let _ = writeln!(
                        out,
                        "FAIL {}: no expected.txt; run necro test --update and review it",
                        o.code
                    );
                }
                Problem::Snapshot {
                    expected: Some(e),
                    actual,
                } => {
                    let _ = writeln!(
                        out,
                        "FAIL {}: bad/ output differs from expected.txt (- expected, + actual):",
                        o.code
                    );
                    out.push_str(&diff(e, actual));
                }
            }
        }
        if o.updated {
            let _ = writeln!(out, "updated {}/expected.txt", o.code);
        }
    }
    let passed = outcomes.iter().filter(|o| o.problems.is_empty()).count();
    let _ = writeln!(out, "necro test: {passed} of {} rules pass", outcomes.len());
    out
}
