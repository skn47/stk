use std::io::{Read, Write};

use crate::capture::executor::CommandExecutor;
use crate::compression;
use crate::history::HistoryStore;
use crate::specialists;
use crate::verbs;

/// Entry point every `stk` invocation goes through. Writes to `stdout`/`stderr` as output
/// becomes available (not a buffered return value) so `run`/`proxy` can stream live later.
pub fn run(
    args: &[String],
    stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    executor: &dyn CommandExecutor,
    history: &dyn HistoryStore,
) -> i32 {
    let (options, rest) = match parse_global_options(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            let _ = writeln!(stderr, "stk: {message}");
            return 2;
        }
    };

    match rest.split_first() {
        Some((command, rest)) if command == "run" => verbs::run::dispatch(rest, stderr, executor),
        Some((command, rest)) if command == "proxy" => {
            verbs::proxy::dispatch(rest, stderr, executor, history)
        }
        Some((command, rest)) if command == "pipe" => {
            verbs::pipe::dispatch(rest, stdin, stdout, stderr)
        }
        Some((command, rest)) if command == "compile" => {
            verbs::compile::dispatch(rest, stdin, stdout, stderr, executor, options.budget)
        }
        Some((command, rest)) if command == "read" => {
            verbs::read::dispatch(rest, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "grep" => {
            verbs::grep::dispatch(rest, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "find" => {
            verbs::find::dispatch(rest, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "log" => {
            verbs::log::dispatch(rest, stdin, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "err" => {
            verbs::err::dispatch(rest, stdout, stderr, executor, options.budget)
        }
        Some((command, rest)) if command == "summary" => {
            verbs::summary::dispatch(rest, stdout, stderr, executor, options.budget)
        }
        Some((command, rest)) if command == "diff" => {
            verbs::diff::dispatch(rest, stdin, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "json" => {
            verbs::json::dispatch(rest, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "config" => {
            verbs::config::dispatch(rest, stdout, stderr, options.budget)
        }
        Some((command, rest)) if command == "init" => verbs::init::dispatch(rest, stdout, stderr),
        Some((command, rest)) => match specialists::lookup(command) {
            Some(classify) => compression::execute_and_compress(
                command,
                rest,
                stdout,
                stderr,
                executor,
                options.budget,
                classify,
            ),
            None => unsupported_command(command, stderr),
        },
        None => no_command_given(stderr),
    }
}

#[derive(Debug, Default)]
struct GlobalOptions {
    budget: Option<usize>,
}

/// Global options are recognized only before the subcommand (matching `rtk`). Rejects
/// `--intent`/`--explain` rather than silently ignoring them: neither ships yet.
fn parse_global_options(args: &[String]) -> Result<(GlobalOptions, &[String]), String> {
    let mut options = GlobalOptions::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--budget" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--budget requires a value".to_string())?;
                options.budget = Some(
                    value
                        .parse()
                        .map_err(|_| format!("--budget value '{value}' is not a valid number"))?,
                );
                i += 2;
            }
            "--intent" | "--explain" => {
                return Err(format!(
                    "unrecognized flag '{}' (not implemented in this release)",
                    args[i]
                ));
            }
            _ => break,
        }
    }
    Ok((options, &args[i..]))
}

fn unsupported_command(command: &str, stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(
        stderr,
        "stk: unsupported command '{command}' (no built-in verb or specialist is registered for it)"
    );
    2
}

fn no_command_given(stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "stk: no command given (try 'stk --help')");
    2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_flag_is_parsed_and_stripped_before_the_subcommand() {
        let args: Vec<String> = ["--budget", "500", "compile"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (options, rest) = parse_global_options(&args).unwrap();
        assert_eq!(options.budget, Some(500));
        assert_eq!(rest, &["compile".to_string()]);
    }

    #[test]
    fn no_global_options_leaves_args_untouched() {
        let args: Vec<String> = ["run", "echo", "hi"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let (options, rest) = parse_global_options(&args).unwrap();
        assert_eq!(options.budget, None);
        assert_eq!(rest, args.as_slice());
    }

    #[test]
    fn intent_and_explain_are_rejected() {
        let args: Vec<String> = vec!["--intent".to_string(), "debug".to_string()];
        assert!(parse_global_options(&args).is_err());

        let args: Vec<String> = vec!["--explain".to_string()];
        assert!(parse_global_options(&args).is_err());
    }

    #[test]
    fn budget_missing_value_is_an_error() {
        let args: Vec<String> = vec!["--budget".to_string()];
        assert!(parse_global_options(&args).is_err());
    }

    #[test]
    fn budget_non_numeric_value_is_an_error() {
        let args: Vec<String> = vec!["--budget".to_string(), "abc".to_string()];
        assert!(parse_global_options(&args).is_err());
    }
}
