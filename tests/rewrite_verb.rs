use std::process::Command;

fn stk_bin() -> &'static str {
    env!("CARGO_BIN_EXE_stk")
}

/// `rewrite`'s logic is a pure function, fully covered by `src/verbs/rewrite.rs`'s own unit
/// tests -- this is a wiring smoke test only, confirming `cli.rs`'s "rewrite" match arm actually
/// reaches it through the real binary, not a re-check of the rewrite rules themselves.
#[test]
fn rewrite_is_reachable_through_the_real_binary() {
    let output = Command::new(stk_bin())
        .args(["rewrite", "cargo build --release"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "stk cargo build --release\n"
    );
}
