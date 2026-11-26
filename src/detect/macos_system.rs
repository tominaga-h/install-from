//! `macos-system`: OS-bundled binaries directly under `/bin`, `/sbin`,
//! `/usr/bin`, `/usr/sbin`. SPEC.md §5.1 row 1.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::{Map, json};
use std::path::Path;

pub struct MacosSystem;

/// The four standard directories, paired with whether a hop directly
/// under them counts as a "shell command" (SPEC.md §5.1: `shell_command`
/// is true only under `/usr/bin`).
const STANDARD_DIRS: &[(&str, bool)] = &[
    ("/bin", false),
    ("/sbin", false),
    ("/usr/bin", true),
    ("/usr/sbin", false),
];

impl Detector for MacosSystem {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            let Some(parent) = hop.parent() else {
                continue;
            };
            for (dir, shell_command) in STANDARD_DIRS {
                if parent == Path::new(dir) {
                    let mut detail = Map::new();
                    detail.insert("shell_command".to_string(), json!(*shell_command));
                    return Some(Finding {
                        source: Source::MacosSystem,
                        package: None,
                        version: None,
                        detail,
                    });
                }
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
    fn bin_matches_without_shell_command() {
        let finding = MacosSystem.detect(&env(), &target_for("/bin/ls")).unwrap();
        assert_eq!(finding.source, Source::MacosSystem);
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(false)));
    }

    #[test]
    fn usr_bin_matches_with_shell_command() {
        let finding = MacosSystem
            .detect(&env(), &target_for("/usr/bin/git"))
            .unwrap();
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(true)));
    }

    #[test]
    fn sbin_matches_without_shell_command() {
        let finding = MacosSystem
            .detect(&env(), &target_for("/sbin/mount"))
            .unwrap();
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(false)));
    }

    #[test]
    fn usr_sbin_matches_without_shell_command() {
        let finding = MacosSystem
            .detect(&env(), &target_for("/usr/sbin/ntpdate"))
            .unwrap();
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(false)));
    }

    #[test]
    fn nested_subdirectory_does_not_match() {
        let target = target_for("/usr/bin/subdir/tool");
        assert!(MacosSystem.detect(&env(), &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let target = target_for("/usr/local/bin/jq");
        assert!(MacosSystem.detect(&env(), &target).is_none());
    }

    #[test]
    fn matches_via_symlink_hop_not_just_first() {
        let mut target = target_for("/opt/homebrew/bin/git");
        target.hops_abs.push(PathBuf::from("/usr/bin/git"));
        target.hops_display.push("/usr/bin/git".to_string());

        let finding = MacosSystem.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(true)));
    }

    #[test]
    fn root_hop_with_no_parent_is_skipped_and_later_hop_still_matches() {
        // The root hop `/` has no parent (`Path::parent()` returns `None`),
        // which must be skipped rather than aborting the scan of later hops.
        let mut target = target_for("/");
        target.hops_abs.push(PathBuf::from("/usr/bin/git"));
        target.hops_display.push("/usr/bin/git".to_string());

        let finding = MacosSystem.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("shell_command"), Some(&json!(true)));
    }
}
