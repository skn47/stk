use std::path::PathBuf;
use std::process::Command;

/// Spawns the real binary with an isolated `HOME`/cwd per case, since `stk init` writes
/// to real, `HOME`/cwd-relative paths that would otherwise race against other tests.
fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-init-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(home: &std::path::Path, cwd: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(stk_bin())
        .args(args)
        .current_dir(cwd)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", home)
        .output()
        .unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn first_run_creates_claude_md_second_run_reports_no_changes() {
    let home = temp_dir("basic-home");
    let cwd = temp_dir("basic-cwd");

    let (first_code, first_stdout, _) = run(&home, &cwd, &["init"]);
    assert_eq!(first_code, 0);
    assert!(first_stdout.contains("Created: CLAUDE.md"));
    assert!(cwd.join("CLAUDE.md").exists());

    let (second_code, second_stdout, _) = run(&home, &cwd, &["init"]);
    assert_eq!(second_code, 0);
    assert!(second_stdout.contains("No changes needed"));
}

#[test]
fn init_appends_to_an_existing_claude_md_without_touching_prior_content() {
    let home = temp_dir("append-home");
    let cwd = temp_dir("append-cwd");
    std::fs::write(cwd.join("CLAUDE.md"), "# My Project\n\nExisting notes.\n").unwrap();

    let (code, stdout, _) = run(&home, &cwd, &["init"]);
    assert_eq!(code, 0);
    assert!(stdout.contains("Updated: CLAUDE.md"));

    let contents = std::fs::read_to_string(cwd.join("CLAUDE.md")).unwrap();
    assert!(contents.starts_with("# My Project\n\nExisting notes.\n"));
    assert!(contents.contains("<!-- stk-instructions v1 -->"));

    let (second_code, second_stdout, _) = run(&home, &cwd, &["init"]);
    assert_eq!(second_code, 0);
    assert!(second_stdout.contains("No changes needed"));
}

#[test]
fn a_malformed_orphaned_marker_is_refused_rather_than_corrupting_the_file() {
    let home = temp_dir("malformed-home");
    let cwd = temp_dir("malformed-cwd");
    let original = "<!-- stk-instructions v1 -->\nstray leftover text, no end marker\n";
    std::fs::write(cwd.join("CLAUDE.md"), original).unwrap();

    let (code, _, stderr) = run(&home, &cwd, &["init"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("malformed"));
    assert_eq!(
        std::fs::read_to_string(cwd.join("CLAUDE.md")).unwrap(),
        original,
        "a refused init must never touch the file"
    );
}

#[test]
fn global_targets_a_different_location_than_the_default() {
    let home = temp_dir("global-home");
    let cwd = temp_dir("global-cwd");

    let (code, stdout, _) = run(&home, &cwd, &["init", "--global"]);
    assert_eq!(code, 0);
    let global_path = home.join(".claude").join("CLAUDE.md");
    assert!(stdout.contains(&global_path.display().to_string()));
    assert!(global_path.exists());
    assert!(!cwd.join("CLAUDE.md").exists());
}

#[test]
fn codex_writes_project_config_idempotently() {
    let home = temp_dir("codex-home");
    let cwd = temp_dir("codex-cwd");

    let (first_code, first_stdout, _) = run(&home, &cwd, &["init", "--codex"]);
    assert_eq!(first_code, 0);
    assert!(first_stdout.contains("Created:"));
    let config_path = cwd.join(".stk").join("config.toml");
    assert!(config_path.exists());
    assert!(!cwd.join("CLAUDE.md").exists());

    let (second_code, second_stdout, _) = run(&home, &cwd, &["init", "--codex"]);
    assert_eq!(second_code, 0);
    assert!(second_stdout.contains("No changes needed"));
}

#[test]
fn codex_and_global_together_is_a_clear_error() {
    let home = temp_dir("codex-global-home");
    let cwd = temp_dir("codex-global-cwd");

    let (code, _, stderr) = run(&home, &cwd, &["init", "--codex", "--global"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("cannot be combined"));
}

#[test]
fn an_unrecognized_argument_is_an_error() {
    let home = temp_dir("bad-arg-home");
    let cwd = temp_dir("bad-arg-cwd");

    let (code, _, stderr) = run(&home, &cwd, &["init", "--bogus"]);
    assert_ne!(code, 0);
    assert!(stderr.contains("unrecognized argument"));
}
