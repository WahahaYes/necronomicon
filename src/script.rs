//! A group's `command`, spoken to over the check protocol (spec: Check protocol):
//! one process per group per run, `describe` then one `check`, NDJSON both ways.

use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value as Json, json};

use crate::checks::Value;
use crate::config::{CheckDecl, Group, ParamDecl, ParamType, ParamValue, Scope};

pub const PROTOCOL: i64 = 1;
const STDERR_TAIL: usize = 20;

/// Why a check couldn't run, with the end of its stderr.
#[derive(Debug, Clone)]
pub struct Failure {
    pub message: String,
    pub stderr: Vec<String>,
}

pub struct Process {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    stderr: Arc<Mutex<VecDeque<String>>>,
    deadline: Instant,
    done: bool,
}

pub struct Job {
    pub id: String,
    pub check: String,
    pub files: Vec<String>,
    pub params: BTreeMap<String, ParamValue>,
}

#[derive(Debug, Clone)]
pub struct ScriptHit {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub values: BTreeMap<String, Value>,
}

#[derive(Default)]
pub struct Response {
    pub results: BTreeMap<String, Vec<ScriptHit>>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

fn param_json(v: &ParamValue) -> Json {
    match v {
        ParamValue::Str(s) => json!(s),
        // Whole numbers go as integers, so scripts can use them as columns and counts.
        ParamValue::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => json!(*n as i64),
        ParamValue::Number(n) => json!(n),
        ParamValue::Bool(b) => json!(b),
        ParamValue::StrList(l) => json!(l),
        ParamValue::StrMap(m) => json!(m),
    }
}

fn param_from_json(ty: ParamType, v: &Json) -> Option<ParamValue> {
    Some(match ty {
        ParamType::Str => ParamValue::Str(v.as_str()?.into()),
        ParamType::Number => ParamValue::Number(v.as_f64()?),
        ParamType::Bool => ParamValue::Bool(v.as_bool()?),
        ParamType::StrList => ParamValue::StrList(
            v.as_array()?
                .iter()
                .map(|s| s.as_str().map(String::from))
                .collect::<Option<_>>()?,
        ),
        ParamType::StrMap => ParamValue::StrMap(
            v.as_object()?
                .iter()
                .map(|(k, s)| s.as_str().map(|s| (k.clone(), s.to_string())))
                .collect::<Option<_>>()?,
        ),
    })
}

impl Process {
    /// Start the group's command with cwd at `root`; files are read relative to `data_root`.
    pub fn spawn(root: &Path, data_root: &Path, group: &Group) -> Result<Process, Failure> {
        let argv = group.command.as_deref().unwrap_or_default();
        let mut cmd = Command::new(&argv[0]);
        cmd.args(&argv[1..])
            .current_dir(root)
            .env("NECRO_PROTOCOL", PROTOCOL.to_string())
            .env("NECRO_ROOT", data_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd.spawn().map_err(|e| Failure {
            message: format!("can't start: {e}"),
            stderr: vec![],
        })?;
        let (tx, lines) = mpsc::channel();
        let stdout = child.stdout.take().expect("piped");
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(VecDeque::new()));
        let tail = Arc::clone(&stderr);
        let err = child.stderr.take().expect("piped");
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines() {
                let Ok(line) = line else { break };
                let mut t = tail.lock().expect("stderr lock");
                if t.len() == STDERR_TAIL {
                    t.pop_front();
                }
                t.push_back(line);
            }
        });
        Ok(Process {
            stdin: child.stdin.take(),
            child,
            lines,
            stderr,
            deadline: Instant::now() + Duration::from_secs_f64(group.timeout),
            done: false,
        })
    }

    fn fail(&mut self, message: impl Into<String>) -> Failure {
        self.kill();
        // Give the stderr reader a moment to drain what the script printed.
        std::thread::sleep(Duration::from_millis(20));
        Failure {
            message: message.into(),
            stderr: self
                .stderr
                .lock()
                .map(|t| t.iter().cloned().collect())
                .unwrap_or_default(),
        }
    }

    fn kill(&mut self) {
        if self.done {
            return;
        }
        self.done = true;
        #[cfg(unix)]
        // SAFETY: kill(2) on our own child's process group.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn exchange(&mut self, msg: &Json, close: bool) -> Result<Json, Failure> {
        let line = format!("{msg}\n");
        let wrote = self
            .stdin
            .as_mut()
            .map(|s| s.write_all(line.as_bytes()).and_then(|_| s.flush()));
        if close {
            self.stdin = None;
        }
        if let Some(Err(e)) = wrote {
            // The process may have exited; report what it says below if it can.
            let _ = e;
        }
        let wait = self.deadline.saturating_duration_since(Instant::now());
        let reply = match self.lines.recv_timeout(wait) {
            Ok(line) => line,
            Err(RecvTimeoutError::Timeout) => return Err(self.fail("timed out")),
            Err(RecvTimeoutError::Disconnected) => {
                let status = self.child.wait().ok();
                self.done = true;
                let what = match status.and_then(|s| s.code()) {
                    Some(code) if code != 0 => format!("exited with status {code}"),
                    _ => "closed stdout without answering".into(),
                };
                return Err(self.fail(what));
            }
        };
        let value: Json = serde_json::from_str(&reply)
            .map_err(|_| self.fail(format!("a line on stdout isn't JSON: {}", clip(&reply))))?;
        match value.get("protocol").and_then(Json::as_i64) {
            Some(PROTOCOL) => Ok(value),
            Some(p) => Err(self.fail(format!("it speaks protocol {p}; necro speaks {PROTOCOL}"))),
            None => Err(self.fail("its answer has no `protocol` field")),
        }
    }

    pub fn describe(&mut self) -> Result<BTreeMap<String, CheckDecl>, Failure> {
        let reply = self.exchange(&json!({"type": "describe", "protocol": PROTOCOL}), false)?;
        match parse_describe(&reply) {
            Ok(d) => Ok(d),
            Err(m) => Err(self.fail(format!("bad describe answer: {m}"))),
        }
    }

    pub fn check(mut self, jobs: &[Job]) -> Result<Response, Failure> {
        let jobs_json: Vec<Json> = jobs
            .iter()
            .map(|j| {
                let params: serde_json::Map<String, Json> = j
                    .params
                    .iter()
                    .map(|(k, v)| (k.clone(), param_json(v)))
                    .collect();
                json!({"id": j.id, "check": j.check, "files": j.files, "params": params})
            })
            .collect();
        let reply = self.exchange(
            &json!({"type": "check", "protocol": PROTOCOL, "jobs": jobs_json}),
            true,
        )?;
        let response = match parse_response(&reply) {
            Ok(r) => r,
            Err(m) => return Err(self.fail(format!("bad check answer: {m}"))),
        };
        // It must exit cleanly, within the same deadline.
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => {
                    self.done = true;
                    if !status.success() {
                        let code = status
                            .code()
                            .map_or("a signal".into(), |c| format!("status {c}"));
                        return Err(self.fail(format!("exited with {code}")));
                    }
                    return Ok(response);
                }
                Ok(None) if Instant::now() < self.deadline => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                _ => return Err(self.fail("timed out")),
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        self.kill();
    }
}

fn clip(s: &str) -> String {
    let short: String = s.chars().take(80).collect();
    if short.len() < s.len() {
        format!("{short}…")
    } else {
        short
    }
}

fn parse_describe(v: &Json) -> Result<BTreeMap<String, CheckDecl>, String> {
    let checks = v
        .get("checks")
        .and_then(Json::as_object)
        .ok_or("no `checks` object")?;
    let mut out = BTreeMap::new();
    for (name, c) in checks {
        let scope = match c.get("scope").and_then(Json::as_str) {
            Some("file") => Scope::File,
            Some("project") => Scope::Project,
            _ => return Err(format!("check `{name}`: scope is `file` or `project`")),
        };
        let mut params = BTreeMap::new();
        if let Some(p) = c.get("params") {
            let p = p
                .as_object()
                .ok_or(format!("check `{name}`: params is an object"))?;
            for (pname, d) in p {
                let ty = d.get("type").and_then(Json::as_str).and_then(ParamType::parse).ok_or(format!(
                    "check `{name}`: param `{pname}` needs a type: string, number, boolean, string[] or map<string>"
                ))?;
                let default = match d.get("default") {
                    None => None,
                    Some(v) => Some(param_from_json(ty, v).ok_or(format!(
                        "check `{name}`: param `{pname}`'s default doesn't match its type"
                    ))?),
                };
                params.insert(pname.clone(), ParamDecl { ty, default });
            }
        }
        let values = match c.get("values") {
            None => vec![],
            Some(v) => v
                .as_array()
                .and_then(|a| {
                    a.iter()
                        .map(|s| s.as_str().map(String::from))
                        .collect::<Option<Vec<_>>>()
                })
                .ok_or(format!("check `{name}`: values is a list of names"))?,
        };
        out.insert(
            name.clone(),
            CheckDecl {
                scope,
                params,
                values,
            },
        );
    }
    Ok(out)
}

/// A whole number, also when a script sends it as a float (`6.0`).
fn whole(v: &Json) -> Option<u64> {
    v.as_u64().or_else(|| {
        v.as_f64()
            .filter(|f| f.fract() == 0.0 && *f >= 0.0)
            .map(|f| f as u64)
    })
}

fn parse_response(v: &Json) -> Result<Response, String> {
    let mut out = Response::default();
    let results = v
        .get("results")
        .and_then(Json::as_object)
        .ok_or("no `results` object")?;
    for (id, r) in results {
        let hits = r
            .get("hits")
            .and_then(Json::as_array)
            .ok_or(format!("result `{id}` has no `hits` list"))?;
        let mut list = Vec::new();
        for h in hits {
            let file = h
                .get("file")
                .and_then(Json::as_str)
                .ok_or(format!("a hit in `{id}` has no `file`"))?;
            let line = h
                .get("line")
                .and_then(whole)
                .ok_or(format!("a hit in `{id}` has no `line`"))?;
            let col = h
                .get("column")
                .map_or(Some(1), whole)
                .ok_or(format!("a hit in `{id}` has a bad `column`"))?;
            let length = h
                .get("length")
                .map_or(Some(0), whole)
                .ok_or(format!("a hit in `{id}` has a bad `length`"))?;
            let mut values = BTreeMap::new();
            if let Some(vs) = h.get("values") {
                for (k, v) in vs
                    .as_object()
                    .ok_or(format!("a hit in `{id}` has bad `values`"))?
                {
                    let value = match v {
                        Json::String(s) => Value::Str(s.clone()),
                        Json::Number(n) => Value::Number(n.as_f64().unwrap_or(0.0)),
                        _ => return Err(format!("value `{k}` in `{id}` isn't a string or number")),
                    };
                    values.insert(k.clone(), value);
                }
            }
            list.push(ScriptHit {
                file: file.to_string(),
                line: line as usize,
                col: col.max(1) as usize,
                length: length as usize,
                values,
            });
        }
        out.results.insert(id.clone(), list);
    }
    let messages = |key: &str| -> Result<Vec<String>, String> {
        match v.get(key) {
            None => Ok(vec![]),
            Some(Json::Array(items)) => items
                .iter()
                .map(|i| {
                    let msg = i
                        .get("message")
                        .and_then(Json::as_str)
                        .ok_or(format!("an entry in `{key}` has no `message`"))?;
                    Ok(match i.get("job").and_then(Json::as_str) {
                        Some(job) => format!("job {job}: {msg}"),
                        None => msg.to_string(),
                    })
                })
                .collect(),
            Some(_) => Err(format!("`{key}` is a list")),
        }
    };
    out.warnings = messages("warnings")?;
    out.errors = messages("errors")?;
    Ok(out)
}
