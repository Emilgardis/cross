//! Tests for the `/ci try` conclusion comment created by `.github/workflows/try.jq`.
//!
//! Set `UPDATE_EXPECT=1` to update `try_comment/expected.md`.

use std::process::Command;

use crate::util::get_cargo_workspace;

#[test]
fn try_comment() {
    let workspace = get_cargo_workspace();
    let dir = workspace.join("xtask/src/ci/test/try_comment");
    let output = Command::new("jq")
        .args(["-r", "-f"])
        .arg(workspace.join(".github/workflows/try.jq"))
        .args(["--arg", "pr_id", "1337"])
        .args([
            "--arg",
            "comment",
            "https://github.com/cross-rs/cross/pull/1337#issuecomment-1",
        ])
        .arg(dir.join("jobs.json"))
        .output()
        .expect("jq should be installed");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = String::from_utf8(output.stdout)
        .unwrap()
        .replace("\r\n", "\n");

    let expected_path = dir.join("expected.md");
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(&expected_path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(&expected_path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        actual == expected,
        "try comment changed, rerun with UPDATE_EXPECT=1 if expected:\n{actual}"
    );
}
