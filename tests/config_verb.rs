use std::path::PathBuf;
use std::process::Command;

/// Spawns the real binary with an isolated `HOME`/cwd per case, since `stk config`
/// resolves real env vars that would otherwise race against other tests in this binary.
fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-config-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(
    home: &std::path::Path,
    cwd: &std::path::Path,
    extra_args: &[&str],
    extra_env: &[(&str, &str)],
) -> (i32, String, String) {
    let mut command = Command::new(stk_bin());
    command
        .args(extra_args)
        .current_dir(cwd)
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", home);
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().unwrap();
    (
        output.status.code().unwrap(),
        String::from_utf8(output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}

#[test]
fn shows_built_in_defaults_when_no_config_exists_anywhere() {
    let home = temp_dir("defaults-home");
    let cwd = temp_dir("defaults-cwd");

    let (code, stdout, _) = run(&home, &cwd, &["config"], &[]);

    assert_eq!(code, 0);
    assert!(stdout.contains("no config file at this path yet"));
    assert!(stdout.contains("budget = 2000"));
}

#[test]
fn create_writes_the_global_file_and_is_idempotent() {
    let home = temp_dir("create-home");
    let cwd = temp_dir("create-cwd");
    let config_path = home.join(".config").join("stk").join("config.toml");

    let (first_code, first_stdout, _) = run(&home, &cwd, &["config", "--create"], &[]);
    assert_eq!(first_code, 0);
    assert!(first_stdout.contains("Created:"));
    assert!(config_path.exists());
    let contents_after_first = std::fs::read_to_string(&config_path).unwrap();

    let (second_code, second_stdout, _) = run(&home, &cwd, &["config", "--create"], &[]);
    assert_eq!(second_code, 0);
    assert!(second_stdout.contains("already exists"));
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        contents_after_first,
        "a second --create must not clobber the existing file"
    );
}

#[test]
fn project_config_overrides_global_config() {
    let home = temp_dir("project-over-global-home");
    let cwd = temp_dir("project-over-global-cwd");

    let global_dir = home.join(".config").join("stk");
    std::fs::create_dir_all(&global_dir).unwrap();
    std::fs::write(global_dir.join("config.toml"), "budget = 3000\n").unwrap();

    let project_dir = cwd.join(".stk");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(project_dir.join("config.toml"), "budget = 4000\n").unwrap();

    let (code, stdout, _) = run(&home, &cwd, &["config"], &[]);
    assert_eq!(code, 0);
    assert!(stdout.contains("budget = 4000"));
}

#[test]
fn env_var_overrides_project_config() {
    let home = temp_dir("env-over-project-home");
    let cwd = temp_dir("env-over-project-cwd");

    let project_dir = cwd.join(".stk");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(project_dir.join("config.toml"), "budget = 4000\n").unwrap();

    let (code, stdout, _) = run(&home, &cwd, &["config"], &[("STK_BUDGET", "5000")]);
    assert_eq!(code, 0);
    assert!(stdout.contains("budget = 5000"));
}

#[test]
fn project_only_config_is_shown_correctly_alongside_the_no_file_note() {
    let home = temp_dir("project-only-home");
    let cwd = temp_dir("project-only-cwd");

    let project_dir = cwd.join(".stk");
    std::fs::create_dir_all(&project_dir).unwrap();
    std::fs::write(project_dir.join("config.toml"), "budget = 4000\n").unwrap();

    let (code, stdout, _) = run(&home, &cwd, &["config"], &[]);
    assert_eq!(code, 0);
    // The note is about the *global* file specifically; it must never be worded as if
    // the displayed value (here, the project override) were just a built-in default.
    assert!(stdout.contains("no config file at this path yet"));
    assert!(stdout.contains("budget = 4000"));
}

#[test]
fn a_malformed_env_var_is_a_clear_error_not_silently_ignored() {
    let home = temp_dir("bad-env-home");
    let cwd = temp_dir("bad-env-cwd");

    let (code, _, stderr) = run(&home, &cwd, &["config"], &[("STK_BUDGET", "not-a-number")]);
    assert_ne!(code, 0);
    assert!(stderr.contains("STK_BUDGET"));
}

#[test]
fn an_unrecognized_argument_is_an_error() {
    let home = temp_dir("bad-arg-home");
    let cwd = temp_dir("bad-arg-cwd");

    let (code, _, stderr) = run(&home, &cwd, &["config", "--bogus"], &[]);
    assert_ne!(code, 0);
    assert!(stderr.contains("unrecognized argument"));
}

#[test]
fn cli_flag_overrides_env_var() {
    let home = temp_dir("cli-over-env-home");
    let cwd = temp_dir("cli-over-env-cwd");

    let (code, stdout, _) = run(
        &home,
        &cwd,
        &["--budget", "6000", "config"],
        &[("STK_BUDGET", "5000")],
    );
    assert_eq!(code, 0);
    assert!(stdout.contains("budget = 6000"));
}
