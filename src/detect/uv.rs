//! `uv`: `uv tool install`, identified by a hop under
//! `$UV_TOOL_DIR/<name>/` (default `$XDG_DATA_HOME/uv/tools`).
//! SPEC.md §5.1 row 6.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::Map;
use std::path::Component;

pub struct Uv;

impl Detector for Uv {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding> {
        let tool_dir = env.uv_tool_dir.as_ref()?;

        for hop in &target.hops_abs {
            let Ok(rest) = hop.strip_prefix(tool_dir) else {
                continue;
            };
            let Some(Component::Normal(name)) = rest.components().next() else {
                continue;
            };
            let Some(name) = name.to_str() else {
                continue;
            };

            return Some(Finding {
                source: Source::Uv,
                package: Some(name.to_string()),
                version: None,
                detail: Map::new(),
            });
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
        let target = target_for("/Users/alice/.local/share/uv/tools/ruff/bin/ruff");
        let finding = Uv.detect(&env, &target).unwrap();
        assert_eq!(finding.source, Source::Uv);
        assert_eq!(finding.package.as_deref(), Some("ruff"));
        assert_eq!(finding.version, None);
    }

    #[test]
    fn uv_tool_dir_override_wins() {
        let env = env_with(&[
            ("HOME", "/Users/alice"),
            ("UV_TOOL_DIR", "/custom/uv-tools"),
        ]);
        let target = target_for("/custom/uv-tools/ruff/bin/ruff");
        let finding = Uv.detect(&env, &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("ruff"));

        let default_location = target_for("/Users/alice/.local/share/uv/tools/ruff/bin/ruff");
        assert!(Uv.detect(&env, &default_location).is_none());
    }

    #[test]
    fn xdg_data_home_override_shifts_default() {
        let env = env_with(&[("HOME", "/Users/alice"), ("XDG_DATA_HOME", "/custom/xdg")]);
        let target = target_for("/custom/xdg/uv/tools/ruff/bin/ruff");
        let finding = Uv.detect(&env, &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("ruff"));
    }

    #[test]
    fn home_unset_and_no_override_skips_detector() {
        let env = env_with(&[]);
        let target = target_for("/Users/alice/.local/share/uv/tools/ruff/bin/ruff");
        assert!(Uv.detect(&env, &target).is_none());
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/usr/local/bin/jq");
        assert!(Uv.detect(&env, &target).is_none());
    }

    #[test]
    fn tool_dir_without_name_after_it_does_not_match() {
        let env = env_with(&[("HOME", "/Users/alice")]);
        let target = target_for("/Users/alice/.local/share/uv/tools");
        assert!(Uv.detect(&env, &target).is_none());
    }

    #[test]
    fn non_utf8_named_hop_is_skipped_and_later_hop_still_matches() {
        // A hop whose tool-name component is not valid UTF-8 must be
        // skipped rather than aborting the scan of later hops.
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let env = env_with(&[("HOME", "/Users/alice")]);
        let mut non_utf8 = PathBuf::from("/Users/alice/.local/share/uv/tools");
        non_utf8.push(OsString::from_vec(vec![0xFF, 0xFE]));
        non_utf8.push("bin/tool");

        let mut target = target_for(non_utf8.to_string_lossy().as_ref());
        target.hops_abs[0] = non_utf8;
        target.hops_abs.push(PathBuf::from(
            "/Users/alice/.local/share/uv/tools/ruff/bin/ruff",
        ));
        target
            .hops_display
            .push("/Users/alice/.local/share/uv/tools/ruff/bin/ruff".to_string());

        let finding = Uv.detect(&env, &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("ruff"));
    }
}
