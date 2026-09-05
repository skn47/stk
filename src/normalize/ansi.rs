/// Strips ANSI CSI escape sequences (`ESC '[' params... final-byte` — the color/cursor
/// codes terminal output actually uses). Doesn't handle OSC or other rarer escape forms.
pub fn strip(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == 0x1B && input.get(i + 1) == Some(&b'[') {
            let mut j = i + 2;
            while j < input.len() && (0x30..=0x3F).contains(&input[j]) {
                j += 1;
            }
            while j < input.len() && (0x20..=0x2F).contains(&input[j]) {
                j += 1;
            }
            match input.get(j) {
                // Valid final byte: the whole sequence is a real CSI code, strip it.
                Some(&byte) if (0x40..=0x7E).contains(&byte) => {
                    i = j + 1;
                    continue;
                }
                // Truncated at end of input: drop the incomplete remnant too.
                None => {
                    i = j;
                    continue;
                }
                // Anything else (e.g. another ESC) isn't a valid final byte, so this
                // isn't really a CSI sequence -- leave it as literal text rather than
                // consuming a byte that might start a real sequence of its own.
                Some(_) => {}
            }
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_color_codes() {
        let input = b"\x1b[31mred text\x1b[0m plain";
        assert_eq!(strip(input), b"red text plain");
    }

    #[test]
    fn leaves_plain_text_untouched() {
        assert_eq!(strip(b"no escapes here"), b"no escapes here");
    }

    #[test]
    fn handles_an_unterminated_escape_at_end_of_input_without_panicking() {
        let input = b"before\x1b[31";
        assert_eq!(strip(input), b"before");
    }

    #[test]
    fn handles_cursor_movement_and_erase_codes() {
        let input = b"\x1b[2K\x1b[1Gloading...";
        assert_eq!(strip(input), b"loading...");
    }

    #[test]
    fn a_malformed_escape_does_not_swallow_a_subsequent_real_one() {
        let input = b"\x1b[1;\x1b[32mgreen\x1b[0m";
        assert_eq!(strip(input), b"\x1b[1;green");
    }
}
