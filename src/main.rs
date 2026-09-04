use std::env;
use std::io;
use std::process::ExitCode;

use stk::capture::executor::RealExecutor;
use stk::cli;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let executor = RealExecutor;

    let mut stdout = io::stdout();
    let mut stderr = io::stderr();

    let exit_code = cli::run(&args, io::stdin(), &mut stdout, &mut stderr, &executor);
    // Clamp rather than cast: exit_code is signed and can exceed u8, and `as u8` would wrap silently.
    ExitCode::from(exit_code.clamp(0, 255) as u8)
}
