//! The conformance suite (tests/conformance/README.md) against this build's `necro`.

use std::process::Command;

#[test]
fn conformance() {
    let out = Command::new("uv")
        .args([
            "run",
            "--script",
            "tests/conformance/run.py",
            "--engine-version",
            env!("CARGO_PKG_VERSION"),
            "--",
            env!("CARGO_BIN_EXE_necro"),
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("uv runs the conformance suite");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "{text}{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
