//! `xcode-clt`: Xcode / Command Line Tools, identified by a hop starting
//! with `/Library/Developer/CommandLineTools/` or
//! `/Applications/Xcode*.app/Contents/Developer/`. SPEC.md §5.1 row 2.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::{Component, Path};

pub struct XcodeClt;

const CLT_ROOT: &str = "/Library/Developer/CommandLineTools";

/// If `hop` starts under `/Applications/Xcode*.app/Contents/Developer`,
/// return the `.../Xcode*.app` prefix as the root.
fn xcode_app_root(hop: &Path) -> Option<String> {
    let components: Vec<Component> = hop.components().collect();
    if components.len() < 2 || components[0] != Component::RootDir {
        return None;
    }
    let Component::Normal(applications) = components.get(1)? else {
        return None;
    };
    if applications.to_str() != Some("Applications") {
        return None;
    }
    let Component::Normal(app) = components.get(2)? else {
        return None;
    };
    let app_name = app.to_str()?;
    if !app_name.starts_with("Xcode") || !app_name.ends_with(".app") {
        return None;
    }
    if components.get(3).and_then(|c| c.as_os_str().to_str()) != Some("Contents") {
        return None;
    }
    if components.get(4).and_then(|c| c.as_os_str().to_str()) != Some("Developer") {
        return None;
    }

    Some(format!("/Applications/{app_name}"))
}

impl Detector for XcodeClt {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            if hop.starts_with(CLT_ROOT) {
                let mut detail = Map::new();
                detail.insert("root".to_string(), json!(CLT_ROOT));
                return Some(Finding {
                    source: Source::XcodeClt,
                    package: None,
                    version: None,
                    detail,
                });
            }
            if let Some(root) = xcode_app_root(hop) {
                let mut detail = Map::new();
                detail.insert("root".to_string(), json!(root));
                return Some(Finding {
                    source: Source::XcodeClt,
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
    fn command_line_tools_root_matches() {
        let target = target_for("/Library/Developer/CommandLineTools/usr/bin/git");
        let finding = XcodeClt.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::XcodeClt);
        assert_eq!(
            finding.detail.get("root"),
            Some(&json!("/Library/Developer/CommandLineTools"))
        );
    }

    #[test]
    fn xcode_app_matches() {
        let target = target_for("/Applications/Xcode.app/Contents/Developer/usr/bin/git");
        let finding = XcodeClt.detect(&env(), &target).unwrap();
        assert_eq!(
            finding.detail.get("root"),
            Some(&json!("/Applications/Xcode.app"))
        );
    }

    #[test]
    fn xcode_beta_app_matches() {
        let target = target_for("/Applications/Xcode-beta.app/Contents/Developer/usr/bin/x");
        let finding = XcodeClt.detect(&env(), &target).unwrap();
        assert_eq!(
            finding.detail.get("root"),
            Some(&json!("/Applications/Xcode-beta.app"))
        );
    }

    #[test]
    fn unrelated_applications_app_does_not_match() {
        let target = target_for("/Applications/Docker.app/Contents/MacOS/Docker");
        assert!(XcodeClt.detect(&env(), &target).is_none());
    }

    #[test]
    fn xcode_app_without_contents_developer_does_not_match() {
        let target = target_for("/Applications/Xcode.app/Contents/MacOS/Xcode");
        assert!(XcodeClt.detect(&env(), &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let target = target_for("/usr/local/bin/git");
        assert!(XcodeClt.detect(&env(), &target).is_none());
    }

    #[test]
    fn matches_via_symlink_hop() {
        let mut target = target_for("/usr/bin/git");
        target.hops_abs.push(PathBuf::from(
            "/Library/Developer/CommandLineTools/usr/bin/git",
        ));
        let finding = XcodeClt.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::XcodeClt);
    }

    #[test]
    fn xcode_numbered_app_matches() {
        // Another `Xcode*.app` naming pattern beyond the two already
        // covered (`Xcode.app`, `Xcode-beta.app`).
        let target = target_for("/Applications/Xcode-16.app/Contents/Developer/usr/bin/git");
        let finding = XcodeClt.detect(&env(), &target).unwrap();
        assert_eq!(
            finding.detail.get("root"),
            Some(&json!("/Applications/Xcode-16.app"))
        );
    }

    #[test]
    fn app_name_not_starting_with_xcode_does_not_match() {
        // `starts_with("Xcode")` does not hold.
        let target = target_for("/Applications/MyXcode.app/Contents/Developer/usr/bin/git");
        assert!(XcodeClt.detect(&env(), &target).is_none());
    }

    #[test]
    fn app_name_not_ending_with_dot_app_does_not_match() {
        // `ends_with(".app")` does not hold.
        let target = target_for("/Applications/Xcode.appx/Contents/Developer/usr/bin/git");
        assert!(XcodeClt.detect(&env(), &target).is_none());
    }
}
