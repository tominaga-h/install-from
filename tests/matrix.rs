//! Table-driven coverage: every `source` in all 3 output modes, and every
//! exit code. SPEC.md §4, §8, §9.1-9.8.
//!
//! Each individual source/detector already has focused unit tests
//! (`src/detect/*.rs`) and often its own integration test file
//! (`tests/cli_*.rs`); this file's job is breadth, not depth — one
//! end-to-end case per source confirming text, `--detail`, and `--json`
//! agree with each other and with SPEC.md §4, in a single table anyone
//! extending detection can extend alongside it.
//!
//! `macos-system` and `xcode-clt` are intentionally NOT in [`CASES`]: both
//! detectors require a hop whose *absolute* path is literally under
//! `/bin`, `/usr/bin`, `/Library/Developer/CommandLineTools`, etc — paths
//! that cannot be relocated under a tempdir fixture root. They already
//! have full unit coverage (`src/detect/macos_system.rs`,
//! `src/detect/xcode_clt.rs`) and end-to-end coverage against the real
//! filesystem in `tests/cli_slice.rs` (`/bin/ls`, `/usr/bin/git`).

mod common;

use common::Fixture;
use serde_json::Value;

struct Case {
    source: &'static str,
    /// Sets up the fixture and returns the command name to resolve.
    setup: fn(&Fixture) -> String,
    /// Expected default (one-line) stdout, without the trailing newline.
    expect_line: &'static str,
}

const CASES: &[Case] = &[
    Case {
        source: "version-manager",
        setup: |f| {
            f.shim("asdf", "toolasdf");
            "toolasdf".to_string()
        },
        expect_line: "toolasdf: version-manager asdf shim",
    },
    Case {
        source: "homebrew",
        setup: |f| {
            f.homebrew("/usr/local", "toolbrew", "1.0.0");
            "toolbrew".to_string()
        },
        expect_line: "toolbrew: homebrew formula toolbrew 1.0.0",
    },
    Case {
        source: "npm",
        setup: |f| {
            f.npm("toolnpm");
            "toolnpm".to_string()
        },
        expect_line: "toolnpm: npm toolnpm",
    },
    Case {
        source: "uv",
        setup: |f| {
            f.uv_tool("tooluv");
            "tooluv".to_string()
        },
        expect_line: "tooluv: uv tooluv",
    },
    Case {
        source: "pipx",
        setup: |f| {
            f.pipx_tool("toolpipx");
            "toolpipx".to_string()
        },
        expect_line: "toolpipx: pipx toolpipx",
    },
    Case {
        source: "gem",
        setup: |f| {
            f.gem("toolgem", "2.0.0");
            "toolgem".to_string()
        },
        expect_line: "toolgem: gem toolgem 2.0.0",
    },
    Case {
        source: "cargo",
        setup: |f| {
            f.cargo("toolcargo");
            "toolcargo".to_string()
        },
        expect_line: "toolcargo: cargo",
    },
    Case {
        source: "go",
        setup: |f| {
            f.go("toolgo");
            "toolgo".to_string()
        },
        expect_line: "toolgo: go",
    },
    Case {
        source: "pip",
        setup: |f| {
            f.pip_script("toolpip", "#!/usr/bin/env python3");
            "toolpip".to_string()
        },
        expect_line: "toolpip: pip (python: python3)",
    },
];

#[test]
fn every_source_matches_expected_default_text() {
    for case in CASES {
        let fixture = Fixture::new();
        let name = (case.setup)(&fixture);

        let output = fixture.command().arg(&name).assert().success().code(0);
        let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        assert_eq!(
            stdout.trim_end(),
            case.expect_line,
            "source {} produced unexpected default text",
            case.source
        );
    }
}

#[test]
fn every_source_json_agrees_with_default_text() {
    for case in CASES {
        let fixture = Fixture::new();
        let name = (case.setup)(&fixture);

        let output = fixture
            .command()
            .arg("--json")
            .arg(&name)
            .assert()
            .success()
            .code(0);
        let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
        let value: Value = serde_json::from_str(&stdout).unwrap();

        // Every field from the SPEC.md §3.3 contract is present.
        for field in [
            "name",
            "path",
            "resolved_path",
            "symlink_chain",
            "dangling",
            "shebang",
            "source",
            "package",
            "version",
            "detail",
        ] {
            assert!(
                value.get(field).is_some(),
                "source {} JSON missing field {field}",
                case.source
            );
        }

        assert_eq!(value["name"], name, "source {} name mismatch", case.source);
    }
}

#[test]
fn every_source_detail_starts_with_default_line_and_has_three_fields() {
    for case in CASES {
        let fixture = Fixture::new();
        let name = (case.setup)(&fixture);

        let output = fixture
            .command()
            .arg("--detail")
            .arg(&name)
            .assert()
            .success()
            .code(0);
        let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

        assert!(
            stdout.starts_with(&format!("{}\n", case.expect_line)),
            "source {} --detail did not start with the default line:\n{stdout}",
            case.source
        );
        assert!(stdout.contains("  path:"), "source {}", case.source);
        assert!(stdout.contains("  symlink:"), "source {}", case.source);
        assert!(stdout.contains("  shebang:"), "source {}", case.source);
    }
}

// ---- Exit codes: SPEC.md §2.2, §9.6-9.7 ----

#[test]
fn exit_0_when_source_is_determined() {
    let fixture = Fixture::new();
    fixture.homebrew("/usr/local", "jq", "1.7.1");
    fixture.command().arg("jq").assert().success().code(0);
}

#[test]
fn exit_1_when_unknown() {
    let fixture = Fixture::new();
    fixture.plain_binary("slim");
    fixture.command().arg("slim").assert().failure().code(1);
}

#[test]
fn exit_2_when_command_not_found() {
    let fixture = Fixture::new();
    fixture.command().arg("nope").assert().failure().code(2);
}

#[test]
fn exit_2_when_path_argument_does_not_exist() {
    let fixture = Fixture::new();
    fixture
        .command()
        .arg("./missing")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn exit_3_on_usage_error() {
    let fixture = Fixture::new();
    fixture.command().arg("--bogus").assert().failure().code(3);
}

#[test]
fn exit_1_on_symlink_loop() {
    let fixture = Fixture::new();
    let a = fixture.bin_dir().join("loop_a");
    let b = fixture.bin_dir().join("loop_b");
    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();

    fixture.command().arg("loop_a").assert().failure().code(1);
}
