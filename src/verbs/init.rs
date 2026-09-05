use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::config::settings;

const MARKER_START: &str = "<!-- stk-instructions v1 -->";
const MARKER_END: &str = "<!-- /stk-instructions -->";

const INSTRUCTIONS_BODY: &str = "\
# STK (Stateful Token Killer)

Prefix commands with `stk` to render their output within a token budget instead of raw \
tool noise.

## Commands

- `stk cargo <args>` / `stk git <args>` -- Specialist-aware compression for Cargo and Git.
- `stk compile` -- generic budgeted pipeline for any other command's output (or piped stdin).
- `stk read` / `stk grep` / `stk find` -- native, budget-aware filesystem filters.
- `stk log` / `stk err` / `stk summary` / `stk diff` / `stk json` -- native stream filters.
- `stk run` / `stk proxy` -- unfiltered passthrough, or transparent usage tracking.
- `stk gain` -- token-savings summary. `stk config` -- show/create configuration.

Add `--budget N` before the subcommand to set an explicit token ceiling.";

enum Target {
    ProjectClaudeMd,
    GlobalClaudeMd,
    ProjectConfig,
}

fn parse_target(args: &[String]) -> Result<Target, String> {
    let mut global = false;
    let mut codex = false;
    for arg in args {
        match arg.as_str() {
            "--global" | "-g" => global = true,
            "--codex" => codex = true,
            other => return Err(format!("unrecognized argument '{other}'")),
        }
    }
    match (global, codex) {
        (true, true) => Err("--codex and --global/-g cannot be combined".to_string()),
        (true, false) => Ok(Target::GlobalClaudeMd),
        (false, true) => Ok(Target::ProjectConfig),
        (false, false) => Ok(Target::ProjectClaudeMd),
    }
}

/// `stk init [--global|-g] [--codex]`: installs shell/agent integration idempotently,
/// printing exactly which file it created, updated, or left alone.
pub fn dispatch(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let target = match parse_target(args) {
        Ok(target) => target,
        Err(err) => {
            let _ = writeln!(stderr, "stk: init: {err}");
            return 2;
        }
    };

    match target {
        Target::ProjectConfig => install_project_config(stdout, stderr),
        Target::GlobalClaudeMd => install_claude_md_block(&global_claude_md_path(), stdout, stderr),
        Target::ProjectClaudeMd => {
            install_claude_md_block(&PathBuf::from("CLAUDE.md"), stdout, stderr)
        }
    }
}

/// `--codex` writes ordinary project config, the same one `stk config` reads -- matching
/// the real `rtk init --codex`'s install-time-only behavior (no runtime detection layer).
fn install_project_config(stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let path = PathBuf::from(".stk").join("config.toml");
    match settings::create_default_file(&path) {
        Ok(true) => {
            let _ = writeln!(stdout, "Created: {}", path.display());
            0
        }
        Ok(false) => {
            let _ = writeln!(
                stdout,
                "No changes needed ('{}' already exists)",
                path.display()
            );
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to create '{}': {err}", path.display());
            1
        }
    }
}

fn global_claude_md_path() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".claude").join("CLAUDE.md");
    }
    // Unset HOME (a minimal container or cron job) must still resolve to a stable
    // absolute path, not a cwd-relative one -- matches history.rs's cache_dir().
    std::env::temp_dir().join("stk-global").join("CLAUDE.md")
}

fn render_block() -> String {
    format!("{MARKER_START}\n{INSTRUCTIONS_BODY}\n{MARKER_END}\n")
}

/// Locates the single well-formed `MARKER_START..MARKER_END` span, if any. Refuses
/// (rather than guessing) when the markers are missing, duplicated, or out of order --
/// pairing an arbitrary start with the next end found anywhere after it risks silently
/// deleting real content that happens to sit between an orphaned marker and the next one.
fn find_managed_block(text: &str) -> Result<Option<(usize, usize)>, String> {
    let start_count = text.matches(MARKER_START).count();
    let end_count = text.matches(MARKER_END).count();
    if start_count == 0 && end_count == 0 {
        return Ok(None);
    }
    if start_count == 1 && end_count == 1 {
        let start = text.find(MARKER_START).unwrap();
        if let Some(end_rel) = text[start..].find(MARKER_END) {
            let mut end = start + end_rel + MARKER_END.len();
            // Consume one trailing newline too, so replacing the span with a freshly
            // rendered block (which supplies its own trailing newline) doesn't double it.
            if text[end..].starts_with('\n') {
                end += 1;
            }
            return Ok(Some((start, end)));
        }
    }
    Err(format!(
        "'{MARKER_START}' / '{MARKER_END}' markers look malformed or duplicated -- resolve manually before running `stk init` again"
    ))
}

fn write_atomically(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut tmp_name = path.as_os_str().to_os_string();
    tmp_name.push(".stk-init.tmp");
    let tmp_path = PathBuf::from(tmp_name);
    std::fs::write(&tmp_path, contents)?;
    std::fs::rename(&tmp_path, path)
}

fn install_claude_md_block(path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    let existing = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to read '{}': {err}", path.display());
            return 1;
        }
    };

    let block = render_block();
    let new_content = match &existing {
        None => block.clone(),
        Some(text) => match find_managed_block(text) {
            Ok(Some((start, end))) => format!("{}{}{}", &text[..start], block, &text[end..]),
            Ok(None) => {
                if text.is_empty() || text.ends_with('\n') {
                    format!("{text}\n{block}")
                } else {
                    format!("{text}\n\n{block}")
                }
            }
            Err(err) => {
                let _ = writeln!(stderr, "stk: {} in '{}'", err, path.display());
                return 1;
            }
        },
    };

    if existing.as_deref() == Some(new_content.as_str()) {
        let _ = writeln!(
            stdout,
            "No changes needed ('{}' already up to date)",
            path.display()
        );
        return 0;
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            if let Err(err) = std::fs::create_dir_all(parent) {
                let _ = writeln!(
                    stderr,
                    "stk: failed to create '{}': {err}",
                    parent.display()
                );
                return 1;
            }
        }
    }
    if let Err(err) = write_atomically(path, &new_content) {
        let _ = writeln!(stderr, "stk: failed to write '{}': {err}", path.display());
        return 1;
    }

    if existing.is_none() {
        let _ = writeln!(stdout, "Created: {}", path.display());
    } else {
        let _ = writeln!(stdout, "Updated: {}", path.display());
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_markers_at_all_is_none() {
        assert_eq!(find_managed_block("just some notes").unwrap(), None);
    }

    #[test]
    fn a_well_formed_block_is_found() {
        let text = format!("before\n{MARKER_START}\nbody\n{MARKER_END}\nafter");
        let (start, end) = find_managed_block(&text).unwrap().unwrap();
        assert_eq!(
            &text[start..end],
            format!("{MARKER_START}\nbody\n{MARKER_END}\n")
        );
    }

    #[test]
    fn an_orphaned_start_with_no_end_anywhere_is_refused_not_silently_appended_past() {
        let text = format!("{MARKER_START}\nstray leftover text with no end marker");
        assert!(find_managed_block(&text).is_err());
    }

    #[test]
    fn an_orphaned_start_paired_with_a_later_unrelated_end_is_refused() {
        // Regression: previously this silently treated the orphaned start and the next
        // end marker in the file as one span, deleting the real content between them.
        let text = format!(
            "{MARKER_START}\nstray leftover text\nsome other content\n{MARKER_END}\nmore stray content\n{MARKER_END}"
        );
        assert!(find_managed_block(&text).is_err());
    }

    #[test]
    fn duplicated_start_markers_are_refused() {
        let text = format!("{MARKER_START}\na\n{MARKER_END}\n{MARKER_START}\nb\n{MARKER_END}");
        assert!(find_managed_block(&text).is_err());
    }

    #[test]
    fn end_before_start_is_refused() {
        let text = format!("{MARKER_END}\nfiller\n{MARKER_START}");
        assert!(find_managed_block(&text).is_err());
    }
}
