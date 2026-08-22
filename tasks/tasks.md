# Tasks: install-from (MVP)

Plan and phase structure: [PLAN.md](PLAN.md). Spec: [SPEC.md](../SPEC.md) (§N refers there).

## Index

| # | Task | Phase | Deps | Scope |
|---|------|-------|------|-------|
| 1 | [Scaffold crate, CLI skeleton, CI](#task-1-scaffold-crate-cli-skeleton-ci) | 1 Foundation | None | S |
| 2 | [`resolve` module — PATH lookup, symlink chain, shebang](#task-2-resolve-module-path-lookup-symlink-chain-shebang) | 1 Foundation | Task 1 | M |
| 3 | [`detect` core — `Env`, `Source`, `Finding`, `Detector`, driver](#task-3-detect-core-env-source-finding-detector-driver) | 1 Foundation | Task 2 | M |
| 4 | [First vertical slice — `macos-system` + `homebrew` + text output + exit codes](#task-4-first-vertical-slice-macos-system-homebrew-text-output-exit-codes) | 1 Foundation | Task 3 | M |
| 5 | [`--detail` and `--json` output](#task-5-detail-and-json-output) | 2 Core | Task 4 | S |
| 6 | [`xcode-clt` + `version-manager` detectors](#task-6-xcode-clt-version-manager-detectors) | 2 Core | Task 4 | S |
| 7 | [`npm` + `gem` detectors](#task-7-npm-gem-detectors) | 2 Core | Task 4 | S |
| 8 | [`uv` + `pipx` detectors](#task-8-uv-pipx-detectors) | 2 Core | Task 4 | S |
| 9 | [`cargo` + `go` detectors](#task-9-cargo-go-detectors) | 2 Core | Task 4 | S |
| 10 | [`pip` (shebang) detector](#task-10-pip-shebang-detector) | 2 Core | Task 4 (uses `Target.shebang` from Task 2) | S |
| 11 | [`--all`](#task-11-all) | 2 Core | Task 5 | S |
| 12 | [Integration test matrix + performance guard](#task-12-integration-test-matrix-performance-guard) | 3 Release | Tasks 5–11 | M |
| 13 | [Release workflow, README, LICENSE, crates.io metadata](#task-13-release-workflow-readme-license-crates-io-metadata) | 3 Release | Task 12 | S |

---

## Task 1: Scaffold crate, CLI skeleton, CI

**Description:** `cargo init` the binary crate `install-from` (edition 2024,
MIT OR Apache-2.0), add dependencies from §7, define the clap `Args` struct
(`command`, `--all`, `--detail`, `--json`), wire exit code 3 for usage
errors, and add a GitHub Actions workflow running fmt/clippy/test on
`macos-latest`. The binary does nothing useful yet but parses args.

**Acceptance criteria:**

- [ ] `cargo build` succeeds with `clap`, `serde`, `serde_json`, `anyhow`, `thiserror` as deps; `tempfile`, `assert_cmd`, `predicates` as dev-deps
- [ ] `install-from --bogus` exits 3; `install-from --help` and `-V` exit 0
- [ ] `.github/workflows/ci.yml` runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` on `macos-latest`

**Verification:**

- [ ] `cargo build && cargo clippy -- -D warnings`
- [ ] `target/debug/install-from --bogus; echo $?` → 3
- [ ] `tests/cli_usage.rs` (assert_cmd) asserts exit 3 / 0 — passes

**Dependencies:** None
**Files likely touched:** `Cargo.toml`, `src/main.rs`, `src/cli.rs`, `.github/workflows/ci.yml`, `tests/cli_usage.rs`, `.gitignore`
**Estimated scope:** S

---

## Task 2: `resolve` module — PATH lookup, symlink chain, shebang

**Description:** Implement §2.1, §5 chain building, §5.2, and the §6 edge
cases that belong to resolution. `resolve::candidates(arg, path_env, cwd,
all) -> Result<Vec<PathBuf>, ResolveError>` (contains `/` → single path,
else PATH walk; regular file or symlink, no exec bit required).
`resolve::chain(path) -> Result<Chain, ResolveError>` builds
`Target { path, hops: Vec<String>, resolved_path, dangling, shebang:
ShebangState }` with a 40-hop cap and loop detection; relative link targets
are absolutized per hop for matching but kept verbatim for display.
`shebang` reads at most 4 KiB, only if the first two bytes are `#!`, and
exposes `interpreter()` handling `env` / `env -S`.

**Acceptance criteria:**

- [ ] Name not on PATH / missing path / directory → `ResolveError::NotFound` (maps to exit 2 later)
- [ ] Symlink chain: hops recorded as written on disk; `resolved_path` canonical; dangling link yields `dangling: true` with last target string as `resolved_path`
- [ ] Loop (a→b→a) and >40 hops → `ResolveError::ChainTooDeep` (exit 1)
- [ ] Shebang: `#!/usr/bin/env -S python3 -u` → interpreter `python3`; non-`#!` file → `Binary`; EACCES → `Unreadable`

**Verification:**

- [ ] Unit tests in `src/resolve.rs` using `tempfile` for: PATH first-match vs `--all` order, relative arg, dangling, loop, 41 hops, each shebang variant
- [ ] `cargo test resolve`

**Dependencies:** Task 1
**Files likely touched:** `src/resolve.rs`, `src/error.rs`, `src/main.rs`
**Estimated scope:** M

---

## Task 3: `detect` core — `Env`, `Source`, `Finding`, `Detector`, driver

**Description:** Define `Env` (all base dirs from §5.1 resolved from env vars
with documented defaults; user-home fields are `Option` and `None` when
`$HOME` unset), `Source` enum (serde rename to §4 strings), `Finding`,
`trait Detector { fn detect(&self, env: &Env, target: &Target) ->
Option<Finding>; }`, and `detect::run(env, target) -> Finding` that iterates
`PRIORITY_ORDER` and returns the first `Some`, else `Finding::unknown()`.
Ship with an empty detector list; detectors come in Tasks 4–10.

**Acceptance criteria:**

- [ ] `Env::from_env()` honors `HOMEBREW_PREFIX, CARGO_HOME, GOPATH, GOBIN, NVM_DIR, PYENV_ROOT, RBENV_ROOT, ASDF_DATA_DIR, MISE_DATA_DIR, PIPX_HOME, UV_TOOL_DIR, VOLTA_HOME, XDG_DATA_HOME, HOME` with §5.1 defaults
- [ ] `Source` serializes exactly to the twelve §4 strings
- [ ] Driver returns first match in declared order; with no detectors returns `unknown`

**Verification:**

- [ ] Unit tests: `Env` defaults vs overrides (use a `from_map(HashMap)` constructor, not global env mutation); serde round-trip of `Source`; driver ordering with two stub detectors
- [ ] `cargo test detect::`

**Dependencies:** Task 2
**Files likely touched:** `src/detect/mod.rs`, `src/detect/env.rs`, `src/detect/finding.rs`
**Estimated scope:** M

---

## Task 4: First vertical slice — `macos-system` + `homebrew` + text output + exit codes

**Description:** Implement the two highest-value detectors, the one-line text
renderer (§3.1), and the full `main` flow: resolve → chain → detect → print →
exit code (§2.2, §2.3). After this task the tool is genuinely useful on a
real Mac.

**Acceptance criteria:**

- [ ] `macos-system`: direct child of `/bin`, `/sbin`, `/usr/bin`, `/usr/sbin`; `detail.shell_command = true` only for `/usr/bin`; text `ls: macos-system` / `git: macos-system (shell command)`
- [ ] `homebrew`: any hop containing `/Cellar/<name>/<ver>/` or `/Caskroom/<name>/<ver>/`; `package`, `version`, `detail.kind`, `detail.prefix` set; text `jq: homebrew formula jq 1.7.1`
- [ ] Unknown → stderr `install-from: could not determine install source of <path>`, exit 1; NotFound → exit 2; ChainTooDeep → exit 1

**Verification:**

- [ ] Unit tests for both detectors against tempdir trees (`<tmp>/Cellar/jq/1.7.1/bin/jq` ← `<tmp>/bin/jq`, `opt/` two-stage link, Caskroom, `/opt/homebrew` and custom prefixes)
- [ ] `tests/cli_slice.rs`: assert_cmd with `PATH=<tmp>/bin` for homebrew fixture (exit 0, exact stdout); `install-from /bin/ls` → `ls: macos-system`; plain Mach-O in tmp → exit 1 + stderr; `install-from nope` → exit 2
- [ ] Manual: `cargo run -- jq` on this machine prints a homebrew line

**Dependencies:** Task 3
**Files likely touched:** `src/detect/macos_system.rs`, `src/detect/homebrew.rs`, `src/output.rs`, `src/main.rs`, `tests/common/mod.rs`, `tests/cli_slice.rs`
**Estimated scope:** M

---

## Task 5: `--detail` and `--json` output

**Description:** Implement §3.2 and §3.3 renderers. `--json` always includes
every field; `--detail --json` ≡ `--json`. On `unknown` (exit 1) the detail
block / JSON is still written to stdout with `source: "unknown"`.

**Acceptance criteria:**

- [ ] `--detail` prints the 3 indented lines exactly as §3.2 (`(none)`, `(binary)`, `(unreadable)`, `(dangling)` suffix, hops joined with ` -> ` ending in canonical path)
- [ ] `--json` emits the §3.3 object with all ten fields; `shebang` is the full first line or `null`; `symlink_chain` is `[]` for non-symlinks
- [ ] Unknown + `--json` → stdout JSON with `"source":"unknown"`, stderr message, exit 1

**Verification:**

- [ ] Unit tests in `src/output.rs` rendering fixed `Target`/`Finding` values (symlink, non-symlink, dangling, unreadable)
- [ ] `tests/cli_output.rs`: parse stdout with `serde_json` and assert field-by-field; byte-exact `--detail` comparison
- [ ] `cargo test output`

**Dependencies:** Task 4
**Files likely touched:** `src/output.rs`, `src/main.rs`, `tests/cli_output.rs`
**Estimated scope:** S

---

## Task 6: `xcode-clt` + `version-manager` detectors

**Description:** Add §5.1 rows 2 and 3. `version-manager` matches
asdf/mise/pyenv/rbenv `shims/`, volta `bin/`, and nvm
`versions/node/*/bin/` **only when no hop is inside `node_modules`** (so
nvm-installed global packages fall through to `npm`).

**Acceptance criteria:**

- [ ] `/Library/Developer/CommandLineTools/usr/bin/x` and `/Applications/Xcode-beta.app/Contents/Developer/usr/bin/x` → `xcode-clt` with `detail.root`
- [ ] Each of the six managers yields `detail.manager` and text `node: version-manager asdf shim`
- [ ] nvm `bin/node` (real file) → `version-manager nvm`; nvm `bin/prettier` → `../lib/node_modules/prettier/…` → **not** matched here

**Verification:**

- [ ] Unit tests with `Env` fields pointed at tempdir (`asdf_data_dir`, `nvm_dir`, …), including env-var override and `$HOME` unset (`None`) skip
- [ ] `cargo test detect::xcode detect::version_manager`

**Dependencies:** Task 4
**Files likely touched:** `src/detect/xcode_clt.rs`, `src/detect/version_manager.rs`, `src/detect/mod.rs`
**Estimated scope:** S

---

## Task 7: `npm` + `gem` detectors

**Description:** §5.1 rows 5 and 8. npm: `/node_modules/<pkg>/` and
`/node_modules/@scope/<pkg>/`, client from `/.yarn/`, `/yarn/global/`,
`/pnpm/`. gem: `/gems/<name>-<ver>/` (split on last `-`) and RubyGems
binstub `/gems/<ver>/bin/`.

**Acceptance criteria:**

- [ ] `…/lib/node_modules/prettier/bin/prettier.cjs` → `npm`, package `prettier`; `@angular/cli` → package `@angular/cli`; text `prettier: npm prettier` / `… yarn …` / `… pnpm …`
- [ ] `…/gems/bundler-2.5.6/exe/bundler` → `gem`, package `bundler`, version `2.5.6`; binstub `…/gems/3.3.0/bin/bundler` → `gem` with package/version `null`
- [ ] Version is always `null` for npm

**Verification:**

- [ ] Unit tests for scoped/unscoped npm, three clients, gem name containing hyphens (`rubocop-rails-2.24.0`), binstub dir
- [ ] `cargo test detect::npm detect::gem`

**Dependencies:** Task 4
**Files likely touched:** `src/detect/npm.rs`, `src/detect/gem.rs`, `src/detect/mod.rs`
**Estimated scope:** S

---

## Task 8: `uv` + `pipx` detectors

**Description:** §5.1 rows 6 and 7. uv: under `$UV_TOOL_DIR/<name>/`
(default `$XDG_DATA_HOME/uv/tools`). pipx: under `$PIPX_HOME/venvs/<name>/`
where default checks `~/.local/pipx` then `~/Library/Application
Support/pipx`.

**Acceptance criteria:**

- [ ] `~/.local/bin/ruff` → `~/.local/share/uv/tools/ruff/bin/ruff` → `uv`, package `ruff`, text `ruff: uv ruff`
- [ ] `~/.local/bin/black` → `~/.local/pipx/venvs/black/bin/black` → `pipx`, package `black`
- [ ] Both defaults and env overrides (`UV_TOOL_DIR`, `PIPX_HOME`, `XDG_DATA_HOME`) honored

**Verification:**

- [ ] Unit tests for default locations, both pipx defaults, overrides
- [ ] `cargo test detect::uv detect::pipx`

**Dependencies:** Task 4
**Files likely touched:** `src/detect/uv.rs`, `src/detect/pipx.rs`, `src/detect/mod.rs`
**Estimated scope:** S

---

## Task 9: `cargo` + `go` detectors

**Description:** §5.1 rows 9 and 10. Match only direct children of
`$CARGO_HOME/bin` and of `$GOBIN` (or `$GOPATH/bin`). Package/version null;
`detail.cargo_home` / `detail.gobin` set.

**Acceptance criteria:**

- [ ] `~/.cargo/bin/rg` → `cargo`, text `rg: cargo`; `CARGO_HOME` override honored
- [ ] `~/go/bin/gopls` → `go`, text `gopls: go`; `GOBIN` takes precedence over `GOPATH/bin`
- [ ] Nested paths (`~/.cargo/bin/sub/x`) do **not** match

**Verification:**

- [ ] Unit tests for defaults, overrides, precedence, nested-non-match
- [ ] `cargo test detect::cargo detect::go`

**Dependencies:** Task 4
**Files likely touched:** `src/detect/cargo.rs`, `src/detect/go.rs`, `src/detect/mod.rs`
**Estimated scope:** S

---

## Task 10: `pip` (shebang) detector

**Description:** §5.1 row 11, lowest priority before `unknown`. Final hop is
a regular file whose shebang interpreter basename matches
`python`, `python3`, or `python3.N` (directly or via `env` / `env -S`).
`detail.python` is the interpreter path token as written.

**Acceptance criteria:**

- [ ] `/usr/local/bin/black` with `#!/usr/local/opt/python@3.12/bin/python3.12` → `pip`, text `black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)`
- [ ] `#!/usr/bin/env python3` → `pip`, `detail.python = "python3"`
- [ ] A pip script living inside `Cellar/` is reported as `homebrew` (priority), a `#!/bin/bash` script is `unknown`

**Verification:**

- [ ] Unit tests for the three shebang forms, `python3.13`, non-python shebang, binary file, priority vs homebrew
- [ ] `cargo test detect::pip`

**Dependencies:** Task 4 (uses `Target.shebang` from Task 2)
**Files likely touched:** `src/detect/pip.rs`, `src/detect/mod.rs`
**Estimated scope:** S

---

## Task 11: `--all`

**Description:** Evaluate every PATH match in order (§2.1, §2.2). Text /
detail blocks printed consecutively; `--json` emits an array. Exit 0 only if
every target was determined, 1 if any was `unknown` / chain error, 2 only if
zero matches. No dedup across PATH entries.

**Acceptance criteria:**

- [ ] `--all python3` with `<tmp>/brew/bin/python3` and `<tmp>/usr/bin/python3` on PATH prints two lines in PATH order
- [ ] `--all --json` is a JSON array of §3.3 objects
- [ ] Exit-code aggregation as specified; chain error on one entry does not abort the others

**Verification:**

- [ ] `tests/cli_all.rs`: two-match, one-unknown (exit 1), zero-match (exit 2), same file via two dirs reported twice
- [ ] `cargo test cli_all`

**Dependencies:** Task 5
**Files likely touched:** `src/main.rs`, `src/output.rs`, `tests/cli_all.rs`
**Estimated scope:** S

---

## Task 12: Integration test matrix + performance guard

**Description:** Consolidate CLI coverage into a single fixture-tree builder
and a table-driven test covering every `source` × {text, detail, json} and
every exit code (§8, §9.1–9.8). Add a smoke check that the binary spawns no
subprocess (assert `std::process::Command` is not referenced in `src/` via a
test that greps the source tree) and completes `< 10 ms` in a debug-build
loop of 50 runs (soft assertion, warning only on CI).

**Acceptance criteria:**

- [ ] `tests/common/mod.rs` exposes `Fixture::new()` with helpers `homebrew(name, ver)`, `npm(pkg)`, `shim(manager, cmd)`, `cargo(cmd)`, `go(cmd)`, `uv(tool)`, `pipx(tool)`, `gem(name, ver)`, `pip_script(name, shebang)`, `plain_binary(name)` and an `env()` returning the PATH/HOME/override map
- [ ] Table-driven test hits all 12 sources in all 3 output modes; all 4 exit codes asserted
- [ ] Source-grep test fails if `process::Command` appears under `src/`

**Verification:**

- [ ] `cargo test` — all green locally and in CI on `macos-latest`
- [ ] Manual: `sudo fs_usage -w -f exec target/release/install-from jq` shows no `execve`

**Dependencies:** Tasks 5–11
**Files likely touched:** `tests/common/mod.rs`, `tests/matrix.rs`, `tests/no_subprocess.rs`; existing `tests/cli_*.rs` may be folded in
**Estimated scope:** M

---

## Task 13: Release workflow, README, LICENSE, crates.io metadata

**Description:** Add `release.yml` triggered on `v*` tags that builds
`x86_64-apple-darwin` and `aarch64-apple-darwin`, attaches tarballs to a
GitHub Release, and runs `cargo publish` (token from secrets). Write README
(usage, output examples from §3, source table from §4, exit codes, what is
*not* detected) and `LICENSE-MIT` / `LICENSE-APACHE`. Fill `Cargo.toml`
`description`, `repository`, `keywords`, `categories`.

**Acceptance criteria:**

- [ ] `cargo publish --dry-run` succeeds
- [ ] `release.yml` produces two artifacts on a tag push (verify with a `v0.0.0-test` tag on a fork or `act`, then delete)
- [ ] README documents every CLI flag, every source string, and exit codes 0–3

**Verification:**

- [ ] `cargo publish --dry-run`
- [ ] Workflow run visible and green in GitHub Actions for the test tag
- [ ] `cargo package --list` includes README and both LICENSE files

**Dependencies:** Task 12
**Files likely touched:** `.github/workflows/release.yml`, `README.md`, `LICENSE-MIT`, `LICENSE-APACHE`, `Cargo.toml`
**Estimated scope:** S
