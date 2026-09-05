use std::io::{Read, Write};

use crate::compression;
use crate::scoring::relevance;

/// `stk log [FILE]`: filters/deduplicates log output (file or stdin, matching real
/// `rtk log`'s own signature), reusing the same fast path/`Budget` pipeline `compile`
/// uses for stdin -- `log` only adds an optional file-path source.
pub fn dispatch(
    args: &[String],
    mut stdin: impl Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    budget: Option<usize>,
) -> i32 {
    let input = match args.first() {
        Some(path) => match std::fs::read(path) {
            Ok(content) => content,
            Err(err) => {
                let _ = writeln!(stderr, "stk: failed to read '{path}': {err}");
                return 1;
            }
        },
        None => {
            let mut buf = Vec::new();
            if let Err(err) = stdin.read_to_end(&mut buf) {
                let _ = writeln!(stderr, "stk: failed to read stdin: {err}");
                return 1;
            }
            buf
        }
    };

    compression::compress_stdin(&input, stdout, stderr, budget, relevance::classify)
}
