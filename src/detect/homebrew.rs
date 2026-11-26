//! `homebrew`: formula or cask, identified by `/Cellar/<name>/<ver>/` or
//! `/Caskroom/<name>/<ver>/` anywhere in a hop's path. SPEC.md §5.1 row 4.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::{Component, Path, PathBuf};

pub struct Homebrew;

struct Match {
    prefix: PathBuf,
    kind: &'static str,
    name: String,
    version: String,
}

/// Look for `.../<prefix>/(Cellar|Caskroom)/<name>/<ver>/...` in `hop`.
fn find_match(hop: &Path) -> Option<Match> {
    let components: Vec<Component> = hop.components().collect();

    for (i, comp) in components.iter().enumerate() {
        let Component::Normal(os_str) = comp else {
            continue;
        };
        let kind = match os_str.to_str() {
            Some("Cellar") => "formula",
            Some("Caskroom") => "cask",
            _ => continue,
        };

        // Need at least <marker>/<name>/<ver>/ after this component.
        let name = components.get(i + 1)?;
        let ver = components.get(i + 2)?;
        let Component::Normal(name) = name else {
            continue;
        };
        let Component::Normal(ver) = ver else {
            continue;
        };

        let mut prefix = PathBuf::new();
        for c in &components[..i] {
            prefix.push(c.as_os_str());
        }

        return Some(Match {
            prefix,
            kind,
            name: name.to_string_lossy().into_owned(),
            version: ver.to_string_lossy().into_owned(),
        });
    }
    None
}

impl Detector for Homebrew {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            if let Some(m) = find_match(hop) {
                let mut detail = Map::new();
                detail.insert("kind".to_string(), json!(m.kind));
                detail.insert(
                    "prefix".to_string(),
                    json!(m.prefix.to_string_lossy().into_owned()),
                );
                return Some(Finding {
                    source: Source::Homebrew,
                    package: Some(m.name),
                    version: Some(m.version),
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

    fn env() -> Env {
        Env::from_map(&std::collections::HashMap::new())
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
    fn formula_under_usr_local() {
        let target = target_with_hops(&["/usr/local/bin/jq", "/usr/local/Cellar/jq/1.7.1/bin/jq"]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::Homebrew);
        assert_eq!(finding.package.as_deref(), Some("jq"));
        assert_eq!(finding.version.as_deref(), Some("1.7.1"));
        assert_eq!(finding.detail.get("kind"), Some(&json!("formula")));
        assert_eq!(finding.detail.get("prefix"), Some(&json!("/usr/local")));
    }

    #[test]
    fn formula_under_opt_homebrew_apple_silicon() {
        let target = target_with_hops(&[
            "/opt/homebrew/bin/jq",
            "/opt/homebrew/Cellar/jq/1.7.1/bin/jq",
        ]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("prefix"), Some(&json!("/opt/homebrew")));
    }

    #[test]
    fn formula_under_custom_prefix() {
        let target = target_with_hops(&[
            "/custom/homebrew/bin/jq",
            "/custom/homebrew/Cellar/jq/1.7.1/bin/jq",
        ]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(
            finding.detail.get("prefix"),
            Some(&json!("/custom/homebrew"))
        );
    }

    #[test]
    fn cask_matches() {
        let target = target_with_hops(&[
            "/usr/local/bin/some-app",
            "/usr/local/Caskroom/some-app/2.0.0/some-app.app/Contents/MacOS/some-app",
        ]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("kind"), Some(&json!("cask")));
        assert_eq!(finding.package.as_deref(), Some("some-app"));
        assert_eq!(finding.version.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn two_stage_symlink_via_opt() {
        // /usr/local/bin/jq -> /usr/local/opt/jq/bin/jq -> /usr/local/Cellar/jq/1.7.1/bin/jq
        let target = target_with_hops(&[
            "/usr/local/bin/jq",
            "/usr/local/opt/jq/bin/jq",
            "/usr/local/Cellar/jq/1.7.1/bin/jq",
        ]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("jq"));
        assert_eq!(finding.version.as_deref(), Some("1.7.1"));
    }

    #[test]
    fn version_taken_verbatim_including_revision_suffix() {
        let target = target_with_hops(&[
            "/usr/local/bin/magick",
            "/usr/local/Cellar/imagemagick-full/7.1.2-18_1/bin/magick",
        ]);
        let finding = Homebrew.detect(&env(), &target).unwrap();
        assert_eq!(finding.version.as_deref(), Some("7.1.2-18_1"));
    }

    #[test]
    fn non_homebrew_path_does_not_match() {
        let target = target_with_hops(&["/usr/local/bin/jq"]);
        assert!(Homebrew.detect(&env(), &target).is_none());
    }

    #[test]
    fn cellar_without_name_and_version_does_not_match() {
        // Pathological/truncated path: "Cellar" present but nothing after it.
        let target = target_with_hops(&["/usr/local/Cellar"]);
        assert!(Homebrew.detect(&env(), &target).is_none());
    }
}
