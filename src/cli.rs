use std::io::{Read, Write};

use crate::capture::executor::CommandExecutor;
use crate::history::HistoryStore;
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
    match args.split_first() {
        Some((command, rest)) if command == "run" => verbs::run::dispatch(rest, stderr, executor),
        Some((command, rest)) if command == "proxy" => {
            verbs::proxy::dispatch(rest, stderr, executor, history)
        }
        Some((command, rest)) if command == "pipe" => {
            verbs::pipe::dispatch(rest, stdin, stdout, stderr)
        }
        Some((command, _)) => unsupported_command(command, stderr),
        None => no_command_given(stderr),
    }
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
