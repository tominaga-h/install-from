# install-from

[日本語](README.ja.md)

A macOS CLI that answers one question: **"Where did this command come from?"**
— i.e. which package manager or mechanism placed the binary on disk.

```
$ install-from jq
jq: homebrew formula jq 1.7.1

$ install-from git
git: macos-system (shell command)

$ install-from node
node: version-manager asdf shim

$ install-from black
black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)
```

## Why

On a long-lived macOS machine, the same command name can end up installed by
Homebrew, a version manager (asdf/mise/nvm/pyenv/rbenv/volta), npm/yarn/pnpm,
uv, pipx, RubyGems, cargo, go install, a pip script, Xcode Command Line
Tools, or the OS itself. `install-from` inspects the file on disk — its
location and symlink chain, no subprocess spawned — and reports which one
placed it.

## Install

```bash
cargo install install-from
```

## Usage

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

### Examples

```
$ install-from jq --detail
jq: homebrew formula jq 1.7.1
  path:    /usr/local/bin/jq
  symlink: /usr/local/bin/jq -> ../Cellar/jq/1.7.1/bin/jq -> /usr/local/Cellar/jq/1.7.1/bin/jq
  shebang: (binary)

$ install-from jq --json
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

### Exit codes

| code | meaning |
| ---: | --- |
| 0 | source determined |
| 1 | target exists but source could not be determined (`unknown`); also symlink loop |
| 2 | command not found / path missing / is a directory / unreadable |
| 3 | usage / argument error |

## Detection sources

Checked in priority order against every hop of the symlink chain; the first
match wins:

1. `macos-system` — OS-bundled binaries (`/bin`, `/sbin`, `/usr/bin`, `/usr/sbin`)
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

See [SPEC.md](SPEC.md) for the full specification, including field
contracts, edge cases, and the exact matching rules for each detector.

## Scope

- Platform: macOS only (Intel and Apple Silicon).
- No external commands are ever spawned (`stat` / `readlink` / read-first-line
  only).
- Package name / version are derived **only from path components**, never
  from package metadata files.
- Linux/Windows support, `.pkg` receipts, code-signing info, and resolving a
  version-manager shim to its real binary are out of scope for now (see
  [SPEC.md §10](SPEC.md#10-future-not-mvp)).

## Development

```bash
make build   # cargo build
make check   # fmt --check, clippy -D warnings, test (pre-push checks)
make release # cargo build --release
```

CI runs on `macos-latest` via GitHub Actions: `cargo fmt --check`,
`cargo clippy -- -D warnings`, `cargo test`.

## License

MIT OR Apache-2.0
