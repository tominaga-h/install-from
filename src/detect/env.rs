//! Resolved base directories used by detectors. SPEC.md §5.1.
//!
//! All values are computed once, either from the real process environment
//! ([`Env::from_env`]) or from an injected map ([`Env::from_map`]) so tests
//! never depend on the real machine's environment or `$HOME`.
//!
//! Wired into `main` in Task 4; used directly by unit tests in the
//! meantime.
#![allow(dead_code)]

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Base directories consulted by detectors, resolved from environment
/// variables with the defaults documented in SPEC.md §5.1.
///
/// Fields that default under `$HOME` are `None` when `$HOME` is unset and
/// no explicit override was given, so the corresponding detector is
/// skipped entirely (SPEC.md §6: "`$HOME` unset → user-home detectors are
/// skipped").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    /// `$HOMEBREW_PREFIX`. No default — Homebrew detection matches any
    /// prefix by inspecting `Cellar`/`Caskroom` path components (Task 4),
    /// so this is an optional hint rather than a required base dir.
    pub homebrew_prefix: Option<PathBuf>,

    /// `$ASDF_DATA_DIR`, default `~/.asdf`.
    pub asdf_data_dir: Option<PathBuf>,
    /// `$MISE_DATA_DIR`, default `$XDG_DATA_HOME/mise`.
    pub mise_data_dir: Option<PathBuf>,
    /// `$NVM_DIR`, default `~/.nvm`.
    pub nvm_dir: Option<PathBuf>,
    /// `$PYENV_ROOT`, default `~/.pyenv`.
    pub pyenv_root: Option<PathBuf>,
    /// `$RBENV_ROOT`, default `~/.rbenv`.
    pub rbenv_root: Option<PathBuf>,
    /// `$VOLTA_HOME`, default `~/.volta`.
    pub volta_home: Option<PathBuf>,

    /// `$UV_TOOL_DIR`, default `$XDG_DATA_HOME/uv/tools`.
    pub uv_tool_dir: Option<PathBuf>,
    /// `$PIPX_HOME` candidates. If the env var is set, exactly one
    /// candidate. Otherwise both documented defaults, in the order they
    /// should be tried: `~/.local/pipx`, then
    /// `~/Library/Application Support/pipx`.
    pub pipx_home_candidates: Vec<PathBuf>,

    /// `$CARGO_HOME`, default `~/.cargo`.
    pub cargo_home: Option<PathBuf>,
    /// `$GOBIN`, no default of its own (falls back to `gopath/bin`).
    pub gobin: Option<PathBuf>,
    /// `$GOPATH`, default `~/go`.
    pub gopath: Option<PathBuf>,

    /// `$XDG_DATA_HOME`, default `~/.local/share`.
    pub xdg_data_home: Option<PathBuf>,
    /// `$HOME`, if set.
    pub home: Option<PathBuf>,
}

impl Env {
    /// Build an `Env` from the real process environment.
    pub fn from_env() -> Self {
        let vars: HashMap<String, String> = env::vars().collect();
        Self::from_map(&vars)
    }

    /// Build an `Env` from an explicit variable map, so tests can inject a
    /// fully controlled environment (including "`$HOME` unset") without
    /// touching real process state.
    pub fn from_map(vars: &HashMap<String, String>) -> Self {
        let get = |key: &str| -> Option<PathBuf> {
            vars.get(key).filter(|v| !v.is_empty()).map(PathBuf::from)
        };

        let home = get("HOME");
        let xdg_data_home =
            get("XDG_DATA_HOME").or_else(|| home.as_ref().map(|h| h.join(".local/share")));

        let asdf_data_dir = get("ASDF_DATA_DIR").or_else(|| home.as_ref().map(|h| h.join(".asdf")));
        let mise_data_dir =
            get("MISE_DATA_DIR").or_else(|| xdg_data_home.as_ref().map(|d| d.join("mise")));
        let nvm_dir = get("NVM_DIR").or_else(|| home.as_ref().map(|h| h.join(".nvm")));
        let pyenv_root = get("PYENV_ROOT").or_else(|| home.as_ref().map(|h| h.join(".pyenv")));
        let rbenv_root = get("RBENV_ROOT").or_else(|| home.as_ref().map(|h| h.join(".rbenv")));
        let volta_home = get("VOLTA_HOME").or_else(|| home.as_ref().map(|h| h.join(".volta")));

        let uv_tool_dir =
            get("UV_TOOL_DIR").or_else(|| xdg_data_home.as_ref().map(|d| d.join("uv/tools")));

        let pipx_home_candidates = match get("PIPX_HOME") {
            Some(dir) => vec![dir],
            None => home
                .as_ref()
                .map(|h| {
                    vec![
                        h.join(".local/pipx"),
                        h.join("Library/Application Support/pipx"),
                    ]
                })
                .unwrap_or_default(),
        };

        let cargo_home = get("CARGO_HOME").or_else(|| home.as_ref().map(|h| h.join(".cargo")));
        let gobin = get("GOBIN");
        let gopath = get("GOPATH").or_else(|| home.as_ref().map(|h| h.join("go")));

        Env {
            homebrew_prefix: get("HOMEBREW_PREFIX"),
            asdf_data_dir,
            mise_data_dir,
            nvm_dir,
            pyenv_root,
            rbenv_root,
            volta_home,
            uv_tool_dir,
            pipx_home_candidates,
            cargo_home,
            gobin,
            gopath,
            xdg_data_home,
            home,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn defaults_derive_from_home() {
        let env = Env::from_map(&map(&[("HOME", "/Users/alice")]));

        assert_eq!(env.home, Some(PathBuf::from("/Users/alice")));
        assert_eq!(
            env.xdg_data_home,
            Some(PathBuf::from("/Users/alice/.local/share"))
        );
        assert_eq!(env.asdf_data_dir, Some(PathBuf::from("/Users/alice/.asdf")));
        assert_eq!(
            env.mise_data_dir,
            Some(PathBuf::from("/Users/alice/.local/share/mise"))
        );
        assert_eq!(env.nvm_dir, Some(PathBuf::from("/Users/alice/.nvm")));
        assert_eq!(env.pyenv_root, Some(PathBuf::from("/Users/alice/.pyenv")));
        assert_eq!(env.rbenv_root, Some(PathBuf::from("/Users/alice/.rbenv")));
        assert_eq!(env.volta_home, Some(PathBuf::from("/Users/alice/.volta")));
        assert_eq!(
            env.uv_tool_dir,
            Some(PathBuf::from("/Users/alice/.local/share/uv/tools"))
        );
        assert_eq!(env.cargo_home, Some(PathBuf::from("/Users/alice/.cargo")));
        assert_eq!(env.gopath, Some(PathBuf::from("/Users/alice/go")));
        assert_eq!(env.gobin, None);
        assert_eq!(env.homebrew_prefix, None);
        assert_eq!(
            env.pipx_home_candidates,
            vec![
                PathBuf::from("/Users/alice/.local/pipx"),
                PathBuf::from("/Users/alice/Library/Application Support/pipx"),
            ]
        );
    }

    #[test]
    fn explicit_overrides_win_over_home_derived_defaults() {
        let env = Env::from_map(&map(&[
            ("HOME", "/Users/alice"),
            ("HOMEBREW_PREFIX", "/opt/homebrew"),
            ("ASDF_DATA_DIR", "/custom/asdf"),
            ("MISE_DATA_DIR", "/custom/mise"),
            ("NVM_DIR", "/custom/nvm"),
            ("PYENV_ROOT", "/custom/pyenv"),
            ("RBENV_ROOT", "/custom/rbenv"),
            ("VOLTA_HOME", "/custom/volta"),
            ("UV_TOOL_DIR", "/custom/uv-tools"),
            ("PIPX_HOME", "/custom/pipx"),
            ("CARGO_HOME", "/custom/cargo"),
            ("GOBIN", "/custom/gobin"),
            ("GOPATH", "/custom/gopath"),
            ("XDG_DATA_HOME", "/custom/xdg"),
        ]));

        assert_eq!(env.homebrew_prefix, Some(PathBuf::from("/opt/homebrew")));
        assert_eq!(env.asdf_data_dir, Some(PathBuf::from("/custom/asdf")));
        assert_eq!(env.mise_data_dir, Some(PathBuf::from("/custom/mise")));
        assert_eq!(env.nvm_dir, Some(PathBuf::from("/custom/nvm")));
        assert_eq!(env.pyenv_root, Some(PathBuf::from("/custom/pyenv")));
        assert_eq!(env.rbenv_root, Some(PathBuf::from("/custom/rbenv")));
        assert_eq!(env.volta_home, Some(PathBuf::from("/custom/volta")));
        assert_eq!(env.uv_tool_dir, Some(PathBuf::from("/custom/uv-tools")));
        assert_eq!(
            env.pipx_home_candidates,
            vec![PathBuf::from("/custom/pipx")]
        );
        assert_eq!(env.cargo_home, Some(PathBuf::from("/custom/cargo")));
        assert_eq!(env.gobin, Some(PathBuf::from("/custom/gobin")));
        assert_eq!(env.gopath, Some(PathBuf::from("/custom/gopath")));
        assert_eq!(env.xdg_data_home, Some(PathBuf::from("/custom/xdg")));
    }

    #[test]
    fn home_unset_skips_user_home_detectors() {
        let env = Env::from_map(&map(&[]));

        assert_eq!(env.home, None);
        assert_eq!(env.xdg_data_home, None);
        assert_eq!(env.asdf_data_dir, None);
        assert_eq!(env.mise_data_dir, None);
        assert_eq!(env.nvm_dir, None);
        assert_eq!(env.pyenv_root, None);
        assert_eq!(env.rbenv_root, None);
        assert_eq!(env.volta_home, None);
        assert_eq!(env.uv_tool_dir, None);
        assert_eq!(env.cargo_home, None);
        assert_eq!(env.gopath, None);
        assert!(env.pipx_home_candidates.is_empty());
    }

    #[test]
    fn home_unset_but_explicit_overrides_still_apply() {
        let env = Env::from_map(&map(&[("CARGO_HOME", "/custom/cargo")]));

        assert_eq!(env.home, None);
        assert_eq!(env.cargo_home, Some(PathBuf::from("/custom/cargo")));
    }

    #[test]
    fn empty_env_var_value_is_treated_as_unset() {
        // e.g. `HOME=` explicitly exported empty must not become PathBuf::from("")
        let env = Env::from_map(&map(&[("HOME", "")]));
        assert_eq!(env.home, None);
    }

    #[test]
    fn gobin_has_no_home_derived_default() {
        let env = Env::from_map(&map(&[("HOME", "/Users/alice")]));
        assert_eq!(env.gobin, None);
        assert_eq!(env.gopath, Some(PathBuf::from("/Users/alice/go")));
    }
}
