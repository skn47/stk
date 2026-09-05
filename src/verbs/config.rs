use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::config::settings::{self, PartialSettings, Settings};

/// `stk config [--create]`: shows the effective, precedence-resolved configuration, or
/// creates the global config file with defaults if `--create` is given and none exists.
pub fn dispatch(
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
) -> i32 {
    let mut create = false;
    for arg in args {
        match arg.as_str() {
            "--create" => create = true,
            other => {
                let _ = writeln!(stderr, "stk: config: unrecognized argument '{other}'");
                return 2;
            }
        }
    }

    let global_path = global_config_path();

    if create {
        return create_config(&global_path, stdout, stderr);
    }

    let project_text = match read_config_file(&project_config_path(), stderr) {
        Ok(text) => text,
        Err(code) => return code,
    };
    let global_text = match read_config_file(&global_path, stderr) {
        Ok(text) => text,
        Err(code) => return code,
    };

    let env = match env_settings() {
        Ok(env) => env,
        Err(err) => {
            let _ = writeln!(stderr, "stk: {err}");
            return 2;
        }
    };

    let resolved =
        match settings::resolve(budget, env, project_text.as_deref(), global_text.as_deref()) {
            Ok(resolved) => resolved,
            Err(err) => {
                let _ = writeln!(stderr, "stk: {err}");
                return 1;
            }
        };

    let _ = writeln!(stdout, "Config: {}", global_path.display());
    if global_text.is_none() {
        let _ = writeln!(
            stdout,
            "\n(no config file at this path yet -- run `stk config --create`)"
        );
    }
    let _ = write!(stdout, "\n{}", settings::render(&resolved));
    0
}

/// `NotFound` means "this layer doesn't exist," not an error; any other read failure
/// (permission denied, not valid UTF-8, ...) is surfaced rather than silently ignored.
fn read_config_file(path: &Path, stderr: &mut dyn Write) -> Result<Option<String>, i32> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to read '{}': {err}", path.display());
            Err(1)
        }
    }
}

/// Uses `create_new` (atomic create-or-fail) rather than a separate `exists()` check
/// followed by a write, so two concurrent `--create` runs can't race past the check and
/// have the second one silently clobber the first's file.
fn create_config(path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            let _ = writeln!(
                stderr,
                "stk: failed to create '{}': {err}",
                parent.display()
            );
            return 1;
        }
    }

    let contents = settings::render(&Settings::defaults());
    match OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(mut file) => match file.write_all(contents.as_bytes()) {
            Ok(()) => {
                let _ = writeln!(stdout, "Created: {}", path.display());
                0
            }
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to write '{}': {err}", path.display());
                1
            }
        },
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            let _ = writeln!(stdout, "Config already exists: {}", path.display());
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to create '{}': {err}", path.display());
            1
        }
    }
}

/// Matches `src/history.rs`'s `cache_dir()` fallback order: an unset `HOME` (a minimal
/// container or cron job) must resolve to a stable absolute path, not a cwd-relative one.
fn global_config_path() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(xdg).join("stk").join("config.toml");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home)
            .join(".config")
            .join("stk")
            .join("config.toml");
    }
    std::env::temp_dir().join("stk-config").join("config.toml")
}

fn project_config_path() -> PathBuf {
    PathBuf::from(".stk").join("config.toml")
}

/// Errors (rather than silently ignoring) on a malformed value, matching how a malformed
/// *file* layer errors -- a typo'd env var should never be mistaken for "not set".
fn env_settings() -> Result<PartialSettings, String> {
    Ok(PartialSettings {
        budget: parse_env("STK_BUDGET")?,
        intent: std::env::var("STK_INTENT").ok(),
        session_memory: parse_env("STK_SESSION_MEMORY")?,
        session_ttl_minutes: parse_env("STK_SESSION_TTL_MINUTES")?,
    })
}

fn parse_env<T: std::str::FromStr>(key: &str) -> Result<Option<T>, String> {
    match std::env::var(key) {
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| format!("invalid value for {key}: '{value}'")),
        Err(_) => Ok(None),
    }
}
