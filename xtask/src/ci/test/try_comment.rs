//! Tests for the `/ci try` conclusion comment created by `.github/workflows/try.jq`.
//!
//! Set `UPDATE_EXPECT=1` to update the `try_comment/expected*.md` files.

use std::path::Path;
use std::process::Command;

use crate::util::get_cargo_workspace;

#[track_caller]
fn check(jobs: &str, annotations: Option<&str>, try_error: &str, expected: &str) {
    let workspace = get_cargo_workspace();
    let dir = workspace.join("xtask/src/ci/test/try_comment");
    let annotations = annotations.map_or_else(
        || "{}".to_owned(),
        |a| std::fs::read_to_string(dir.join(a)).unwrap(),
    );
    let output = Command::new("jq")
        .args(["-r", "-f"])
        .arg(workspace.join(".github/workflows/try.jq"))
        .args(["--arg", "pr_id", "1337"])
        .args([
            "--arg",
            "comment",
            "https://github.com/cross-rs/cross/pull/1337#issuecomment-1",
        ])
        .args(["--arg", "try_error", try_error])
        .args(["--argjson", "annotations", &annotations])
        .arg(dir.join(jobs))
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
    compare(&dir.join(expected), &actual);
}

#[track_caller]
fn compare(expected_path: &Path, actual: &str) {
    if std::env::var_os("UPDATE_EXPECT").is_some() {
        std::fs::write(expected_path, actual).unwrap();
    }
    let expected = std::fs::read_to_string(expected_path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        actual == expected,
        "try comment changed, rerun with UPDATE_EXPECT=1 if expected:\n{actual}"
    );
}

#[test]
fn try_comment() {
    check("jobs.json", Some("annotations.json"), "", "expected.md");
}

#[test]
fn try_comment_invalid_command() {
    let err = eyre::eyre!("no targets matched `i686-linux`");
    let try_error = super::super::target_matrix::try_error(&err).to_string();
    check("jobs-invalid.json", None, &try_error, "expected-invalid.md");
}
