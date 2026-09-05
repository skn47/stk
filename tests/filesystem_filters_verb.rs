use std::path::PathBuf;

use stk::budget::tokenizer::TokenCounter;
use stk::capture::executor::FakeExecutor;
use stk::history::FakeHistoryStore;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "stk-fs-filters-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[String]) -> (i32, String, String) {
    let executor = FakeExecutor::new();
    let history = FakeHistoryStore::new();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    let exit_code = stk::cli::run(
        args,
        std::io::empty(),
        &mut stdout,
        &mut stderr,
        &executor,
        &history,
    );

    assert!(
        executor.invocations().is_empty(),
        "filesystem filters must never reach the CommandExecutor"
    );

    (
        exit_code,
        String::from_utf8(stdout).unwrap(),
        String::from_utf8(stderr).unwrap(),
    )
}

fn args(strs: &[&str]) -> Vec<String> {
    strs.iter().map(|s| s.to_string()).collect()
}

#[test]
fn read_returns_a_small_files_content_verbatim() {
    let dir = temp_dir("read-small");
    let file = dir.join("small.txt");
    std::fs::write(&file, "hello\nworld\n").unwrap();

    let (exit_code, stdout, _) = run(&args(&["read", file.to_str().unwrap()]));

    assert_eq!(exit_code, 0);
    assert_eq!(stdout, "hello\nworld\n");
}

#[test]
fn read_of_a_file_too_large_for_the_budget_keeps_head_and_tail_with_a_marker() {
    let dir = temp_dir("read-large");
    let file = dir.join("large.txt");
    let content: String = (0..300).map(|i| format!("line number {i}\n")).collect();
    std::fs::write(&file, &content).unwrap();

    let (exit_code, stdout, _) = run(&args(&["--budget", "80", "read", file.to_str().unwrap()]));

    assert_eq!(exit_code, 0);
    assert!(stdout.starts_with("line number 0"));
    assert!(stdout.trim_end().ends_with("line number 299"));
    assert!(
        stdout.contains("[stk: omitted"),
        "expected an omission marker for the dropped middle, got: {stdout:?}"
    );
    let rendered_tokens = stk::budget::tokenizer::ApproximateCounter.count(&stdout);
    assert!(
        rendered_tokens <= 80,
        "rendered output must honor the budget, got {rendered_tokens} tokens"
    );
}

#[test]
fn read_reports_a_missing_file_clearly() {
    let dir = temp_dir("read-missing");
    let missing = dir.join("does_not_exist.txt");

    let (exit_code, stdout, stderr) = run(&args(&["read", missing.to_str().unwrap()]));

    assert_ne!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(!stderr.is_empty());
}

#[test]
fn grep_finds_matches_recursively_with_line_numbers() {
    let dir = temp_dir("grep");
    std::fs::write(dir.join("a.txt"), "no match\nneedle here\n").unwrap();
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("b.txt"), "another needle\nirrelevant\n").unwrap();

    let (exit_code, stdout, _) = run(&args(&["grep", "needle", dir.to_str().unwrap()]));

    assert_eq!(exit_code, 0);
    assert!(stdout.contains("a.txt:2:needle here"));
    assert!(stdout.contains("b.txt:1:another needle"));
    assert!(!stdout.contains("irrelevant"));
}

#[test]
fn grep_running_twice_against_unchanged_files_is_stateless_and_identical() {
    let dir = temp_dir("grep-stateless");
    std::fs::write(dir.join("a.txt"), "needle\n").unwrap();

    let (_, first, _) = run(&args(&["grep", "needle", dir.to_str().unwrap()]));
    let (_, second, _) = run(&args(&["grep", "needle", dir.to_str().unwrap()]));

    assert_eq!(first, second);
}

#[test]
fn find_lists_files_recursively_and_supports_a_name_filter() {
    let dir = temp_dir("find");
    std::fs::write(dir.join("keep.rs"), "").unwrap();
    std::fs::write(dir.join("skip.txt"), "").unwrap();
    let sub = dir.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(sub.join("nested.rs"), "").unwrap();

    let (exit_code, stdout, _) = run(&args(&["find", dir.to_str().unwrap()]));
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("keep.rs"));
    assert!(stdout.contains("skip.txt"));
    assert!(stdout.contains("nested.rs"));

    let (_, filtered, _) = run(&args(&["find", dir.to_str().unwrap(), "--name", ".rs"]));
    assert!(filtered.contains("keep.rs"));
    assert!(filtered.contains("nested.rs"));
    assert!(!filtered.contains("skip.txt"));
}
