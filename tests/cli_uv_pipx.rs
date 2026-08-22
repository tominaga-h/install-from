//! `uv` and `pipx` end to end. SPEC.md §5.1 rows 6-7, §9.

mod common;

use common::Fixture;

#[test]
fn uv_tool_via_default_location() {
    let fixture = Fixture::new();
    fixture.uv_tool("ruff");

    fixture
        .command()
        .arg("ruff")
        .assert()
        .success()
        .code(0)
        .stdout("ruff: uv ruff\n");
}

#[test]
fn pipx_tool_via_default_location() {
    let fixture = Fixture::new();
    fixture.pipx_tool("black");

    fixture
        .command()
        .arg("black")
        .assert()
        .success()
        .code(0)
        .stdout("black: pipx black\n");
}

#[test]
fn uv_json_has_null_version() {
    let fixture = Fixture::new();
    fixture.uv_tool("ruff");

    let output = fixture
        .command()
        .arg("--json")
        .arg("ruff")
        .assert()
        .success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let value: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(value["source"], "uv");
    assert_eq!(value["package"], "ruff");
    assert_eq!(value["version"], serde_json::Value::Null);
}
