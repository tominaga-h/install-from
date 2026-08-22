//! `npm`: npm / yarn / pnpm global installs, identified by
//! `/node_modules/<pkg>/` or `/node_modules/@scope/<pkg>/` anywhere in a
//! hop's path. SPEC.md §5.1 row 5.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::{Component, Path};

pub struct Npm;

/// Find `/node_modules/<pkg>/` (or `/node_modules/@scope/<pkg>/`) in `hop`
/// and return the package name.
fn find_package(hop: &Path) -> Option<String> {
    let components: Vec<Component> = hop.components().collect();

    for (i, comp) in components.iter().enumerate() {
        if comp.as_os_str() != "node_modules" {
            continue;
        }

        let Component::Normal(first) = components.get(i + 1)? else {
            continue;
        };
        let first = first.to_str()?;

        if let Some(scope) = first.strip_prefix('@') {
            let Component::Normal(pkg) = components.get(i + 2)? else {
                continue;
            };
            return Some(format!("@{scope}/{}", pkg.to_str()?));
        }
        return Some(first.to_string());
    }
    None
}

fn client_for(hop: &Path) -> &'static str {
    let s = hop.to_string_lossy();
    if s.contains("/.yarn/") || s.contains("/yarn/global/") {
        "yarn"
    } else if s.contains("/pnpm/") {
        "pnpm"
    } else {
        "npm"
    }
}

impl Detector for Npm {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            if let Some(package) = find_package(hop) {
                let mut detail = Map::new();
                detail.insert("client".to_string(), json!(client_for(hop)));
                return Some(Finding {
                    source: Source::Npm,
                    package: Some(package),
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
    use std::path::PathBuf;

    fn env() -> Env {
        Env::from_map(&std::collections::HashMap::new())
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
    fn unscoped_package_matches() {
        let target = target_for("/usr/local/lib/node_modules/prettier/bin/prettier.cjs");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::Npm);
        assert_eq!(finding.package.as_deref(), Some("prettier"));
        assert_eq!(finding.version, None);
        assert_eq!(finding.detail.get("client"), Some(&json!("npm")));
    }

    #[test]
    fn scoped_package_matches() {
        let target = target_for("/usr/local/lib/node_modules/@angular/cli/bin/ng");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("@angular/cli"));
    }

    #[test]
    fn yarn_client_via_dot_yarn() {
        let target = target_for("/Users/alice/.yarn/global/node_modules/prettier/bin/prettier");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("client"), Some(&json!("yarn")));
    }

    #[test]
    fn yarn_client_via_yarn_global() {
        let target = target_for("/opt/yarn/global/node_modules/prettier/bin/prettier");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("client"), Some(&json!("yarn")));
    }

    #[test]
    fn pnpm_client() {
        let target = target_for("/Users/alice/Library/pnpm/global/5/node_modules/prettier/bin/x");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("client"), Some(&json!("pnpm")));
    }

    #[test]
    fn version_is_always_null() {
        let target = target_for("/usr/local/lib/node_modules/prettier/bin/prettier.cjs");
        let finding = Npm.detect(&env(), &target).unwrap();
        assert_eq!(finding.version, None);
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let target = target_for("/usr/local/bin/jq");
        assert!(Npm.detect(&env(), &target).is_none());
    }

    #[test]
    fn node_modules_without_package_after_it_does_not_match() {
        let target = target_for("/usr/local/lib/node_modules");
        assert!(Npm.detect(&env(), &target).is_none());
    }

    #[test]
    fn scoped_without_package_after_scope_does_not_match() {
        let target = target_for("/usr/local/lib/node_modules/@angular");
        assert!(Npm.detect(&env(), &target).is_none());
    }
}
