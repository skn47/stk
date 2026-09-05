/// Whether the real `rtk` binary is installed in this environment. Golden-comparison
/// tests skip (not fail) when it isn't, per the spec's testing decisions.
pub fn is_available() -> bool {
    std::process::Command::new("rtk")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
