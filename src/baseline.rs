//! `necro.baseline` (spec: The baseline): accepted hit counts per rule, path
//! and label, so a repo can adopt a rule and ratchet its old hits down.

use std::collections::BTreeMap;
use std::path::Path;

pub const FILE: &str = "necro.baseline";
const HEADER: &str = "# necro.baseline: hits accepted when adopted; rule, path, label, count. Rewrite with: necro baseline";

/// (rule, path, label) → count.
pub type Counts = BTreeMap<(String, String, String), usize>;

/// The baseline at `root`, empty when there's none.
pub fn read(root: &Path) -> Result<Counts, String> {
    let text = match std::fs::read_to_string(root.join(FILE)) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Counts::new()),
        Err(e) => return Err(format!("can't read {FILE}: {e}")),
    };
    let mut counts = Counts::new();
    for (i, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        let parsed = match fields.as_slice() {
            [rule, path, label @ .., count] if !label.is_empty() => count
                .parse::<usize>()
                .ok()
                .map(|n| ((rule.to_string(), path.to_string(), label.join("\t")), n)),
            _ => None,
        };
        let Some((key, n)) = parsed else {
            return Err(format!(
                "{FILE}:{}: expected rule, path, label and count, tab-separated",
                i + 1
            ));
        };
        *counts.entry(key).or_default() += n;
    }
    Ok(counts)
}

pub fn write(root: &Path, counts: &Counts) -> Result<(), String> {
    let mut text = format!("{HEADER}\n");
    for ((rule, path, label), n) in counts {
        text.push_str(&format!("{rule}\t{path}\t{label}\t{n}\n"));
    }
    std::fs::write(root.join(FILE), text).map_err(|e| format!("can't write {FILE}: {e}"))
}
