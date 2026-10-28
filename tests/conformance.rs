//! The conformance suite (conformance/README.md) against this build's `necro`.

use std::process::Command;

#[test]
fn conformance() {
    let out = Command::new("python3")
        .args(["conformance/run.py", "--", env!("CARGO_BIN_EXE_necro")])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("python3 runs the conformance suite");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
