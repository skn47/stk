use std::io::Write;

use crate::specialists;

/// `stk rewrite "<command string>"`: prefixes a raw command with `stk ` when safe, for the
/// `PreToolUse:Bash` hook to auto-apply. Exit 0 + stdout on a safe rewrite, exit 1 (silent)
/// otherwise -- no deny/ask tier, since Claude Code's own permissions own that decision.
pub fn dispatch(args: &[String], stdout: &mut dyn Write, _stderr: &mut dyn Write) -> i32 {
    let command = args.join(" ");
    match rewrite(&command) {
        Some(rewritten) => {
            let _ = writeln!(stdout, "{rewritten}");
            0
        }
        None => 1,
    }
}

/// Specialist dispatch always executes the real binary with the exact args given, unchanged
/// (see `compression::execute_and_compress`), so prepending `stk ` in front is always safe --
/// including a chained form like `cargo build && cargo test`, since the prefix only ever lands
/// at the very front. Delegates to `specialists::lookup` (rather than a second hardcoded list)
/// so a future specialist is automatically rewrite-eligible too.
fn rewrite(command: &str) -> Option<String> {
    let trimmed = command.trim();
    // A heredoc's body could contain arbitrary text; today's front-only prepend doesn't
    // strictly need this, but it's cheap insurance for a future, smarter rewrite.
    if trimmed.contains("<<") {
        return None;
    }

    // Empty input (no first word) and an already-"stk"-prefixed command (no specialist
    // registered under that name) both fall through to `None` here without a separate guard.
    let first_word = trimmed.split_whitespace().next()?;
    specialists::lookup(first_word)?;
    Some(format!("stk {trimmed}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_a_plain_cargo_command() {
        assert_eq!(
            rewrite("cargo build --release"),
            Some("stk cargo build --release".to_string())
        );
    }

    #[test]
    fn rewrites_a_plain_git_command() {
        assert_eq!(rewrite("git status"), Some("stk git status".to_string()));
    }

    #[test]
    fn rewrites_a_bare_command_with_no_args() {
        assert_eq!(rewrite("cargo"), Some("stk cargo".to_string()));
    }

    #[test]
    fn rewrites_a_chained_command_prefix_only() {
        assert_eq!(
            rewrite("cargo build && cargo test"),
            Some("stk cargo build && cargo test".to_string())
        );
    }

    #[test]
    fn trims_surrounding_whitespace_before_rewriting() {
        assert_eq!(
            rewrite("  cargo check  "),
            Some("stk cargo check".to_string())
        );
    }

    #[test]
    fn refuses_empty_input() {
        assert_eq!(rewrite(""), None);
        assert_eq!(rewrite("   "), None);
    }

    #[test]
    fn refuses_a_command_already_prefixed_with_stk() {
        assert_eq!(rewrite("stk cargo build"), None);
        assert_eq!(rewrite("stk"), None);
    }

    #[test]
    fn refuses_a_command_containing_a_heredoc() {
        assert_eq!(rewrite("cargo build <<'EOF'\nsomething\nEOF"), None);
    }

    #[test]
    fn refuses_a_non_cargo_git_command() {
        assert_eq!(rewrite("npm install"), None);
        assert_eq!(rewrite("ls -la"), None);
    }

    #[test]
    fn refuses_a_command_that_only_contains_cargo_as_a_substring() {
        // "cargofmt" is not the word "cargo" -- must not match on prefix alone.
        assert_eq!(rewrite("cargofmt --check"), None);
    }

    #[test]
    fn dispatch_prints_the_rewritten_command_and_exits_zero() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = dispatch(&["cargo build".to_string()], &mut stdout, &mut stderr);
        assert_eq!(code, 0);
        assert_eq!(String::from_utf8(stdout).unwrap(), "stk cargo build\n");
    }

    #[test]
    fn dispatch_prints_nothing_and_exits_one_when_no_rewrite_applies() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let code = dispatch(&["npm install".to_string()], &mut stdout, &mut stderr);
        assert_eq!(code, 1);
        assert!(stdout.is_empty());
    }

    #[test]
    fn dispatch_joins_multiple_unquoted_args_into_one_command() {
        // Robustness: if a caller doesn't quote the command as a single argv item, rejoin it.
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args: Vec<String> = ["cargo", "build", "--release"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let code = dispatch(&args, &mut stdout, &mut stderr);
        assert_eq!(code, 0);
        assert_eq!(
            String::from_utf8(stdout).unwrap(),
            "stk cargo build --release\n"
        );
    }
}
