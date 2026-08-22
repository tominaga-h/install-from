//! `pip` end to end, including the priority rule against `homebrew`.
//! SPEC.md §5.1 row 11, §5, §9.

mod common;

use common::Fixture;
use predicates::prelude::*;

#[test]
fn pip_script_via_shebang() {
    let fixture = Fixture::new();
    fixture.pip_script("black", "#!/usr/local/opt/python@3.12/bin/python3.12");

    fixture
        .command()
        .arg("black")
        .assert()
        .success()
        .code(0)
        .stdout("black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)\n");
}

#[test]
fn pip_script_via_env_form() {
    let fixture = Fixture::new();
    fixture.pip_script("mypy", "#!/usr/bin/env python3");

    fixture
        .command()
        .arg("mypy")
        .assert()
        .success()
        .stdout("mypy: pip (python: python3)\n");
}

#[test]
fn homebrew_wins_over_pip_for_a_script_inside_cellar() {
    let fixture = Fixture::new();
    fixture.pip_script_inside_homebrew_cellar(
        "/usr/local",
        "black",
        "24.2.0",
        "black",
        "#!/usr/local/Cellar/black/24.2.0/libexec/bin/python3.12",
    );

    // The file has a Python shebang AND lives under Cellar/. Per SPEC.md
    // §5 "whoever placed the file wins", Homebrew's Cellar placement
    // takes priority over the pip shebang heuristic.
    fixture
        .command()
        .arg("black")
        .assert()
        .success()
        .code(0)
        .stdout("black: homebrew formula black 24.2.0\n");
}

#[test]
fn non_python_shebang_is_unknown() {
    let fixture = Fixture::new();
    fixture.pip_script("myscript", "#!/bin/bash");

    fixture
        .command()
        .arg("myscript")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "could not determine install source of",
        ));
}
