/// Golden-comparison tests skip (not fail) when the real `rtk` binary isn't installed
/// in this environment, per the spec's testing decisions.
pub fn is_available() -> bool {
    stk::rtk::is_available()
}
