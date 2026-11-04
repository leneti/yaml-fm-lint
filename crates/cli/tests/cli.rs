use assert_cmd::Command;

#[test]
fn prints_version() {
    Command::cargo_bin("yaml-fm-lint")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout("yaml-fm-lint 2.0.0-dev\n");
}

#[test]
fn other_invocations_explain_and_exit_2() {
    Command::cargo_bin("yaml-fm-lint")
        .unwrap()
        .arg("docs")
        .assert()
        .code(2)
        .stderr(predicates::str::contains("under development"));
}
