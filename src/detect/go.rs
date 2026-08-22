//! `go`: `go install`, identified by a hop that is a direct child of
//! `$GOBIN`, or of `$GOPATH/bin` (default `~/go/bin`) if `$GOBIN` is unset.
//! SPEC.md §5.1 row 10.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::PathBuf;

pub struct Go;

impl Detector for Go {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding> {
        let bin_dir: PathBuf = match (&env.gobin, &env.gopath) {
            (Some(gobin), _) => gobin.clone(),
            (None, Some(gopath)) => gopath.join("bin"),
            (None, None) => return None,
        };

        for hop in &target.hops_abs {
            if hop.parent() == Some(bin_dir.as_path()) {
                let mut detail = Map::new();
                detail.insert("gobin".to_string(), json!(bin_dir.to_string_lossy()));
                return Some(Finding {
                    source: Source::Go,
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
    fn default_gopath_bin_matches() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/go/bin/gopls");
        let finding = Go.detect(&env, &target).unwrap();
        assert_eq!(finding.source, Source::Go);
        assert_eq!(finding.package, None);
        assert_eq!(finding.version, None);
        assert_eq!(
            finding.detail.get("gobin"),
            Some(&json!("/Users/alice/go/bin"))
        );
    }

    #[test]
    fn gopath_override_wins() {
        let env = env_with(&[("HOME", "/Users/alice"), ("GOPATH", "/custom/gopath")]);
        let target = target_for("/custom/gopath/bin/gopls");
        let finding = Go.detect(&env, &target).unwrap();
        assert_eq!(
            finding.detail.get("gobin"),
            Some(&json!("/custom/gopath/bin"))
        );
    }

    #[test]
    fn gobin_takes_precedence_over_gopath_bin() {
        let env = env_with(&[
            ("HOME", "/Users/alice"),
            ("GOBIN", "/custom/gobin"),
            ("GOPATH", "/custom/gopath"),
        ]);
        let target = target_for("/custom/gobin/gopls");
        let finding = Go.detect(&env, &target).unwrap();
        assert_eq!(finding.detail.get("gobin"), Some(&json!("/custom/gobin")));

        // GOPATH/bin is not consulted once GOBIN is set.
        let gopath_bin_location = target_for("/custom/gopath/bin/gopls");
        assert!(Go.detect(&env, &gopath_bin_location).is_none());
    }

    #[test]
    fn home_unset_and_no_override_skips_detector() {
        let env = env_with(&[]);
        let target = target_for("/Users/alice/go/bin/gopls");
        assert!(Go.detect(&env, &target).is_none());
    }

    #[test]
    fn nested_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/go/bin/sub/gopls");
        assert!(Go.detect(&env, &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/usr/local/bin/jq");
        assert!(Go.detect(&env, &target).is_none());
    }
}
