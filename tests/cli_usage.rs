use assert_cmd::Command;

fn install_from() -> Command {
    Command::cargo_bin("install-from").unwrap()
}

#[test]
fn unknown_flag_exits_3() {
    install_from().arg("--bogus").assert().failure().code(3);
}

#[test]
fn missing_argument_exits_3() {
    install_from().assert().failure().code(3);
}

#[test]
fn help_exits_0() {
    install_from().arg("--help").assert().success().code(0);
}

#[test]
fn version_flag_exits_0() {
    install_from().arg("-V").assert().success().code(0);
}
