use std::io::{ErrorKind, Read, Write};

use crate::normalize::ansi;

/// `stk pipe [-f FILTER] [--passthrough]`: read stdin, apply one named filter, print the
/// result. Defaults to passthrough when neither flag is given, matching real `rtk pipe`.
pub fn dispatch(
    args: &[String],
    mut stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32 {
    let mode = match parse(args) {
        Ok(mode) => mode,
        Err(message) => {
            let _ = writeln!(stderr, "stk: {message}");
            return 2;
        }
    };

    // Passthrough needs no buffering at all -- stream stdin straight to stdout, so a
    // huge input isn't held twice in memory or delayed waiting to be read in full first.
    if let Mode::Passthrough = mode {
        return match std::io::copy(&mut stdin, stdout) {
            Ok(_) => 0,
            Err(err) if err.kind() == ErrorKind::BrokenPipe => 0,
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to copy stdin to stdout: {err}");
                1
            }
        };
    }

    let Mode::Named(name) = mode else {
        unreachable!("Passthrough handled above")
    };

    let mut input = Vec::new();
    if let Err(err) = stdin.read_to_end(&mut input) {
        let _ = writeln!(stderr, "stk: failed to read stdin: {err}");
        return 1;
    }

    let output = match apply_filter(&name, &input) {
        Some(output) => output,
        None => {
            let _ = writeln!(
                stderr,
                "stk: unknown filter '{name}'. Available: {}",
                available_filters().join(", ")
            );
            return 1;
        }
    };

    match stdout.write_all(&output) {
        Ok(()) => 0,
        // The downstream reader closed early (e.g. `| head`) -- a normal Unix pipeline
        // shutdown, not a real error.
        Err(err) if err.kind() == ErrorKind::BrokenPipe => 0,
        Err(err) => {
            let _ = writeln!(stderr, "stk: failed to write output: {err}");
            1
        }
    }
}

enum Mode {
    Passthrough,
    Named(String),
}

fn parse(args: &[String]) -> Result<Mode, String> {
    let mut mode = Mode::Passthrough;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--passthrough" => {
                mode = Mode::Passthrough;
                i += 1;
            }
            "-f" | "--filter" => {
                let name = args
                    .get(i + 1)
                    .ok_or_else(|| "pipe -f requires a filter name".to_string())?;
                mode = Mode::Named(name.clone());
                i += 2;
            }
            other => return Err(format!("unrecognized argument '{other}'")),
        }
    }
    Ok(mode)
}

/// `ansi` is STK-original, not an alias of any real `rtk` filter name (unlike
/// `--passthrough`, which does match `rtk pipe` exactly).
fn available_filters() -> Vec<&'static str> {
    vec!["ansi"]
}

fn apply_filter(name: &str, input: &[u8]) -> Option<Vec<u8>> {
    match name {
        "ansi" => Some(ansi::strip(input)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(args: &[&str], stdin: &str) -> (i32, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let exit_code = dispatch(&args, stdin.as_bytes(), &mut stdout, &mut stderr);
        (
            exit_code,
            String::from_utf8(stdout).unwrap(),
            String::from_utf8(stderr).unwrap(),
        )
    }

    #[test]
    fn defaults_to_passthrough_when_no_flag_is_given() {
        let (exit_code, stdout, stderr) = run(&[], "hello\nworld\n");
        assert_eq!(exit_code, 0);
        assert_eq!(stdout, "hello\nworld\n");
        assert!(stderr.is_empty());
    }

    #[test]
    fn explicit_passthrough_flag_prints_input_unmodified() {
        let (exit_code, stdout, _) = run(&["--passthrough"], "\x1b[31mred\x1b[0m");
        assert_eq!(exit_code, 0);
        assert_eq!(stdout, "\x1b[31mred\x1b[0m");
    }

    #[test]
    fn named_filter_applies_the_transform() {
        let (exit_code, stdout, _) = run(&["-f", "ansi"], "\x1b[31mred\x1b[0m plain");
        assert_eq!(exit_code, 0);
        assert_eq!(stdout, "red plain");
    }

    #[test]
    fn unknown_filter_is_a_clear_error_not_silent_passthrough() {
        let (exit_code, stdout, stderr) = run(&["-f", "nonexistent"], "hello");
        assert_ne!(exit_code, 0);
        assert!(stdout.is_empty());
        assert!(stderr.contains("nonexistent"));
    }

    #[test]
    fn last_flag_wins_when_both_are_given() {
        let (_, stdout, _) = run(&["-f", "ansi", "--passthrough"], "\x1b[31mred\x1b[0m");
        assert_eq!(stdout, "\x1b[31mred\x1b[0m");
    }
}
