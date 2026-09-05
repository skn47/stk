use std::process::Command;

/// Whether the real `rtk` binary is installed in this environment. The single source of
/// truth for that check -- both `stk bench` and the test suite's golden-comparison tests
/// use it, so it can't drift into disagreeing with itself.
pub fn is_available() -> bool {
    Command::new("rtk")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
