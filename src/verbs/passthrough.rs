use std::io::Write;

/// Shared usage/exec-failure error formatting for the passthrough verbs (`run`/`proxy`),
/// which must report failures identically since they're otherwise byte-identical.
pub fn no_command_error(verb: &str, stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "stk: {verb} requires a command");
    2
}

pub fn exec_error(command: &str, err: &std::io::Error, stderr: &mut dyn Write) -> i32 {
    let _ = writeln!(stderr, "stk: failed to run '{command}': {err}");
    1
}
