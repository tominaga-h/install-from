//! `--detail` and `--json` output, end to end. SPEC.md §3.2, §3.3, §9.

mod common;

use common::Fixture;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn detail_homebrew_symlink_is_byte_exact() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");

    let bin_dir = fixture.bin_dir();
    let root = fixture.root();
    let expected = format!(
        "jq: homebrew formula jq 1.7.1\n  path:    {}/jq\n  symlink: {}/jq -> ../usr/local/Cellar/jq/1.7.1/bin/jq -> {}/usr/local/Cellar/jq/1.7.1/bin/jq\n  shebang: (binary)\n",
        bin_dir.display(),
        bin_dir.display(),
        root.display(),
    );

    fixture
        .command()
        .arg("--detail")
        .arg("jq")
        .assert()
        .success()
        .code(0)
        .stdout(expected);
}

#[test]
fn detail_macos_system_shows_none_and_binary() {
    let fixture = Fixture::new();

    fixture
        .command()
        .arg("--detail")
        .arg("/bin/ls")
        .assert()
        .success()
        .stdout("ls: macos-system\n  path:    /bin/ls\n  symlink: (none)\n  shebang: (binary)\n");
}

#[test]
fn detail_unknown_still_prints_to_stdout_and_exits_1() {
    let fixture = Fixture::new();
    let path = fixture.plain_binary("slim");

    fixture
        .command()
        .arg("--detail")
        .arg("slim")
        .assert()
        .failure()
        .code(1)
        .stdout(predicate::str::starts_with("slim: unknown\n"))
        .stdout(predicate::str::contains(format!(
            "path:    {}",
            path.display()
        )))
        .stderr(predicate::str::contains(
            "could not determine install source of",
        ));
}

#[test]
fn json_homebrew_has_all_ten_fields() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");

    let output = fixture
        .command()
        .arg("--json")
        .arg("jq")
        .assert()
        .success()
        .code(0);
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(&stdout).unwrap();

    let bin_dir = fixture.bin_dir();
    let root = fixture.root();
    assert_eq!(value["name"], "jq");
    assert_eq!(value["path"], format!("{}/jq", bin_dir.display()));
    assert_eq!(
        value["resolved_path"],
        format!("{}/usr/local/Cellar/jq/1.7.1/bin/jq", root.display())
    );
    assert_eq!(
        value["symlink_chain"],
        serde_json::json!([
            format!("{}/jq", bin_dir.display()),
            "../usr/local/Cellar/jq/1.7.1/bin/jq"
        ])
    );
    assert_eq!(value["dangling"], false);
    assert_eq!(value["shebang"], Value::Null);
    assert_eq!(value["source"], "homebrew");
    assert_eq!(value["package"], "jq");
    assert_eq!(value["version"], "1.7.1");
    assert_eq!(value["detail"]["kind"], "formula");
    assert_eq!(
        value["detail"]["prefix"],
        format!("{}/usr/local", root.display())
    );

    // exactly ten top-level fields, per the field contract table
    assert_eq!(value.as_object().unwrap().len(), 10);
}

#[test]
fn json_non_symlink_has_empty_chain_and_null_shebang() {
    let fixture = Fixture::new();

    let output = fixture
        .command()
        .arg("--json")
        .arg("/bin/ls")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["symlink_chain"], serde_json::json!([]));
    assert_eq!(value["shebang"], Value::Null);
    assert_eq!(value["source"], "macos-system");
}

#[test]
fn json_unknown_prints_to_stdout_with_source_unknown_and_exits_1() {
    let fixture = Fixture::new();
    fixture.plain_binary("slim");

    let output = fixture
        .command()
        .arg("--json")
        .arg("slim")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "could not determine install source of",
        ));
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["source"], "unknown");
    assert_eq!(value["package"], Value::Null);
    assert_eq!(value["version"], Value::Null);
    assert_eq!(value["detail"], serde_json::json!({}));
}

#[test]
fn detail_and_json_together_behaves_like_json_only() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");

    let with_both = fixture
        .command()
        .arg("--detail")
        .arg("--json")
        .arg("jq")
        .assert()
        .success();
    let json_only = fixture.command().arg("--json").arg("jq").assert().success();

    assert_eq!(with_both.get_output().stdout, json_only.get_output().stdout);
}
