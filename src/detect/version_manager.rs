//! `version-manager`: shims/real binaries owned by asdf, mise, nvm, pyenv,
//! rbenv, or volta. SPEC.md §5.1 row 3.
//!
//! nvm is special: `$NVM_DIR/versions/node/*/bin/` also contains symlinks
//! into `lib/node_modules/<pkg>` for globally-installed npm packages. Per
//! SPEC.md §5, "whoever placed the file wins" — those should be reported
//! as `npm` (Task 7), so nvm only matches when *no* hop in the chain is
//! inside `node_modules`.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::{Path, PathBuf};

pub struct VersionManager;

fn hop_in_node_modules(hops: &[PathBuf]) -> bool {
    hops.iter()
        .any(|h| h.components().any(|c| c.as_os_str() == "node_modules"))
}

fn nvm_versions_node_bin(nvm_dir: &Path, hop: &Path) -> bool {
    let Ok(rest) = hop.strip_prefix(nvm_dir) else {
        return false;
    };
    let mut comps = rest.components();
    matches!(comps.next().and_then(|c| c.as_os_str().to_str()), Some("versions"))
        && matches!(comps.next().and_then(|c| c.as_os_str().to_str()), Some("node"))
        && comps.next().is_some() // vX.Y.Z
        && matches!(comps.next().and_then(|c| c.as_os_str().to_str()), Some("bin"))
        && comps.next().is_some() // the command itself
}

impl Detector for VersionManager {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding> {
        let shims_managers: &[(Option<&PathBuf>, &str)] = &[
            (env.asdf_data_dir.as_ref(), "asdf"),
            (env.mise_data_dir.as_ref(), "mise"),
            (env.pyenv_root.as_ref(), "pyenv"),
            (env.rbenv_root.as_ref(), "rbenv"),
        ];

        for hop in &target.hops_abs {
            for (base, manager) in shims_managers {
                if let Some(base) = base
                    && hop.starts_with(base.join("shims"))
                {
                    return Some(finding(manager));
                }
            }

            if let Some(volta_home) = &env.volta_home
                && hop.starts_with(volta_home.join("bin"))
            {
                return Some(finding("volta"));
            }
        }

        if let Some(nvm_dir) = &env.nvm_dir
            && !hop_in_node_modules(&target.hops_abs)
            && target
                .hops_abs
                .iter()
                .any(|hop| nvm_versions_node_bin(nvm_dir, hop))
        {
            return Some(finding("nvm"));
        }

        None
    }
}

fn finding(manager: &str) -> Finding {
    let mut detail = Map::new();
    detail.insert("manager".to_string(), json!(manager));
    Finding {
        source: Source::VersionManager,
        package: None,
        version: None,
        detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::Shebang;
    use std::collections::HashMap;

    fn env_with(pairs: &[(&str, &str)]) -> Env {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Env::from_map(&map)
    }

    fn target_with_hops(hops: &[&str]) -> Target {
        let last = hops.last().unwrap();
        Target {
            path: PathBuf::from(hops[0]),
            hops_display: hops.iter().map(|s| s.to_string()).collect(),
            hops_abs: hops.iter().map(|s| PathBuf::from(*s)).collect(),
            resolved_path: PathBuf::from(*last),
            dangling: false,
            shebang: Shebang::Binary,
        }
    }

    #[test]
    fn asdf_shim_matches_default_location() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.asdf/shims/node"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.source, Source::VersionManager);
        assert_eq!(finding.detail.get("manager"), Some(&json!("asdf")));
    }

    #[test]
    fn mise_shim_matches_default_location() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.local/share/mise/shims/node"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("mise")));
    }

    #[test]
    fn pyenv_shim_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.pyenv/shims/python"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("pyenv")));
    }

    #[test]
    fn rbenv_shim_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.rbenv/shims/ruby"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("rbenv")));
    }

    #[test]
    fn volta_bin_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.volta/bin/node"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("volta")));
    }

    #[test]
    fn env_var_override_wins_over_home_default() {
        let env = env_with(&[("HOME", "/Users/alice"), ("ASDF_DATA_DIR", "/custom/asdf")]);
        let target = target_with_hops(&["/custom/asdf/shims/node"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("asdf")));

        let target_default_location = target_with_hops(&["/Users/alice/.asdf/shims/node"]);
        assert!(
            VersionManager
                .detect(&env, &target_default_location)
                .is_none()
        );
    }

    #[test]
    fn home_unset_skips_all_shim_managers() {
        let env = env_with(&[]);
        let target = target_with_hops(&["/Users/alice/.asdf/shims/node"]);
        assert!(VersionManager.detect(&env, &target).is_none());
    }

    #[test]
    fn nvm_real_binary_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.nvm/versions/node/v20.11.0/bin/node"]);
        let finding = VersionManager.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("manager"), Some(&json!("nvm")));
    }

    #[test]
    fn nvm_global_npm_package_symlink_is_not_matched() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        // nvm bin/prettier -> ../lib/node_modules/prettier/bin/prettier.cjs
        let target = target_with_hops(&[
            "/Users/alice/.nvm/versions/node/v20.11.0/bin/prettier",
            "../lib/node_modules/prettier/bin/prettier.cjs",
        ]);
        let mut target = target;
        target.hops_abs = vec![
            PathBuf::from("/Users/alice/.nvm/versions/node/v20.11.0/bin/prettier"),
            PathBuf::from(
                "/Users/alice/.nvm/versions/node/v20.11.0/lib/node_modules/prettier/bin/prettier.cjs",
            ),
        ];
        assert!(VersionManager.detect(&env, &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/usr/local/bin/jq"]);
        assert!(VersionManager.detect(&env, &target).is_none());
    }

    #[test]
    fn nvm_shallow_path_without_command_segment_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_with_hops(&["/Users/alice/.nvm/versions/node/v20.11.0/bin"]);
        assert!(VersionManager.detect(&env, &target).is_none());
    }
}
