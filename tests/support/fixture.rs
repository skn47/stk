use serde::Deserialize;
use std::path::Path;

/// A raw command or literal input from the fixture corpus, with expectations about what
/// must survive compression and what may be removed. A raw passthrough verb removes
/// nothing, so `may_remove` goes unused by tests of one. `stdin` is for canned log
/// content with no real underlying command (e.g. fast-path fixtures); `command` is for
/// golden-comparison fixtures that must actually run against the real `rtk`.
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub name: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub stdin: Option<String>,
    #[serde(default)]
    pub must_preserve: Vec<String>,
    #[serde(default)]
    pub may_remove: Vec<String>,
}

pub fn load(path: impl AsRef<Path>) -> Fixture {
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("fixture {:?} should be readable: {e}", path.as_ref()));
    toml::from_str(&text)
        .unwrap_or_else(|e| panic!("fixture {:?} should be valid TOML: {e}", path.as_ref()))
}

pub fn assert_preserves(output: &str, fixture: &Fixture) {
    for needle in &fixture.must_preserve {
        assert!(
            output.contains(needle.as_str()),
            "expected output to preserve {needle:?}, got: {output:?}"
        );
    }
}
