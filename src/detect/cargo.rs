//! `cargo`: `cargo install`, identified by a hop that is a direct child of
//! `$CARGO_HOME/bin` (default `~/.cargo/bin`). SPEC.md §5.1 row 9.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};

pub struct Cargo;

impl Detector for Cargo {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding> {
        let cargo_home = env.cargo_home.as_ref()?;
        let bin_dir = cargo_home.join("bin");

        for hop in &target.hops_abs {
            if hop.parent() == Some(bin_dir.as_path()) {
                let mut detail = Map::new();
                detail.insert(
                    "cargo_home".to_string(),
                    json!(cargo_home.to_string_lossy()),
                );
                return Some(Finding {
                    source: Source::Cargo,
                    package: None,
                    version: None,
                    detail,
                });
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::Shebang;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn env_with(pairs: &[(&str, &str)]) -> Env {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Env::from_map(&map)
    }

    fn target_for(path: &str) -> Target {
        Target {
            path: PathBuf::from(path),
            hops_display: vec![path.to_string()],
            hops_abs: vec![PathBuf::from(path)],
            resolved_path: PathBuf::from(path),
            dangling: false,
            shebang: Shebang::Binary,
        }
    }

    #[test]
    fn default_location_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/.cargo/bin/rg");
        let finding = Cargo.detect(&env, &target).unwrap();
        assert_eq!(finding.source, Source::Cargo);
        assert_eq!(finding.package, None);
        assert_eq!(finding.version, None);
        assert_eq!(
            finding.detail.get("cargo_home"),
            Some(&json!("/Users/alice/.cargo"))
        );
    }

    #[test]
    fn cargo_home_override_wins() {
        let env = env_with(&[("HOME", "/Users/alice"), ("CARGO_HOME", "/custom/cargo")]);
        let target = target_for("/custom/cargo/bin/rg");
        let finding = Cargo.detect(&env, &target).unwrap();
        assert_eq!(
            finding.detail.get("cargo_home"),
            Some(&json!("/custom/cargo"))
        );

        let default_location = target_for("/Users/alice/.cargo/bin/rg");
        assert!(Cargo.detect(&env, &default_location).is_none());
    }

    #[test]
    fn home_unset_and_no_override_skips_detector() {
        let env = env_with(&[]);
        let target = target_for("/Users/alice/.cargo/bin/rg");
        assert!(Cargo.detect(&env, &target).is_none());
    }

    #[test]
    fn nested_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/.cargo/bin/sub/rg");
        assert!(Cargo.detect(&env, &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/usr/local/bin/jq");
        assert!(Cargo.detect(&env, &target).is_none());
    }
}
