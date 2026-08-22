//! First vertical slice: macos-system + homebrew end to end. SPEC.md §9.

mod common;

use common::Fixture;
use predicates::prelude::*;

#[test]
fn homebrew_formula_via_path_lookup() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");

    fixture
        .command()
        .arg("jq")
        .assert()
        .success()
        .code(0)
        .stdout("jq: homebrew formula jq 1.7.1\n")
        .stderr("");
}

#[test]
fn macos_system_via_absolute_path_argument() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("/bin/ls")
        .assert()
        .success()
        .code(0)
        .stdout("ls: macos-system\n");
}

#[test]
fn macos_system_shell_command_under_usr_bin() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("/usr/bin/git")
        .assert()
        .success()
        .code(0)
        .stdout("git: macos-system (shell command)\n");
}

#[test]
fn unrecognized_binary_is_unknown_exit_1() {
    let fixture = Fixture::new();
    let path = fixture.plain_binary("slim");

    fixture
        .command()
        .arg("slim")
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains(format!(
            "could not determine install source of {}",
            path.display()
        )));
}

#[test]
fn command_not_found_exits_2() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("nope")
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicate::str::contains("nope: command not found"));
}

#[test]
fn missing_path_argument_exits_2() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("./does-not-exist")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("no such file"));
}

#[test]
fn homebrew_via_opt_homebrew_prefix() {
    let fixture = Fixture::new();
    fixture.homebrew("/opt/homebrew", "jq", "1.7.1");

    fixture
        .command()
        .arg("jq")
        .assert()
        .success()
        .stdout("jq: homebrew formula jq 1.7.1\n");
}
