use std::io::{Read, Write};

use crate::capture::executor::CommandExecutor;

/// Entry point every `stk` invocation goes through. Writes to `stdout`/`stderr` as output
/// becomes available (not a buffered return value) so `run`/`proxy` can stream live later.
pub fn run(
    args: &[String],
    _stdin: impl Read,
    _stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    _executor: &dyn CommandExecutor,
) -> i32 {
    match args.first() {
        Some(command) => unsupported_command(command, stderr),
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
