//! Integration tests for the OxideAgent application.

use assert_cmd::Command;

#[test]
fn test_help_output() {
    let mut cmd = Command::cargo_bin("OxideAgent").unwrap();
    let assert = cmd.arg("--help").assert();
    assert.success();
}
