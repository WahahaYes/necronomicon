//! The skills necro carries for coding agents (spec: Skills), and
//! `necro skills install`, which writes them stamped with this version.

use std::path::Path;

/// The skills necro carries: (name, SKILL.md source).
const SKILLS: &[(&str, &str)] = &[
    ("necro", include_str!("../.agents/skills/necro/SKILL.md")),
    (
        "necro-rules",
        include_str!("../.agents/skills/necro-rules/SKILL.md"),
    ),
];

const USAGE: &str = "necro skills install";

/// A carried skill's text with `metadata: necro-version:` closing its
/// frontmatter, or `None` when necro doesn't carry `name`.
pub fn stamped(name: &str) -> Option<String> {
    let (_, source) = SKILLS.iter().find(|(n, _)| *n == name)?;
    let body = source.strip_prefix("---\n")?;
    let end = body.find("\n---\n")?;
    Some(format!(
        "---\n{}\nmetadata:\n  necro-version: \"{}\"\n---\n{}",
        &body[..end],
        env!("CARGO_PKG_VERSION"),
        &body[end + "\n---\n".len()..]
    ))
}

/// `necro skills <action>`: the lines to print, or a usage error.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    match args {
        [] => Err(format!("name an action: {USAGE}")),
        [a] if a == "install" => install(),
        _ => Err(format!("unknown action {}: {USAGE}", args.join(" "))),
    }
}

/// Write every carried skill into `.agents/skills` at the work tree's top and
/// link each from `.claude/skills`. Checks first, so a refusal writes nothing.
fn install() -> Result<Vec<String>, String> {
    let out = crate::files::git()
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git work tree".into());
    }
    let top = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let top = Path::new(&top);
    for (name, _) in SKILLS {
        let link = format!(".claude/skills/{name}");
        if std::fs::symlink_metadata(top.join(&link)).is_ok_and(|m| m.is_dir()) {
            return Err(format!(
                "{link} is a folder, not a link; move it to .agents/skills/{name} or remove it, then run again"
            ));
        }
    }
    let mut lines = Vec::new();
    for (name, _) in SKILLS {
        let text = stamped(name).ok_or(format!("the {name} skill has no frontmatter"))?;
        let folder = format!(".agents/skills/{name}");
        let file = top.join(&folder).join("SKILL.md");
        std::fs::create_dir_all(top.join(&folder))
            .map_err(|e| format!("can't create {folder}: {e}"))?;
        std::fs::write(&file, text).map_err(|e| format!("can't write {folder}/SKILL.md: {e}"))?;
        let link = top.join(format!(".claude/skills/{name}"));
        if std::fs::symlink_metadata(&link).is_ok() {
            std::fs::remove_file(&link)
                .map_err(|e| format!("can't replace .claude/skills/{name}: {e}"))?;
        }
        std::fs::create_dir_all(top.join(".claude/skills"))
            .map_err(|e| format!("can't create .claude/skills: {e}"))?;
        crate::files::symlink(&format!("../../.agents/skills/{name}"), &link)?;
        lines.push(format!(
            "necro: installed {folder} (necro {})",
            env!("CARGO_PKG_VERSION")
        ));
    }
    Ok(lines)
}
