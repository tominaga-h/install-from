//! `pipx`: identified by a hop under `$PIPX_HOME/venvs/<name>/`, where the
//! default checks `~/.local/pipx` then `~/Library/Application
//! Support/pipx` (see [`Env::pipx_home_candidates`]). SPEC.md §5.1 row 7.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::Map;
use std::path::Component;

pub struct Pipx;

impl Detector for Pipx {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            for pipx_home in &env.pipx_home_candidates {
                let venvs = pipx_home.join("venvs");
                let Ok(rest) = hop.strip_prefix(&venvs) else {
                    continue;
                };
                let Some(Component::Normal(name)) = rest.components().next() else {
                    continue;
                };
                let Some(name) = name.to_str() else {
                    continue;
                };

                return Some(Finding {
                    source: Source::Pipx,
                    package: Some(name.to_string()),
                    version: None,
                    detail: Map::new(),
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
    fn default_local_pipx_location_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/.local/pipx/venvs/black/bin/black");
        let finding = Pipx.detect(&env, &target).unwrap();
        assert_eq!(finding.source, Source::Pipx);
        assert_eq!(finding.package.as_deref(), Some("black"));
        assert_eq!(finding.version, None);
    }

    #[test]
    fn default_application_support_location_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target =
            target_for("/Users/alice/Library/Application Support/pipx/venvs/black/bin/black");
        let finding = Pipx.detect(&env, &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("black"));
    }

    #[test]
    fn pipx_home_override_wins_and_disables_defaults() {
        let env = env_with(&[("HOME", "/Users/alice"), ("PIPX_HOME", "/custom/pipx")]);
        let target = target_for("/custom/pipx/venvs/black/bin/black");
        let finding = Pipx.detect(&env, &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("black"));

        let default_location = target_for("/Users/alice/.local/pipx/venvs/black/bin/black");
        assert!(Pipx.detect(&env, &default_location).is_none());
    }

    #[test]
    fn home_unset_and_no_override_skips_detector() {
        let env = env_with(&[]);
        let target = target_for("/Users/alice/.local/pipx/venvs/black/bin/black");
        assert!(Pipx.detect(&env, &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/usr/local/bin/jq");
        assert!(Pipx.detect(&env, &target).is_none());
    }

    #[test]
    fn venvs_without_name_after_it_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/.local/pipx/venvs");
        assert!(Pipx.detect(&env, &target).is_none());
    }
}
