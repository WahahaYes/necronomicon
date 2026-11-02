//! Packs (spec: Packs): rule sets built into necro and versioned with it,
//! included as `necro:<pack>`. Their commands use only finders.

/// Every pack: its name and its rules, a YAML file holding only `rules:`.
const PACKS: &[(&str, &str)] = &[("rust", include_str!("rust.yaml"))];

/// The pack named `name`, as (name, YAML).
pub fn find(name: &str) -> Option<(&'static str, &'static str)> {
    PACKS.iter().find(|(n, _)| *n == name).copied()
}

/// The pack names, for unknown-pack errors.
pub fn names() -> Vec<&'static str> {
    PACKS.iter().map(|(n, _)| *n).collect()
}
