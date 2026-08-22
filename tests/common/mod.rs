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
        let path = self.bin_dir().join(name);
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
        let prefix_dir = self.root().join(prefix.trim_start_matches('/'));
        let real = prefix_dir
            .join("Cellar")
            .join(name)
            .join(version)
            .join("bin")
            .join(name);
        self.write_file(&real, b"\x7fELF binary");

        let link = self.bin_dir().join(name);
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
