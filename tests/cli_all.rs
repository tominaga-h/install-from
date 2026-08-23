//! `--all`: evaluate every PATH match. SPEC.md §2.1, §2.2, §3, §9.

mod common;

use common::Fixture;
use serde_json::Value;

#[test]
fn two_matches_reported_in_path_order() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture.homebrew_linked_from("/usr/local", "python3", "3.12.1", &first_dir);
    fixture.homebrew_linked_from("/opt/homebrew", "python3", "3.13.0", &second_dir);

    fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("python3")
        .assert()
        .success()
        .code(0)
        .stdout(
            "python3: homebrew formula python3 3.12.1\n\
             python3: homebrew formula python3 3.13.0\n",
        );
}

#[test]
fn all_json_is_an_array_in_path_order() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture.homebrew_linked_from("/usr/local", "python3", "3.12.1", &first_dir);
    fixture.homebrew_linked_from("/opt/homebrew", "python3", "3.13.0", &second_dir);

    let output = fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("--json")
        .arg("python3")
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(&stdout).unwrap();

    let array = value.as_array().unwrap();
    assert_eq!(array.len(), 2);
    assert_eq!(array[0]["version"], "3.12.1");
    assert_eq!(array[1]["version"], "3.13.0");
}

#[test]
fn one_unknown_among_matches_exits_1_but_reports_both() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture.homebrew_linked_from("/usr/local", "mystery", "1.0.0", &first_dir);
    let plain = fixture.plain_binary_in(&second_dir, "mystery");

    fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("mystery")
        .assert()
        .failure()
        .code(1)
        .stdout("mystery: homebrew formula mystery 1.0.0\n")
        .stderr(predicates::prelude::predicate::str::contains(format!(
            "could not determine install source of {}",
            plain.display()
        )));
}

#[test]
fn zero_matches_exits_2() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("nope")
        .assert()
        .failure()
        .code(2)
        .stdout("")
        .stderr(predicates::prelude::predicate::str::contains(
            "nope: command not found",
        ));
}

#[test]
fn same_file_via_two_path_dirs_is_reported_twice_no_dedup() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    // Both dirs symlink to the *same* real Homebrew binary.
    let real = fixture.root().join("usr/local/Cellar/jq/1.7.1/bin/jq");
    std::fs::create_dir_all(real.parent().unwrap()).unwrap();
    std::fs::write(&real, b"\x7fELF binary").unwrap();
    std::os::unix::fs::symlink(&real, first_dir.join("jq")).unwrap();
    std::os::unix::fs::symlink(&real, second_dir.join("jq")).unwrap();

    fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("jq")
        .assert()
        .success()
        .code(0)
        .stdout(
            "jq: homebrew formula jq 1.7.1\n\
             jq: homebrew formula jq 1.7.1\n",
        );
}

#[test]
fn all_detail_prints_consecutive_blocks() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture.plain_binary_in(&first_dir, "tool");
    fixture.homebrew_linked_from("/usr/local", "tool", "9.9.9", &second_dir);

    let output = fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("--all")
        .arg("--detail")
        .arg("tool")
        .assert()
        .failure()
        .code(1);
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.starts_with("tool: unknown\n"));
    assert!(stdout.contains("tool: homebrew formula tool 9.9.9\n"));
    // Two blocks, each with their own path:/symlink:/shebang: lines.
    assert_eq!(stdout.matches("  path:").count(), 2);
}

#[test]
fn without_all_only_first_match_is_reported() {
    let fixture = Fixture::new();
    let first_dir = fixture.extra_dir("first");
    let second_dir = fixture.extra_dir("second");

    fixture.homebrew_linked_from("/usr/local", "python3", "3.12.1", &first_dir);
    fixture.homebrew_linked_from("/opt/homebrew", "python3", "3.13.0", &second_dir);

    fixture
        .command_with_path(&[&first_dir, &second_dir])
        .arg("python3")
        .assert()
        .success()
        .code(0)
        .stdout("python3: homebrew formula python3 3.12.1\n");
}
