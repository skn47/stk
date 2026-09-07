use std::io::Write;
use std::path::Path;

use crate::config::settings;

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

    let global_path = settings::global_config_path();

    if create {
        return create_config(&global_path, stdout, stderr);
    }

    let project_text = match settings::read_config_file(&settings::project_config_path()) {
        Ok(text) => text,
        Err(err) => {
            let _ = writeln!(stderr, "stk: {err}");
            return 1;
        }
    };
    let global_text = match settings::read_config_file(&global_path) {
        Ok(text) => text,
        Err(err) => {
            let _ = writeln!(stderr, "stk: {err}");
            return 1;
        }
    };

    let env = match settings::env_settings() {
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

fn create_config(path: &Path, stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    match settings::create_default_file(path) {
        Ok(true) => {
            let _ = writeln!(stdout, "Created: {}", path.display());
            0
        }
        Ok(false) => {
            let _ = writeln!(stdout, "Config already exists: {}", path.display());
            0
        }
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to create '{}': {err}", path.display());
            1
        }
    }
}
