//! Running a rule's command and reading what it prints (spec: Commands).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(120);
const STDERR_TAIL: usize = 20;

/// Split `s` into words as a POSIX shell would (quotes, backslashes), with no
/// expansion. `None` on an unclosed quote.
pub fn split(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => cur.push(c),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' if matches!(chars.peek(), Some('"' | '\\' | '$')) => {
                            cur.push(chars.next()?)
                        }
                        c => cur.push(c),
                    }
                }
            }
            '\\' => {
                in_word = true;
                cur.push(chars.next()?);
            }
            c if c.is_whitespace() => {
                if in_word {
                    out.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        out.push(cur);
    }
    Some(out)
}

/// Why a rule's command produced no usable hits.
pub enum Failure {
    /// The program isn't on PATH (a setup error).
    Missing(String),
    /// A check error, with the end of the command's stderr.
    Check(String, Vec<String>),
}

/// Run `command` in `root` and return its stdout lines.
pub fn run(command: &str, root: &Path) -> Result<Vec<String>, Failure> {
    let words = split(command).unwrap_or_default();
    let Some((program, args)) = words.split_first() else {
        return Err(Failure::Check("the command is empty".into(), vec![]));
    };
    let mut cmd = Command::new(program);
    // necro's own folder first, so `necro grep` runs as this necro.
    if let Some(path) = path_with_necro() {
        cmd.env("PATH", path);
    }
    // git sets these for hooks in a worktree, and git then takes the root as
    // the repo's top; without them it finds the repo from the root (spec:
    // Commands).
    cmd.env_remove("GIT_DIR").env_remove("GIT_WORK_TREE");
    cmd.args(args)
        .current_dir(root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(Failure::Missing(program.clone()));
        }
        Err(e) => return Err(Failure::Check(format!("can't run {program}: {e}"), vec![])),
    };
    let mut stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");
    let out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err = std::thread::spawn(move || {
        let mut tail = VecDeque::new();
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if tail.len() == STDERR_TAIL {
                tail.pop_front();
            }
            tail.push_back(line);
        }
        Vec::from(tail)
    });
    let deadline = Instant::now() + TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                kill(&mut child);
                break None;
            }
        }
    };
    let stdout = out.join().unwrap_or_default();
    let tail = err.join().unwrap_or_default();
    let Some(status) = status else {
        let secs = TIMEOUT.as_secs();
        return Err(Failure::Check(format!("timed out after {secs} s"), tail));
    };
    match status.code() {
        Some(0 | 1) => {}
        Some(n) => return Err(Failure::Check(format!("exited with status {n}"), tail)),
        None => return Err(Failure::Check("was killed by a signal".into(), tail)),
    }
    Ok(String::from_utf8_lossy(&stdout)
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect())
}

fn path_with_necro() -> Option<std::ffi::OsString> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    let rest = std::env::var_os("PATH").unwrap_or_default();
    std::env::join_paths(std::iter::once(dir).chain(std::env::split_paths(&rest))).ok()
}

fn kill(child: &mut std::process::Child) {
    #[cfg(unix)]
    // SAFETY: signals the process group the child leads (process_group(0)).
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// One violation as printed: `path[:line[:col[-endcol]]][:label]`.
#[derive(Debug, PartialEq)]
pub struct Hit {
    pub path: String,
    /// 0 for a whole-file hit.
    pub line: usize,
    pub col: usize,
    pub endcol: Option<usize>,
    pub label: String,
}

fn number(s: &str) -> Option<usize> {
    (!s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        .then(|| s.parse().ok())
        .flatten()
        .filter(|&n| n > 0)
}

fn label(s: &str) -> String {
    s.strip_prefix(' ').unwrap_or(s).to_string()
}

/// Parse one output line. After the path, a field that doesn't start with a
/// digit is the label; at the path level it must start with a space.
pub fn parse(line: &str) -> Option<Hit> {
    let (path, rest) = match line.split_once(':') {
        None => (line, None),
        Some((p, r)) => (p, Some(r)),
    };
    let mut hit = Hit {
        path: path.to_string(),
        line: 0,
        col: 0,
        endcol: None,
        label: String::new(),
    };
    if path.is_empty() {
        return None;
    }
    let Some(rest) = rest else { return Some(hit) };
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        if !rest.is_empty() && !rest.starts_with(' ') {
            return None;
        }
        hit.label = label(rest);
        return Some(hit);
    }
    let (l, rest) = match rest.split_once(':') {
        None => (rest, None),
        Some((l, r)) => (l, Some(r)),
    };
    hit.line = number(l)?;
    hit.col = 1;
    let Some(rest) = rest else { return Some(hit) };
    if !rest.starts_with(|c: char| c.is_ascii_digit()) {
        hit.label = label(rest);
        return Some(hit);
    }
    let (c, rest) = match rest.split_once(':') {
        None => (rest, None),
        Some((c, r)) => (c, Some(r)),
    };
    match c.split_once('-') {
        Some((c, end)) => {
            hit.col = number(c)?;
            hit.endcol = Some(number(end)?);
        }
        None => hit.col = number(c)?,
    }
    if let Some(rest) = rest {
        hit.label = label(rest);
    }
    Some(hit)
}
