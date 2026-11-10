//! The option parser the CLI and finders share (spec: Finders).
//!
//! A command declares the options it takes and gets back the words that aren't
//! options, in order, plus any pathspecs after `--`. Options may come anywhere
//! before `--`; the last value given wins for [`Args::value`], while
//! [`Args::values`] keeps them all.

use std::collections::HashSet;

/// The options a command takes.
pub struct Spec<'a> {
    /// Options that take a value (`"--files-from"`, `"-C"`).
    pub values: &'a [&'a str],
    /// Boolean options (`"--write"`).
    pub flags: &'a [&'a str],
    /// Whether a bare `--` ends the options and the rest are pathspecs.
    pub pathspecs: bool,
}

/// Why parsing stopped.
#[derive(Debug, PartialEq, Eq)]
pub enum ArgError {
    /// An option that takes a value was given without one.
    MissingValue(String),
    /// A word starting with `-` that the command doesn't declare.
    UnknownOption(String),
}

impl ArgError {
    /// The error with the command's usage line appended, as finders word it.
    pub fn with_usage(&self, usage: &str) -> String {
        match self {
            ArgError::MissingValue(name) => format!("{name} needs a value: {usage}"),
            ArgError::UnknownOption(word) => format!("unknown option {word}: {usage}"),
        }
    }
}

/// What a parsed command line holds.
#[derive(Debug, PartialEq, Eq)]
pub struct Args {
    /// The words that aren't options, in order.
    pub positional: Vec<String>,
    /// The words after `--`, in order (empty unless the spec has pathspecs).
    pub pathspecs: Vec<String>,
    values: Vec<(String, String)>,
    flags: HashSet<String>,
}

impl Args {
    /// The last value given for `name`.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every value given for `name`, in order.
    pub fn values(&self, name: &str) -> Vec<&str> {
        self.values
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, value)| value.as_str())
            .collect()
    }

    /// Whether `name` was given.
    pub fn flag(&self, name: &str) -> bool {
        self.flags.contains(name)
    }
}

/// Parse `args` against `spec`: a `--long` value option takes `--name=value` or
/// `--name value`, a short one (`-C`) only the next word, and a word starting
/// with `-` (longer than `-`) that isn't declared is an error.
pub fn parse(args: &[String], spec: &Spec) -> Result<Args, ArgError> {
    let mut out = Args {
        positional: Vec::new(),
        pathspecs: Vec::new(),
        values: Vec::new(),
        flags: HashSet::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--" && spec.pathspecs {
            out.pathspecs.extend(it.cloned());
            break;
        }
        if let Some(name) = spec.values.iter().find(|name| **name == arg.as_str()) {
            let value = match it.next() {
                Some(value) if !(spec.pathspecs && value == "--") => value,
                _ => return Err(ArgError::MissingValue(name.to_string())),
            };
            out.values.push((name.to_string(), value.clone()));
            continue;
        }
        if arg.starts_with("--")
            && let Some((name, value)) = arg.split_once('=')
            && spec.values.contains(&name)
        {
            out.values.push((name.to_string(), value.to_string()));
            continue;
        }
        if spec.flags.contains(&arg.as_str()) {
            out.flags.insert(arg.clone());
            continue;
        }
        if arg.starts_with('-') && arg.len() > 1 {
            return Err(ArgError::UnknownOption(arg.clone()));
        }
        out.positional.push(arg.clone());
    }
    Ok(out)
}

/// The one check word a finder takes: the first positional, which must name a
/// check in `checks`, and nothing after it.
pub fn check<'a>(
    positional: &'a [String],
    checks: &[&str],
    usage: &str,
) -> Result<&'a str, String> {
    let (first, rest) = positional
        .split_first()
        .ok_or_else(|| format!("name a check: {usage}"))?;
    if let Some(extra) = rest.first() {
        return Err(format!("unexpected argument {extra}: {usage}"));
    }
    if !checks.contains(&first.as_str()) {
        return Err(format!("unknown check {first}: {usage}"));
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    const USAGE: &str = "necro test [--glob <glob>] [--write] [-- <pathspec>…]";

    const SPEC: Spec = Spec {
        values: &["--glob", "-C"],
        flags: &["--write"],
        pathspecs: true,
    };

    fn parsed(words: &[&str]) -> Result<Args, ArgError> {
        let words: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        super::parse(&words, &SPEC)
    }

    fn checked(words: &[&str]) -> Result<String, String> {
        let words: Vec<String> = words.iter().map(|w| w.to_string()).collect();
        super::check(&words, &["spacing", "uid"], USAGE).map(str::to_string)
    }

    #[test]
    fn long_value_takes_equals_or_next_word() {
        let args = parsed(&["--glob=*.rs", "--glob", "*.gd", "x"]).unwrap();
        assert_eq!(args.values("--glob"), ["*.rs", "*.gd"]);
        assert_eq!(args.positional, ["x"]);
    }

    #[test]
    fn short_value_takes_only_the_next_word() {
        let args = parsed(&["-C", "dir", "x"]).unwrap();
        assert_eq!(args.value("-C"), Some("dir"));
        assert_eq!(args.positional, ["x"]);
        assert_eq!(
            parsed(&["-C=dir"]),
            Err(ArgError::UnknownOption("-C=dir".to_string()))
        );
    }

    #[test]
    fn missing_value_names_the_option() {
        let error = parsed(&["--glob"]).unwrap_err();
        assert_eq!(error, ArgError::MissingValue("--glob".to_string()));
        assert_eq!(
            error.with_usage(USAGE),
            "--glob needs a value: necro test [--glob <glob>] [--write] [-- <pathspec>…]"
        );
        assert_eq!(
            parsed(&["-C"]),
            Err(ArgError::MissingValue("-C".to_string()))
        );
        // `--` ends the options, so it can't be the value.
        assert_eq!(
            parsed(&["--glob", "--", "x"]),
            Err(ArgError::MissingValue("--glob".to_string()))
        );
    }

    #[test]
    fn unknown_option_names_the_word() {
        let error = parsed(&["--nope"]).unwrap_err();
        assert_eq!(error, ArgError::UnknownOption("--nope".to_string()));
        assert_eq!(
            error.with_usage(USAGE),
            "unknown option --nope: necro test [--glob <glob>] [--write] [-- <pathspec>…]"
        );
        assert_eq!(
            parsed(&["-x"]),
            Err(ArgError::UnknownOption("-x".to_string()))
        );
    }

    #[test]
    fn double_dash_splits_pathspecs() {
        let args = parsed(&["a", "--", "-b", "--glob"]).unwrap();
        assert_eq!(args.positional, ["a"]);
        assert_eq!(args.pathspecs, ["-b", "--glob"]);
        // A lone `-` is a positional, not an option.
        assert_eq!(parsed(&["-"]).unwrap().positional, ["-"]);
    }

    #[test]
    fn repeated_values_keep_their_order() {
        let args = parsed(&["--glob", "a", "--glob=b", "--write", "--glob", "c"]).unwrap();
        assert_eq!(args.values("--glob"), ["a", "b", "c"]);
        assert_eq!(args.value("--glob"), Some("c"));
        assert!(args.flag("--write"));
        assert!(!args.flag("--glob"));
    }

    #[test]
    fn check_names_the_one_word() {
        assert_eq!(checked(&["spacing"]), Ok("spacing".to_string()));
        assert_eq!(checked(&[]), Err(format!("name a check: {USAGE}")));
        assert_eq!(
            checked(&["nope"]),
            Err(format!("unknown check nope: {USAGE}"))
        );
        assert_eq!(
            checked(&["spacing", "extra"]),
            Err(format!("unexpected argument extra: {USAGE}"))
        );
    }
}
