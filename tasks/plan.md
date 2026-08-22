# Implementation Plan: install-from (MVP)

Source of truth: [SPEC.md](../SPEC.md). Section references (§N) point there.

## Overview

Build a macOS-only Rust CLI that, given one command name or path, reports
which mechanism placed the binary (macos-system / xcode-clt /
version-manager / homebrew / npm / uv / pipx / gem / cargo / go / pip /
unknown). Pure path analysis, no subprocesses, sequential detectors in
priority order, text / `--detail` / `--json` output, exit codes 0–3.

## Architecture Decisions

- **Four modules, one crate, one binary**: `resolve` (arg → paths, symlink
  chain, shebang), `detect` (`Env` + `Detector` trait + one detector per
  source), `output` (text/detail/json), `main` (clap + exit mapping). Matches
  §7; keeps each task inside one module.
- **`Env` is the only seam for testability.** All base directories
  (`home`, `cargo_home`, `gobin`, `asdf_data_dir`, …) are resolved once from
  env vars into a plain struct. Detectors take `&Env`; tests construct `Env`
  pointing into a tempdir. No filesystem abstraction / mocking beyond that.
- **`Finding` is a single struct** `{source: Source, package, version,
  detail: serde_json::Map}` — `Source` is an enum serialized to the §4
  strings. Text rendering lives in `output`, not in detectors.
- **Detectors get the full hop list**, not a single path, so the "try every
  hop" rule (§5) is implemented once in the driver, and each detector is a
  pure `fn(&Env, &Target) -> Option<Finding>`.
- **Vertical slice first**: Task 4 ships an end-to-end working binary with
  only two detectors (`macos-system`, `homebrew`). Every later detector is an
  S-sized additive task with the same shape, safe to parallelize.
- **Integration tests use a fixture-tree builder** (`tests/common/mod.rs`)
  that creates `Cellar/…`, `node_modules/…`, shims etc. under a tempdir and
  runs the binary with `PATH`/`HOME`/`*_HOME` overridden (§8).

## Dependency Graph

```
T1 scaffold (Cargo, clap skeleton, CI)
 └─ T2 resolve: PATH walk + symlink chain + shebang
     └─ T3 detect core: Env, Source, Finding, Detector trait, driver
         └─ T4 slice: macos-system + homebrew + text output + exit codes  ← first working binary
             ├─ T5 output: --detail and --json
             ├─ T6 xcode-clt + version-manager
             ├─ T7 npm + gem
             ├─ T8 uv + pipx
             ├─ T9 cargo + go
             ├─ T10 pip (shebang)           (needs T2 shebang)
             └─ T11 --all
                 └─ T12 integration test matrix (assert_cmd)
                     └─ T13 release workflow + README + LICENSE
```

T6–T10 are independent of each other and of T5/T11; they can run in
parallel sessions after T4.

---

## Task List

### Phase 1: Foundation

- [ ] [Task 1: Scaffold crate, CLI skeleton, CI](tasks.md#task-1-scaffold-crate-cli-skeleton-ci)
- [ ] [Task 2: `resolve` module — PATH lookup, symlink chain, shebang](tasks.md#task-2-resolve-module-path-lookup-symlink-chain-shebang)
- [ ] [Task 3: `detect` core — `Env`, `Source`, `Finding`, `Detector`, driver](tasks.md#task-3-detect-core-env-source-finding-detector-driver)
- [ ] [Task 4: First vertical slice — `macos-system` + `homebrew` + text output + exit codes](tasks.md#task-4-first-vertical-slice-macos-system-homebrew-text-output-exit-codes)

### Checkpoint: Foundation (after Tasks 1–4)

- [ ] `cargo fmt --check && cargo clippy -- -D warnings && cargo test` clean
- [ ] CI green on GitHub
- [ ] `install-from jq`, `install-from ls`, `install-from git` give correct lines on a real Mac
- [ ] Review with human before Phase 2

---

### Phase 2: Output modes and remaining detectors

- [ ] [Task 5: `--detail` and `--json` output](tasks.md#task-5-detail-and-json-output)
- [ ] [Task 6: `xcode-clt` + `version-manager` detectors](tasks.md#task-6-xcode-clt-version-manager-detectors)
- [ ] [Task 7: `npm` + `gem` detectors](tasks.md#task-7-npm-gem-detectors)
- [ ] [Task 8: `uv` + `pipx` detectors](tasks.md#task-8-uv-pipx-detectors)
- [ ] [Task 9: `cargo` + `go` detectors](tasks.md#task-9-cargo-go-detectors)
- [ ] [Task 10: `pip` (shebang) detector](tasks.md#task-10-pip-shebang-detector)
- [ ] [Task 11: `--all`](tasks.md#task-11-all)

### Checkpoint: Core complete (after Tasks 5–11)

- [ ] Every §4 source has at least one passing unit test and one CLI test
- [ ] `cargo clippy -- -D warnings` clean; CI green
- [ ] On this Mac: `jq`, `ls`, `git`, `node`, `aws` (→ unknown, exit 1), `--all python3`, `--json jq` all behave per spec
- [ ] Review with human before Phase 3

---

### Phase 3: Hardening and release

- [ ] [Task 12: Integration test matrix + performance guard](tasks.md#task-12-integration-test-matrix-performance-guard)
- [ ] [Task 13: Release workflow, README, LICENSE, crates.io metadata](tasks.md#task-13-release-workflow-readme-license-crates-io-metadata)

### Checkpoint: Complete

- [ ] All §9 acceptance criteria 1–10 checked off with evidence
- [ ] CHANGELOG / first tag `v0.1.0` ready
- [ ] Human sign-off

---

## Parallelization

| Group | Tasks | Notes |
| --- | --- | --- |
| Sequential spine | 1 → 2 → 3 → 4 | Each builds on the previous API |
| Parallel after 4 | 5, 6, 7, 8, 9, 10 | Each adds one or two files under `src/detect/` or `src/output.rs`; only shared touch point is the `PRIORITY_ORDER` list in `src/detect/mod.rs` — merge conflicts are one-line |
| After 5 | 11 | Needs JSON array rendering |
| After all | 12 → 13 | |

## Risks and Mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Symlink chain display: relative vs absolute semantics get muddled between matching and rendering | Med | Task 2 stores both `hops_display: Vec<String>` (verbatim) and `hops_abs: Vec<PathBuf>` (absolutized); detectors only ever see `hops_abs` |
| `std::fs::canonicalize` fails on dangling links, so "resolved_path = last target string" needs a manual walk | Med | Task 2 implements its own hop-by-hop walk with `read_link` and never calls `canonicalize` |
| Detector priority bugs (e.g. pip script under Cellar reported as pip) | High | Task 10 and Task 12 include explicit priority tests; driver order is a single const array |
| macOS-specific paths (`/Library/Application Support/pipx`, Xcode app bundle globs) hard to fixture | Low | Fixture builder uses `Env` overrides so tests never touch real system paths |
| CI runner lacks `$HOME` quirks (HOME unset case) | Low | Unit-test `Env::from_map` with `HOME` absent; don't rely on runner env |
| `--all` exit aggregation ambiguous when mixed chain errors and unknowns | Low | Spec says 1 for any non-determined; implement as `max(code)` over per-target codes excluding 2 |
| Non-UTF-8 paths | Low | Use `to_string_lossy` everywhere at output boundary only; internal types stay `PathBuf` |

## Open Questions

None blocking. Decisions already settled in SPEC.md §1–§7. Two choices
deferred to implementation time and to be recorded in code comments:

- Exact gem name/version split for names containing `-` and versions with
  platform suffixes (e.g. `nokogiri-1.16.0-x86_64-darwin`): plan splits on the
  first `-` followed by a digit.
- Whether `detail.prefix` for Homebrew should be the symlink's prefix or the
  Cellar's prefix when they differ (custom prefix + opt link): plan uses the
  hop that matched.
