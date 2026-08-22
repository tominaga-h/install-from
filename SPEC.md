# install-from — Specification (MVP)

`install-from` is a macOS CLI that answers one question:
**"Where did this command come from?"** — i.e. which package manager or
mechanism placed the binary on disk.

Written in Rust. Distributed as an OSS tool (crates.io + GitHub Releases,
Homebrew tap later). Must work on *other people's* Macs, not just the
author's.

---

## 1. Scope

### In scope (MVP)

- Platform: **macOS only** (Intel and Apple Silicon).
- Input: **one command name or path per invocation**.
- Detection sources (in priority order, see §5):
  1. `macos-system` — OS-bundled binaries
  2. `xcode-clt` — Xcode / Command Line Tools
  3. `version-manager` — shims of asdf / mise / nvm / pyenv / rbenv / volta
  4. `homebrew` — formula or cask
  5. `npm` — npm / yarn / pnpm global installs
  6. `uv` — `uv tool install`
  7. `pipx`
  8. `gem` — RubyGems
  9. `cargo` — `cargo install`
  10. `go` — `go install`
  11. `pip` — script whose shebang is a Python interpreter
  12. `unknown` — nothing matched
- Output: one-line text (default), `--detail` multi-line text, `--json`.
- Resolve command name via PATH in-process; `--all` to evaluate every PATH
  match.

### Explicitly out of scope (MVP)

- Linux / Windows.
- `.pkg` receipts (`pkgutil`), code-signing info, quarantine xattrs,
  Go buildinfo, or any other content-based heuristic for unplaced binaries.
- Resolving a version-manager shim to the real binary.
- Reading package metadata files (package.json, METADATA, .crates.toml, …).
  Package name / version come **only from path components**.
- Executing any external command (`brew`, `python`, `which`, …). Never.
- Config file. Paths are hard-coded + environment-variable overrides.
- Scanning an entire directory.
- i18n. All output is English.

---

## 2. CLI

```
install-from [OPTIONS] <COMMAND>

Arguments:
  <COMMAND>   Command name (searched in $PATH) or a path (contains '/')

Options:
  --all       Evaluate every match on $PATH, not only the first
  --detail    Print path / symlink chain / shebang below the result line
  --json      Emit JSON (always includes detail fields)
  -h, --help
  -V, --version
```

### 2.1 Argument interpretation

- Contains `/` → treated as a path. Relative paths are resolved against cwd.
- Otherwise → searched in `$PATH` **in-process** (no `which` spawn).
  Default: first match only. `--all`: every match, in PATH order.
- Matching a PATH entry requires the file to exist and be a regular file or
  symlink. Execute permission is **not** required.

### 2.2 Exit codes

| code | meaning |
| -----: | --------- |
| 0 | source determined for the (first) target |
| 1 | target exists but source could not be determined (`unknown`); also symlink loop / >40 hops |
| 2 | command not found on PATH, path does not exist, is a directory, or unreadable |
| 3 | usage / argument error |

With `--all`: exit 0 if **every** evaluated path was determined, else 1;
exit 2 only if there were zero matches.

### 2.3 stdout / stderr

- Result lines → stdout.
- Error messages → stderr.
- On exit 1 with `--detail` or `--json`, the detail block / JSON object is
  **still printed to stdout** (with `source: "unknown"`), and the error
  message goes to stderr.

---

## 3. Output formats

### 3.1 Default (one line)

```
<name>: <source-description>
```

Examples:

```
jq: homebrew formula jq 1.7.1
black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)
node: version-manager asdf shim
ls: macos-system
git: macos-system (shell command)
prettier: npm prettier
ruff: uv ruff
rg: cargo
gopls: go
bundler: gem bundler 2.5.6
```

Unknown (exit 1, stderr):

```
install-from: could not determine install source of /usr/local/bin/slim
```

### 3.2 `--detail`

```
jq: homebrew formula jq 1.7.1
  path:    /usr/local/bin/jq
  symlink: /usr/local/bin/jq -> ../Cellar/jq/1.7.1/bin/jq -> /usr/local/Cellar/jq/1.7.1/bin/jq
  shebang: (binary)
```

```
black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)
  path:    /usr/local/bin/black
  symlink: (none)
  shebang: #!/usr/local/opt/python@3.12/bin/python3.12
```

- `symlink:` lists each hop as written on disk (relative links shown
  relative), followed by the final canonicalized absolute path. `(none)` if
  not a symlink. A dangling link appends `(dangling)`.
- `shebang:` first line if the file starts with `#!`; `(binary)` if it
  doesn't; `(unreadable)` if the file can't be opened.
- With `--all`, blocks are printed consecutively in PATH order.

### 3.3 `--json`

Single object (or array with `--all`). `--detail --json` ≡ `--json`.

```json
{
  "name": "jq",
  "path": "/usr/local/bin/jq",
  "resolved_path": "/usr/local/Cellar/jq/1.7.1/bin/jq",
  "symlink_chain": ["/usr/local/bin/jq", "../Cellar/jq/1.7.1/bin/jq"],
  "dangling": false,
  "shebang": null,
  "source": "homebrew",
  "package": "jq",
  "version": "1.7.1",
  "detail": { "kind": "formula", "prefix": "/usr/local" }
}
```

Field contract:

| field | type | notes |
| --- | --- | --- |
| `name` | string | argument as given (basename if a path was given) |
| `path` | string | absolute path before following symlinks |
| `resolved_path` | string | canonical path; equals `path` if not a symlink; last target string if dangling |
| `symlink_chain` | string[] | `[path, target1, target2, …]` as written on disk; `[]` if not a symlink |
| `dangling` | bool | |
| `shebang` | string \| null | full first line incl. `#!`, or null |
| `source` | enum | see §4 |
| `package` | string \| null | from path only |
| `version` | string \| null | from path only |
| `detail` | object | source-specific keys, see §4 |

---

## 4. `source` values and `detail` keys

| source | package / version | detail |
| --- | --- | --- |
| `macos-system` | null | `{"shell_command": bool}` — true when under `/usr/bin` |
| `xcode-clt` | null | `{"root": "/Library/Developer/CommandLineTools" \| "/Applications/Xcode.app/…"}` |
| `version-manager` | null | `{"manager": "asdf"\|"mise"\|"nvm"\|"pyenv"\|"rbenv"\|"volta"}` |
| `homebrew` | name / version from `Cellar/<name>/<ver>` ; cask: name from `Caskroom/<name>/<ver>` | `{"kind": "formula"\|"cask", "prefix": "..."}` |
| `npm` | name from `node_modules/<pkg>` or `node_modules/@scope/<pkg>` ; version null | `{"client": "npm"\|"yarn"\|"pnpm"}` |
| `uv` | name from `tools/<name>` ; version null | `{}` |
| `pipx` | name from `venvs/<name>` ; version null | `{}` |
| `gem` | name / version from `gems/<name>-<ver>` | `{}` |
| `cargo` | null | `{"cargo_home": "..."}` |
| `go` | null | `{"gobin": "..."}` |
| `pip` | null | `{"python": "<interpreter from shebang>"}` |
| `unknown` | null | `{}` |

Text rendering of each source:

```
macos-system                      | macos-system (shell command)
xcode-clt
version-manager <manager> shim
homebrew formula <name> <ver>     | homebrew cask <name> <ver>
npm <pkg>                          (client name substituted if yarn/pnpm)
uv <name> | pipx <name>
gem <name> <ver>
cargo | go
pip (python: <interpreter>)
```

---

## 5. Detection algorithm

```
resolve argument → list of candidate paths (1, or N with --all)
for each path:
    build symlink chain (max 40 hops; loop ⇒ exit 1)
    hops = [path, hop1, hop2, …, canonical]   (strings, absolute-ized per hop)
    for detector in PRIORITY_ORDER:
        for hop in hops:                      # every hop is tried
            if detector.matches(hop, file_info) ⇒ result; stop
    ⇒ unknown
```

Rules:

- **"Whoever placed the file wins."** Detection is by the location of the
  symlink target / real file, not by the interpreter. A pip-installed script
  whose shebang points at a Homebrew Python is `pip`, not `homebrew`.
- Every hop of the symlink chain is tested so that `opt/` links and
  two-stage links inside Cellar are handled.
- A dangling symlink is still classified by its target string.
- Only the **first matching detector** in priority order is reported.
- No external process is ever spawned. Only `stat`, `readlink`, and reading
  the first line of the file (for shebang).

### 5.1 Detector definitions

Environment variables are read once; `~` means `$HOME`. `$XDG_DATA_HOME`
defaults to `~/.local/share`.

| # | detector | matches when hop path… |
| --- | --- | --- |
| 1 | macos-system | is directly under `/bin`, `/sbin`, `/usr/bin`, `/usr/sbin` (`shell_command = under /usr/bin`) |
| 2 | xcode-clt | starts with `/Library/Developer/CommandLineTools/` or `/Applications/Xcode*.app/Contents/Developer/` |
| 3 | version-manager | starts with one of: `$ASDF_DATA_DIR`(`~/.asdf`)`/shims/`, `$MISE_DATA_DIR`(`$XDG_DATA_HOME/mise`)`/shims/`, `$NVM_DIR`(`~/.nvm`)`/versions/node/*/bin/` **and the target is not inside `node_modules`**, `$PYENV_ROOT`(`~/.pyenv`)`/shims/`, `$RBENV_ROOT`(`~/.rbenv`)`/shims/`, `$VOLTA_HOME`(`~/.volta`)`/bin/` |
| 4 | homebrew | contains `/Cellar/<name>/<ver>/` or `/Caskroom/<name>/<ver>/`. Prefix = path before `/Cellar`/`/Caskroom`. Accept any prefix (covers `/usr/local`, `/opt/homebrew`, `$HOMEBREW_PREFIX`, custom) |
| 5 | npm | contains `/node_modules/<pkg>/` (handles `@scope/<pkg>`). Client: `yarn` if path contains `/.yarn/` or `/yarn/global/`; `pnpm` if contains `/pnpm/`; else `npm` |
| 6 | uv | starts with `$UV_TOOL_DIR`(`$XDG_DATA_HOME/uv/tools`)`/<name>/` |
| 7 | pipx | starts with `$PIPX_HOME`(`~/.local/pipx` or `~/Library/Application Support/pipx`)`/venvs/<name>/` |
| 8 | gem | contains `/gems/<name>-<ver>/` or `/gems/<ver>/bin/` (RubyGems binstub dir) |
| 9 | cargo | is directly under `$CARGO_HOME`(`~/.cargo`)`/bin/` |
| 10 | go | is directly under `$GOBIN` or `$GOPATH`(`~/go`)`/bin/` |
| 11 | pip | file (final hop) is a regular file whose shebang interpreter basename matches `python`, `python3`, `python3.N`, or is `env` with first arg matching the same |
| 12 | unknown | — |

Notes:

- nvm's `versions/node/vX/bin/<cmd>` is typically a symlink into
  `../lib/node_modules/<pkg>`; by the "whoever placed it" rule that resolves
  to `npm`. Only `node`/`npm`/`npx`/`corepack` themselves (real files) are
  reported as `version-manager nvm`.
- `<ver>` components are taken verbatim (e.g. `7.1.2-18_1`, `1.7.1`).

### 5.2 Shebang parsing

- Read only if the first two bytes are `#!`. Read at most the first line
  (cap 4 KiB).
- Interpreter = first whitespace-separated token after `#!`. If its basename
  is `env`, interpreter = next token (skipping `-S` if present).

---

## 6. Edge cases

| situation | behaviour |
| --- | --- |
| Not on PATH / path missing | exit 2, `install-from: <name>: command not found` / `no such file` |
| Path is a directory | exit 2 |
| Unreadable file (EACCES) | still classified by path; `shebang: (unreadable)` |
| Broken symlink | classify by target string; `dangling: true`; `(dangling)` in detail |
| Symlink loop or > 40 hops | exit 1, `symlink chain too deep or cyclic` |
| Multiple PATH matches | first only unless `--all` |
| Same file via several PATH dirs (`--all`) | reported once per PATH entry (no dedup) |
| Non-UTF-8 path | print lossily; JSON uses lossy string |
| `$HOME` unset | user-home detectors are skipped |

---

## 7. Implementation constraints

- Language: Rust, edition 2024, MSRV = current stable at first release.
- Dependencies: `clap` (derive), `serde` + `serde_json`, `anyhow`,
  `thiserror`. Dev: `tempfile`, `assert_cmd`, `predicates`.
- **No tokio / no async / no threads.** Detectors run sequentially in
  priority order and stop at first match.
- Architecture:
  - `resolve` — argument → candidate paths (PATH walk), symlink chain builder
  - `detect` — `trait Detector { fn detect(&self, ctx: &Ctx) -> Option<Finding> }`
    with one struct per source; an `Env` struct holds all resolved base
    directories so tests can inject them
  - `output` — text / detail / JSON renderers
  - `main` — clap, exit-code mapping
- Binary name: `install-from`. Crate name: `install-from` (verified unused on
  crates.io, 2026-08-22). License: MIT OR Apache-2.0.

---

## 8. Testing

- Unit tests per detector: build a fake tree in `tempfile::tempdir()`
  (e.g. `<tmp>/Cellar/jq/1.7.1/bin/jq` + symlink `<tmp>/bin/jq`), inject
  `Env` with `home`/`cargo_home`/… pointing into the tempdir. No dependency
  on the real machine's Homebrew or PATH.
- Integration tests with `assert_cmd`: run the binary with `PATH` and `HOME`
  overridden to the tempdir; assert stdout, stderr, exit code for every
  source and every exit code; assert JSON shape.
- CI: GitHub Actions `macos-latest` — `cargo fmt --check`, `cargo clippy
  -D warnings`, `cargo test`. On `v*` tag: build `x86_64-apple-darwin` and
  `aarch64-apple-darwin`, attach to GitHub Release, `cargo publish`.

---

## 9. Acceptance criteria

1. `install-from jq` on a Mac with Homebrew jq prints
   `jq: homebrew formula jq <ver>` and exits 0, on both `/usr/local` and
   `/opt/homebrew` prefixes.
2. `install-from ls` → `ls: macos-system`, exit 0.
3. `install-from git` (CLT) → `git: macos-system (shell command)`, exit 0.
4. `install-from node` with asdf → `node: version-manager asdf shim`.
5. A hand-copied Mach-O in `/usr/local/bin` → stderr message, exit 1;
   with `--json`, stdout has `"source":"unknown"` and full path fields.
6. `install-from nonexistent-cmd` → exit 2.
7. `install-from --bogus` → exit 3.
8. `--all python3` with brew + system Python lists both in PATH order.
9. All of the above pass in CI on a clean `macos-latest` runner using
   fixture trees (no real Homebrew required).
10. Total runtime per invocation < 10 ms; zero subprocesses (verifiable with
    `dtruss`/`fs_usage` showing no `execve`).

---

## 10. Future (not MVP)

- Linux support (apt/dnf/pacman receipts, `/usr/local` = manual).
- `pkgutil` receipt lookup; code-signature Team ID; quarantine/WhereFroms
  URL; Go buildinfo module path.
- Resolve version-manager shims to the real binary.
- Read package metadata for versions (package.json, dist-info, .crates.toml).
- `brew info`-based tap detection.
- Directory scan mode.
- Config file for custom path patterns.
