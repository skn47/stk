use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

/// The `rtk` version every golden comparison in this codebase was last verified
/// against (see the spec's Further Notes). A different version doesn't fail the run --
/// it's flagged so a maintainer knows to re-verify before trusting a `PASS`.
const PINNED_RTK_VERSION: &str = "0.42.4";

type Case = (&'static str, fn(&Path) -> CaseOutcome);

enum CaseOutcome {
    Pass,
    Fail(String),
    /// Not a failure of `stk` -- the real `rtk` itself didn't exhibit the behavior
    /// being checked (e.g. a fixture directory is missing), so there's nothing to
    /// meaningfully compare against.
    Skipped(String),
}

/// `stk bench --against rtk`: runs the golden comparisons for every aliased verb as one
/// command, reporting pass/fail/skip per case and exiting nonzero if anything regressed
/// -- usable as a CI gate, not just a developer's local check.
pub fn dispatch(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if args != ["--against".to_string(), "rtk".to_string()] {
        let _ = writeln!(stderr, "stk: bench: usage: stk bench --against rtk");
        return 2;
    }

    if !crate::rtk::is_available() {
        let _ = writeln!(
            stdout,
            "rtk is not installed in this environment; nothing to bench against."
        );
        return 0;
    }

    report_rtk_version(stdout);

    let self_exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(err) => {
            let _ = writeln!(
                stderr,
                "stk: bench: could not determine stk's own executable path: {err}"
            );
            return 1;
        }
    };

    let cases: Vec<Case> = vec![
        ("run/echo", case_run_echo),
        ("proxy/echo", case_proxy_echo),
        ("pipe/passthrough", case_pipe_passthrough),
        ("pipe/default", case_pipe_default),
        ("cargo/type_error", case_cargo_type_error),
        ("git/diff_rename", case_git_diff_rename),
        ("git/status", case_git_status),
        ("init/plain", case_init_plain),
        ("config/plain", case_config_plain),
        ("gain/plain", case_gain_plain),
    ];

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let _ = writeln!(stdout);
    for (name, case) in cases {
        match case(&self_exe) {
            CaseOutcome::Pass => {
                passed += 1;
                let _ = writeln!(stdout, "[PASS] {name}");
            }
            CaseOutcome::Fail(detail) => {
                failed += 1;
                let _ = writeln!(stdout, "[FAIL] {name}: {detail}");
            }
            CaseOutcome::Skipped(detail) => {
                skipped += 1;
                let _ = writeln!(stdout, "[SKIP] {name}: {detail}");
            }
        }
    }
    let _ = writeln!(
        stdout,
        "\n{passed} passed, {failed} failed, {skipped} skipped"
    );

    if failed > 0 {
        1
    } else {
        0
    }
}

fn detected_rtk_version() -> Option<String> {
    let output = Command::new("rtk").arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .last()
        .map(str::to_string)
}

fn version_report_line(detected: Option<&str>) -> String {
    match detected {
        Some(version) if version == PINNED_RTK_VERSION => {
            format!("rtk version: {version} (matches pinned {PINNED_RTK_VERSION})")
        }
        Some(version) => format!(
            "rtk version: {version} (WARNING: golden comparisons were last verified against {PINNED_RTK_VERSION} -- re-verify before trusting a PASS)"
        ),
        None => "rtk version: unknown (`rtk --version` failed)".to_string(),
    }
}

fn report_rtk_version(stdout: &mut dyn Write) {
    let _ = writeln!(
        stdout,
        "{}",
        version_report_line(detected_rtk_version().as_deref())
    );
}

fn combined_output(output: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-bench-{name}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

fn run_piped(program: &Path, args: &[&str], input: &[u8]) -> Result<std::process::Output, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to spawn {}: {err}", program.display()))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "child had no stdin handle".to_string())?;
    stdin
        .write_all(input)
        .map_err(|err| format!("failed to write stdin: {err}"))?;
    drop(stdin);
    child
        .wait_with_output()
        .map_err(|err| format!("failed to wait for child: {err}"))
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Reads a fixture TOML file's `command` array via `toml::Table` rather than the
/// derived `Fixture` struct `tests/support/fixture.rs` uses for `cargo test` -- keeps
/// this production binary's dependency tree free of serde's derive machinery, while
/// still reading the real fixture corpus instead of retyping its data by hand.
fn fixture_command(path: &Path) -> Result<Vec<String>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let table: toml::Table = toml::from_str(&text)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    Ok(table
        .get("command")
        .and_then(|v| v.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default())
}

fn case_run_echo(self_exe: &Path) -> CaseOutcome {
    let fixture = fixture_path("echo_hello.toml");
    let command = match fixture_command(&fixture) {
        Ok(command) if !command.is_empty() => command,
        Ok(_) => {
            return CaseOutcome::Skipped(format!("fixture has no command: {}", fixture.display()))
        }
        Err(err) => return CaseOutcome::Skipped(err),
    };
    let mut args = vec!["run".to_string()];
    args.extend(command);

    let via_rtk = match Command::new("rtk").args(&args).output() {
        Ok(output) => output,
        Err(err) => return CaseOutcome::Fail(format!("failed to run rtk: {err}")),
    };
    let via_stk = match Command::new(self_exe).args(&args).output() {
        Ok(output) => output,
        Err(err) => return CaseOutcome::Fail(format!("failed to run stk: {err}")),
    };
    exact_match(&via_rtk, &via_stk)
}

fn case_proxy_echo(self_exe: &Path) -> CaseOutcome {
    let fixture = fixture_path("echo_hello.toml");
    let command = match fixture_command(&fixture) {
        Ok(command) if !command.is_empty() => command,
        Ok(_) => {
            return CaseOutcome::Skipped(format!("fixture has no command: {}", fixture.display()))
        }
        Err(err) => return CaseOutcome::Skipped(err),
    };
    let mut args = vec!["proxy".to_string()];
    args.extend(command);

    let via_rtk = Command::new("rtk")
        .args(&args)
        .env("XDG_CACHE_HOME", unique_temp_dir("proxy-rtk-cache"))
        .output();
    let via_stk = Command::new(self_exe)
        .args(&args)
        .env("XDG_CACHE_HOME", unique_temp_dir("proxy-stk-cache"))
        .output();
    match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => exact_match(&rtk, &stk),
        (Err(err), _) | (_, Err(err)) => CaseOutcome::Fail(format!("failed to spawn: {err}")),
    }
}

fn case_pipe_passthrough(self_exe: &Path) -> CaseOutcome {
    let payload = b"\x1b[31mred\x1b[0m\nplain line\n";
    let via_rtk = run_piped(Path::new("rtk"), &["pipe", "--passthrough"], payload);
    let via_stk = run_piped(self_exe, &["pipe", "--passthrough"], payload);
    match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => exact_match(&rtk, &stk),
        (Err(err), _) | (_, Err(err)) => CaseOutcome::Fail(err),
    }
}

fn case_pipe_default(self_exe: &Path) -> CaseOutcome {
    let payload = b"plain output, no flags given\n";
    let via_rtk = run_piped(Path::new("rtk"), &["pipe"], payload);
    let via_stk = run_piped(self_exe, &["pipe"], payload);
    match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => exact_match(&rtk, &stk),
        (Err(err), _) | (_, Err(err)) => CaseOutcome::Fail(err),
    }
}

fn exact_match(via_rtk: &std::process::Output, via_stk: &std::process::Output) -> CaseOutcome {
    if via_stk.stdout == via_rtk.stdout && via_stk.status.code() == via_rtk.status.code() {
        CaseOutcome::Pass
    } else {
        CaseOutcome::Fail(format!(
            "stdout or exit code differs from rtk (rtk exit {:?}, stk exit {:?})",
            via_rtk.status.code(),
            via_stk.status.code()
        ))
    }
}

fn cargo_fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cargo_projects")
        .join(name)
}

fn case_cargo_type_error(self_exe: &Path) -> CaseOutcome {
    let dir = cargo_fixture_dir("type_error");
    if !dir.exists() {
        return CaseOutcome::Skipped(format!("fixture directory not found: {}", dir.display()));
    }

    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["cargo", "check"])
        .output();
    let via_stk = Command::new(self_exe)
        .current_dir(&dir)
        .args(["cargo", "check"])
        .output();
    let (via_rtk, via_stk) = match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => (rtk, stk),
        (Err(err), _) | (_, Err(err)) => {
            return CaseOutcome::Fail(format!("failed to spawn: {err}"))
        }
    };

    let rtk_combined = combined_output(&via_rtk);
    if !rtk_combined.contains("E0308") {
        return CaseOutcome::Skipped(
            "rtk itself doesn't show the expected error code here -- inconclusive".to_string(),
        );
    }
    let stk_combined = combined_output(&via_stk);
    let missing: Vec<&str> = ["E0308", "src/main.rs:6:30"]
        .into_iter()
        .filter(|evidence| !stk_combined.contains(evidence))
        .collect();
    if missing.is_empty() {
        CaseOutcome::Pass
    } else {
        CaseOutcome::Fail(format!("stk dropped required evidence: {missing:?}"))
    }
}

fn git(dir: &Path, args: &[&str]) -> std::io::Result<std::process::Output> {
    Command::new("git").current_dir(dir).args(args).output()
}

/// A repo with a file modified in two separate places (a multi-hunk diff) plus a
/// rename, staged but not committed -- mirrors `tests/git_specialist_verb.rs`'s own
/// fixture, since git repos with real history can't be safely checked in as fixtures.
fn multi_hunk_diff_repo() -> Result<PathBuf, String> {
    let dir = unique_temp_dir("git-multi-hunk");
    let steps: Result<(), std::io::Error> = (|| {
        git(&dir, &["init", "-q"])?;
        git(&dir, &["config", "user.email", "bench@stk.test"])?;
        git(&dir, &["config", "user.name", "stk bench"])?;
        std::fs::write(
            dir.join("foo.rs"),
            "fn foo() {\n    let a = 1;\n    let b = 2;\n}\n\nfn bar() {\n    println!(\"bar\");\n}\n",
        )?;
        std::fs::write(dir.join("old_name.rs"), "fn old() {}\n")?;
        git(&dir, &["add", "-A"])?;
        git(&dir, &["commit", "-q", "-m", "initial"])?;
        std::fs::write(
            dir.join("foo.rs"),
            "fn foo() {\n    let a = 999;\n    let b = 2;\n}\n\nfn bar() {\n    println!(\"bar changed\");\n}\n",
        )?;
        std::fs::rename(dir.join("old_name.rs"), dir.join("renamed_name.rs"))?;
        git(&dir, &["add", "-A"])?;
        Ok(())
    })();
    steps.map_err(|err| format!("failed to build fixture repo: {err}"))?;
    Ok(dir)
}

fn case_git_diff_rename(self_exe: &Path) -> CaseOutcome {
    let dir = match multi_hunk_diff_repo() {
        Ok(dir) => dir,
        Err(err) => return CaseOutcome::Fail(err),
    };
    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["git", "diff", "--staged"])
        .output();
    let via_stk = Command::new(self_exe)
        .current_dir(&dir)
        .args(["git", "diff", "--staged"])
        .output();
    let (via_rtk, via_stk) = match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => (rtk, stk),
        (Err(err), _) | (_, Err(err)) => {
            return CaseOutcome::Fail(format!("failed to spawn: {err}"))
        }
    };

    let rtk_stdout = String::from_utf8_lossy(&via_rtk.stdout);
    if !rtk_stdout.contains("renamed_name.rs") {
        return CaseOutcome::Skipped(
            "rtk itself doesn't mention the renamed file here -- inconclusive".to_string(),
        );
    }
    let stk_stdout = String::from_utf8_lossy(&via_stk.stdout);
    let missing: Vec<&str> = ["renamed_name.rs", "let a = 999"]
        .into_iter()
        .filter(|evidence| !stk_stdout.contains(evidence))
        .collect();
    if missing.is_empty() {
        CaseOutcome::Pass
    } else {
        CaseOutcome::Fail(format!("stk dropped required evidence: {missing:?}"))
    }
}

fn case_git_status(self_exe: &Path) -> CaseOutcome {
    let dir = match multi_hunk_diff_repo() {
        Ok(dir) => dir,
        Err(err) => return CaseOutcome::Fail(err),
    };
    let via_rtk = Command::new("rtk")
        .current_dir(&dir)
        .args(["git", "status"])
        .output();
    let via_stk = Command::new(self_exe)
        .current_dir(&dir)
        .args(["git", "status"])
        .output();
    let (via_rtk, via_stk) = match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) => (rtk, stk),
        (Err(err), _) | (_, Err(err)) => {
            return CaseOutcome::Fail(format!("failed to spawn: {err}"))
        }
    };

    let rtk_stdout = String::from_utf8_lossy(&via_rtk.stdout);
    if !rtk_stdout.contains("foo.rs") {
        return CaseOutcome::Skipped(
            "rtk itself doesn't list foo.rs here -- inconclusive".to_string(),
        );
    }
    let stk_stdout = String::from_utf8_lossy(&via_stk.stdout);
    let missing: Vec<&str> = ["foo.rs", "renamed_name.rs"]
        .into_iter()
        .filter(|evidence| !stk_stdout.contains(evidence))
        .collect();
    if missing.is_empty() {
        CaseOutcome::Pass
    } else {
        CaseOutcome::Fail(format!("stk dropped required evidence: {missing:?}"))
    }
}

/// `init`/`config`/`gain` don't produce byte-identical output to `rtk` by design --
/// here "equivalent-or-better" means neither binary errors on the same invocation.
fn smoke_both_exit_zero(rtk_cmd: &mut Command, stk_cmd: &mut Command) -> CaseOutcome {
    let via_rtk = rtk_cmd.output();
    let via_stk = stk_cmd.output();
    match (via_rtk, via_stk) {
        (Ok(rtk), Ok(stk)) if rtk.status.success() && stk.status.success() => CaseOutcome::Pass,
        (Ok(rtk), Ok(stk)) => CaseOutcome::Fail(format!(
            "exit codes: rtk={:?} stk={:?}",
            rtk.status.code(),
            stk.status.code()
        )),
        (Err(err), _) | (_, Err(err)) => CaseOutcome::Fail(format!("failed to spawn: {err}")),
    }
}

fn case_init_plain(self_exe: &Path) -> CaseOutcome {
    let rtk_home = unique_temp_dir("init-rtk-home");
    let rtk_cwd = unique_temp_dir("init-rtk-cwd");
    let stk_home = unique_temp_dir("init-stk-home");
    let stk_cwd = unique_temp_dir("init-stk-cwd");
    smoke_both_exit_zero(
        Command::new("rtk")
            .arg("init")
            .current_dir(&rtk_cwd)
            .env("HOME", &rtk_home)
            .env_remove("XDG_CONFIG_HOME"),
        Command::new(self_exe)
            .arg("init")
            .current_dir(&stk_cwd)
            .env("HOME", &stk_home)
            .env_remove("XDG_CONFIG_HOME"),
    )
}

fn case_config_plain(self_exe: &Path) -> CaseOutcome {
    let rtk_home = unique_temp_dir("config-rtk-home");
    let stk_home = unique_temp_dir("config-stk-home");
    smoke_both_exit_zero(
        Command::new("rtk")
            .arg("config")
            .env("HOME", &rtk_home)
            .env_remove("XDG_CONFIG_HOME"),
        Command::new(self_exe)
            .arg("config")
            .env("HOME", &stk_home)
            .env_remove("XDG_CONFIG_HOME"),
    )
}

fn case_gain_plain(self_exe: &Path) -> CaseOutcome {
    let rtk_home = unique_temp_dir("gain-rtk-home");
    let stk_home = unique_temp_dir("gain-stk-home");
    smoke_both_exit_zero(
        Command::new("rtk")
            .arg("gain")
            .env("HOME", &rtk_home)
            .env_remove("XDG_CACHE_HOME"),
        Command::new(self_exe)
            .arg("gain")
            .env("HOME", &stk_home)
            .env_remove("XDG_CACHE_HOME"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_line_reports_a_match_against_the_pinned_version() {
        let line = version_report_line(Some(PINNED_RTK_VERSION));
        assert!(line.contains("matches pinned"));
    }

    #[test]
    fn version_line_warns_on_a_mismatch() {
        let line = version_report_line(Some("0.99.0"));
        assert!(line.contains("WARNING"));
        assert!(line.contains("0.99.0"));
    }

    #[test]
    fn version_line_handles_an_undetectable_version() {
        let line = version_report_line(None);
        assert!(line.contains("unknown"));
    }

    #[test]
    fn wrong_args_is_a_usage_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = dispatch(
            &["--against".to_string(), "other".to_string()],
            &mut stdout,
            &mut stderr,
        );
        assert_eq!(code, 2);
        assert!(String::from_utf8(stderr).unwrap().contains("usage"));
    }

    #[test]
    fn no_args_is_a_usage_error() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = dispatch(&[], &mut stdout, &mut stderr);
        assert_eq!(code, 2);
    }
}
