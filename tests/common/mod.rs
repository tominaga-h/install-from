//! Fixture-tree builder for CLI integration tests. SPEC.md §8.
//!
//! Builds a fake `/`-like tree under a tempdir and returns the env vars
//! (`PATH`, `HOME`, ...) needed to point the binary at it, so tests never
//! depend on the real machine's Homebrew, PATH, or `$HOME`.
//!
//! Grows one helper per source as later tasks add coverage.

#![allow(dead_code)]

use assert_cmd::Command;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

pub struct Fixture {
    pub tmp: TempDir,
}

impl Fixture {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("bin")).unwrap();
        Fixture { tmp }
    }

    pub fn root(&self) -> &Path {
        self.tmp.path()
    }

    pub fn bin_dir(&self) -> PathBuf {
        self.root().join("bin")
    }

    fn write_file(&self, path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    /// A plain regular file on `bin/`, not matching any detector.
    pub fn plain_binary(&self, name: &str) -> PathBuf {
        self.plain_binary_in(&self.bin_dir(), name)
    }

    /// Like [`Fixture::plain_binary`], but written into an arbitrary
    /// directory (e.g. one returned by [`Fixture::extra_dir`]).
    pub fn plain_binary_in(&self, dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        self.write_file(&path, &[0x7f, b'E', b'L', b'F', 0, 0, 0, 0]);
        path
    }

    /// A Homebrew-formula-style install: the real file lives at
    /// `<root>/<prefix>/Cellar/<name>/<ver>/bin/<name>` (so the detector
    /// sees the given `prefix`, e.g. `/usr/local` or `/opt/homebrew`, as
    /// the path component before `Cellar`), symlinked from
    /// [`Fixture::bin_dir`] — the one directory always on `PATH` for tests
    /// run via [`Fixture::command`]. Returns the symlink path.
    pub fn homebrew(&self, prefix: &str, name: &str, version: &str) -> PathBuf {
        self.homebrew_linked_from(prefix, name, version, &self.bin_dir())
    }

    /// Like [`Fixture::homebrew`], but the symlink is placed in `link_dir`
    /// instead of [`Fixture::bin_dir`]. For `--all` tests that need the
    /// same command name on more than one `PATH` entry.
    pub fn homebrew_linked_from(
        &self,
        prefix: &str,
        name: &str,
        version: &str,
        link_dir: &Path,
    ) -> PathBuf {
        let prefix_dir = self.root().join(prefix.trim_start_matches('/'));
        let real = prefix_dir
            .join("Cellar")
            .join(name)
            .join(version)
            .join("bin")
            .join(name);
        self.write_file(&real, b"\x7fELF binary");

        let link = link_dir.join(name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// The fixed `HOME` used by [`Fixture::env`]/[`Fixture::command`]:
    /// `<root>/home`. Created on demand.
    pub fn home_dir(&self) -> PathBuf {
        let home = self.root().join("home");
        fs::create_dir_all(&home).unwrap();
        home
    }

    /// A `uv tool install`-style install: real file at
    /// `<home>/.local/share/uv/tools/<name>/bin/<name>`, symlinked from
    /// [`Fixture::bin_dir`].
    pub fn uv_tool(&self, name: &str) -> PathBuf {
        let real = self
            .home_dir()
            .join(".local/share/uv/tools")
            .join(name)
            .join("bin")
            .join(name);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A `pipx install`-style install under the default `~/.local/pipx`
    /// location: real file at `<home>/.local/pipx/venvs/<name>/bin/<name>`,
    /// symlinked from [`Fixture::bin_dir`].
    pub fn pipx_tool(&self, name: &str) -> PathBuf {
        let real = self
            .home_dir()
            .join(".local/pipx/venvs")
            .join(name)
            .join("bin")
            .join(name);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A pip-installed-style script: a regular file directly on
    /// `bin_dir()` (no symlink) whose first line is `shebang`.
    pub fn pip_script(&self, name: &str, shebang: &str) -> PathBuf {
        let path = self.bin_dir().join(name);
        self.write_file(&path, format!("{shebang}\nprint('hi')\n").as_bytes());
        path
    }

    /// Like [`Fixture::pip_script`], but placed inside a Homebrew
    /// formula's Cellar directory (symlinked from `bin_dir()`) instead of
    /// directly on PATH — for exercising the "whoever placed the file
    /// wins" priority rule (homebrew beats pip).
    pub fn pip_script_inside_homebrew_cellar(
        &self,
        prefix: &str,
        formula: &str,
        version: &str,
        cmd_name: &str,
        shebang: &str,
    ) -> PathBuf {
        let prefix_dir = self.root().join(prefix.trim_start_matches('/'));
        let real = prefix_dir
            .join("Cellar")
            .join(formula)
            .join(version)
            .join("libexec/bin")
            .join(cmd_name);
        self.write_file(&real, format!("{shebang}\nprint('hi')\n").as_bytes());

        let link = self.bin_dir().join(cmd_name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// Environment map for running the binary against this fixture:
    /// `PATH=<bin_dir>`, `HOME=<root>/home` (a fixed, empty home so
    /// user-home detectors never touch the real machine).
    pub fn env(&self) -> HashMap<String, String> {
        let home = self.home_dir();
        let mut vars = HashMap::new();
        vars.insert(
            "PATH".to_string(),
            self.bin_dir().to_string_lossy().into_owned(),
        );
        vars.insert("HOME".to_string(), home.to_string_lossy().into_owned());
        vars
    }

    /// A ready-to-run `install-from` command with PATH/HOME pointed at
    /// this fixture and no inherited environment.
    pub fn command(&self) -> Command {
        let mut cmd = Command::cargo_bin("install-from").unwrap();
        cmd.env_clear();
        for (k, v) in self.env() {
            cmd.env(k, v);
        }
        cmd.current_dir(self.root());
        cmd
    }

    /// Like [`Fixture::command`], but with `PATH` overridden to the given
    /// directories (joined with `:`) instead of just [`Fixture::bin_dir`].
    /// For `--all` tests, which need the same command name on more than
    /// one `PATH` entry.
    pub fn command_with_path(&self, dirs: &[&Path]) -> Command {
        let joined = std::env::join_paths(dirs).unwrap();
        let mut cmd = self.command();
        cmd.env("PATH", joined);
        cmd
    }

    /// A second directory (not [`Fixture::bin_dir`]) for `--all` tests
    /// that need to place the same command name in more than one `PATH`
    /// entry. Not on `PATH` unless passed to
    /// [`Fixture::command_with_path`].
    pub fn extra_dir(&self, name: &str) -> PathBuf {
        let dir = self.root().join(name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// An npm-global-style install: real file at
    /// `<root>/usr/local/lib/node_modules/<pkg>/bin/<pkg>`, symlinked from
    /// [`Fixture::bin_dir`]. `pkg` may be scoped (`@scope/name`).
    pub fn npm(&self, pkg: &str) -> PathBuf {
        let bin_name = pkg.rsplit('/').next().unwrap();
        let real = self
            .root()
            .join("usr/local/lib/node_modules")
            .join(pkg)
            .join("bin")
            .join(bin_name);
        self.write_file(&real, b"#!/usr/bin/env node\n");

        let link = self.bin_dir().join(bin_name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A version-manager shim: real file under `<home>/.<manager>/shims/`
    /// (asdf/pyenv/rbenv) or `<home>/.volta/bin/` (volta), symlinked from
    /// [`Fixture::bin_dir`] the same way real shim managers put their
    /// shims directory on `PATH`. `manager` must be one of `"asdf"`,
    /// `"pyenv"`, `"rbenv"`, `"volta"`.
    pub fn shim(&self, manager: &str, cmd: &str) -> PathBuf {
        let dir = match manager {
            "asdf" => self.home_dir().join(".asdf/shims"),
            "pyenv" => self.home_dir().join(".pyenv/shims"),
            "rbenv" => self.home_dir().join(".rbenv/shims"),
            "volta" => self.home_dir().join(".volta/bin"),
            other => panic!("Fixture::shim: unsupported manager {other:?}"),
        };
        let real = dir.join(cmd);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(cmd);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A `cargo install`-style install: real file directly under
    /// `<home>/.cargo/bin/<cmd>`, symlinked from [`Fixture::bin_dir`].
    pub fn cargo(&self, cmd: &str) -> PathBuf {
        let real = self.home_dir().join(".cargo/bin").join(cmd);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(cmd);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A `go install`-style install: real file directly under
    /// `<home>/go/bin/<cmd>`, symlinked from [`Fixture::bin_dir`].
    pub fn go(&self, cmd: &str) -> PathBuf {
        let real = self.home_dir().join("go/bin").join(cmd);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(cmd);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    /// A RubyGems-style install: real file at
    /// `<root>/usr/local/lib/ruby/gems/3.3.0/gems/<name>-<ver>/exe/<name>`,
    /// symlinked from [`Fixture::bin_dir`].
    pub fn gem(&self, name: &str, version: &str) -> PathBuf {
        let real = self
            .root()
            .join("usr/local/lib/ruby/gems/3.3.0/gems")
            .join(format!("{name}-{version}"))
            .join("exe")
            .join(name);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(name);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let rel_target = pathdiff(&real, link.parent().unwrap());
        symlink(&rel_target, &link).unwrap();
        link
    }

    // Deliberately no `xcode_clt()` / `macos_system()` fixture helpers:
    // both detectors require a hop whose *absolute* path is literally
    // under `/Library/Developer/CommandLineTools`, `/bin`, `/usr/bin`,
    // etc, which cannot be relocated under a tempdir root. They're
    // covered by unit tests in `src/detect/{macos_system,xcode_clt}.rs`
    // and by end-to-end tests against the real filesystem in
    // `tests/cli_slice.rs`.
}

/// Compute a relative path from `base` to `target` (both must be absolute
/// and share a common ancestor), for writing symlinks the way Homebrew
/// does (`../Cellar/...`).
fn pathdiff(target: &Path, base: &Path) -> PathBuf {
    let target_comps: Vec<_> = target.components().collect();
    let base_comps: Vec<_> = base.components().collect();

    let common = target_comps
        .iter()
        .zip(base_comps.iter())
        .take_while(|(a, b)| a == b)
        .count();

    let mut result = PathBuf::new();
    for _ in common..base_comps.len() {
        result.push("..");
    }
    for comp in &target_comps[common..] {
        result.push(comp.as_os_str());
    }
    result
}
