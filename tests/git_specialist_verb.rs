mod support;

use std::path::{Path, PathBuf};
use std::process::Command;

use support::rtk;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn temp_repo_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-git-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-q"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test"]);
}

/// A repo with a file modified in two separate places (a multi-hunk diff) plus a rename.
fn multi_hunk_diff_repo() -> PathBuf {
    let dir = temp_repo_dir("multi-hunk");
    init_repo(&dir);
    std::fs::write(
        dir.join("foo.rs"),
        "fn foo() {\n    let a = 1;\n    let b = 2;\n}\n\nfn bar() {\n    println!(\"bar\");\n}\n",
    )
    .unwrap();
    std::fs::write(dir.join("old_name.rs"), "fn old() {}\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "initial"]);

    std::fs::write(
        dir.join("foo.rs"),
        "fn foo() {\n    let a = 999;\n    let b = 2;\n}\n\nfn bar() {\n    println!(\"bar changed\");\n}\n",
    )
    .unwrap();
    std::fs::rename(dir.join("old_name.rs"), dir.join("renamed_name.rs")).unwrap();
    git(&dir, &["add", "-A"]);
    dir
}

/// A repo with a real, currently-unresolved merge conflict.
fn merge_conflict_repo() -> PathBuf {
    let dir = temp_repo_dir("conflict");
    init_repo(&dir);
    std::fs::write(dir.join("shared.rs"), "line one\nline two\nline three\n").unwrap();
    git(&dir, &["add", "-A"]);
    git(&dir, &["commit", "-q", "-m", "initial"]);

    git(&dir, &["checkout", "-q", "-b", "feature"]);
    std::fs::write(
        dir.join("shared.rs"),
        "line one\nfeature change\nline three\n",
    )
    .unwrap();
    git(&dir, &["commit", "-q", "-am", "feature change"]);

    git(&dir, &["checkout", "-q", "master"]);
    std::fs::write(
        dir.join("shared.rs"),
        "line one\nmaster change\nline three\n",
    )
    .unwrap();
    git(&dir, &["commit", "-q", "-am", "master change"]);

    git(&dir, &["merge", "feature"]); // conflicts; left unresolved on purpose
    dir
}

#[test]
fn multi_hunk_diff_preserves_both_hunks_and_the_rename() {
    let dir = multi_hunk_diff_repo();

    let output = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["git", "diff", "--staged"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(output.status.success());
    assert!(
        stdout.contains("let a = 999"),
        "first hunk missing: {stdout:?}"
    );
    assert!(
        stdout.contains("bar changed"),
        "second hunk missing: {stdout:?}"
    );
    assert!(
        stdout.contains("rename from old_name.rs") && stdout.contains("rename to renamed_name.rs"),
        "rename info missing (treated as delete+add instead?): {stdout:?}"
    );
}

#[test]
fn multi_hunk_diff_under_budget_keeps_hunks_associated_with_their_file() {
    let dir = multi_hunk_diff_repo();

    let output = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["--budget", "150", "git", "diff", "--staged", "--", "foo.rs"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("diff --git a/foo.rs b/foo.rs"));
    assert!(stdout.contains("let a = 999"));
    assert!(stdout.contains("bar changed"));
}

#[test]
fn merge_conflict_status_preserves_unmerged_path_info() {
    let dir = merge_conflict_repo();

    let output = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["git", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(stdout.contains("Unmerged paths"));
    assert!(stdout.contains("both modified"));
    assert!(stdout.contains("shared.rs"));
}

#[test]
fn merge_conflict_status_under_budget_still_shows_the_conflicted_path() {
    let dir = merge_conflict_repo();

    let output = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["--budget", "80", "git", "status"])
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    assert!(
        stdout.contains("shared.rs"),
        "expected the conflicted file to survive under budget, got: {stdout:?}"
    );
}

#[test]
fn golden_comparison_diff_retains_the_rename_info_real_rtk_does() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let dir = multi_hunk_diff_repo();

    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["git", "diff", "--staged"])
        .output()
        .unwrap();
    let via_stk = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["git", "diff", "--staged"])
        .output()
        .unwrap();

    let rtk_stdout = String::from_utf8_lossy(&via_rtk.stdout);
    let stk_stdout = String::from_utf8_lossy(&via_stk.stdout);

    assert!(
        rtk_stdout.contains("renamed_name.rs"),
        "sanity check: rtk itself should mention the renamed file"
    );
    assert!(stk_stdout.contains("renamed_name.rs"));
    assert!(stk_stdout.contains("let a = 999"));
}

#[test]
fn golden_comparison_status_retains_the_same_files_real_rtk_does() {
    if !rtk::is_available() {
        eprintln!("skipping: rtk is not installed in this environment");
        return;
    }
    let dir = multi_hunk_diff_repo();

    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["git", "status"])
        .output()
        .unwrap();
    let via_stk = Command::new(stk_bin())
        .current_dir(&dir)
        .args(["git", "status"])
        .output()
        .unwrap();

    let rtk_stdout = String::from_utf8_lossy(&via_rtk.stdout);
    let stk_stdout = String::from_utf8_lossy(&via_stk.stdout);

    assert!(rtk_stdout.contains("foo.rs"));
    assert!(stk_stdout.contains("foo.rs"));
    assert!(stk_stdout.contains("renamed_name.rs"));
}
