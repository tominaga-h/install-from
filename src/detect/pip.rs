//! `pip`: a script whose shebang interpreter is Python, identified by the
//! final hop's shebang line. Lowest-priority detector before `unknown`.
//! SPEC.md §5.1 row 11.
//!
//! Unlike every other detector, this one only looks at the final hop
//! (there is no filesystem location to test) — and only after every
//! location-based detector has already had a chance to match. A pip
//! script that happens to live under `Cellar/` is still reported as
//! `homebrew`, per SPEC.md §5's "whoever placed the file wins" rule.

use super::{Detector, Env, Finding, Source};
use crate::resolve::{Shebang, Target};
use serde_json::{Map, json};

pub struct Pip;

/// True if `interpreter`'s basename is `python`, `python3`, or
/// `python3.N` (e.g. `python3.12`).
fn is_python_interpreter(interpreter: &str) -> bool {
    let basename = interpreter.rsplit('/').next().unwrap_or(interpreter);
    match basename.strip_prefix("python") {
        Some("") => true,
        Some(rest) => match rest.strip_prefix('3') {
            Some("") => true,
            Some(rest) => rest.starts_with('.') && rest[1..].chars().all(|c| c.is_ascii_digit()),
            None => false,
        },
        None => false,
    }
}

impl Detector for Pip {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        let Shebang::Line(_) = &target.shebang else {
            return None;
        };
        let interpreter = target.shebang.interpreter()?;
        if !is_python_interpreter(interpreter) {
            return None;
        }

        let mut detail = Map::new();
        detail.insert("python".to_string(), json!(interpreter));
        Some(Finding {
            source: Source::Pip,
            package: None,
            version: None,
            detail,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn env() -> Env {
        Env::from_map(&std::collections::HashMap::new())
    }

    fn target_with_shebang(shebang: Shebang) -> Target {
        Target {
            path: PathBuf::from("/usr/local/bin/black"),
            hops_display: vec!["/usr/local/bin/black".to_string()],
            hops_abs: vec![PathBuf::from("/usr/local/bin/black")],
            resolved_path: PathBuf::from("/usr/local/bin/black"),
            dangling: false,
            shebang,
        }
    }

    #[test]
    fn direct_python_interpreter_path() {
        let target = target_with_shebang(Shebang::Line(
            "#!/usr/local/opt/python@3.12/bin/python3.12".to_string(),
        ));
        let finding = Pip.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::Pip);
        assert_eq!(
            finding.detail.get("python"),
            Some(&json!("/usr/local/opt/python@3.12/bin/python3.12"))
        );
    }

    #[test]
    fn env_python3_form() {
        let target = target_with_shebang(Shebang::Line("#!/usr/bin/env python3".to_string()));
        let finding = Pip.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("python"), Some(&json!("python3")));
    }

    #[test]
    fn env_dash_s_python_form() {
        let target = target_with_shebang(Shebang::Line("#!/usr/bin/env -S python3 -u".to_string()));
        let finding = Pip.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("python"), Some(&json!("python3")));
    }

    #[test]
    fn bare_python_matches() {
        let target = target_with_shebang(Shebang::Line("#!/usr/bin/python".to_string()));
        let finding = Pip.detect(&env(), &target).unwrap();
        assert_eq!(
            finding.detail.get("python"),
            Some(&json!("/usr/bin/python"))
        );
    }

    #[test]
    fn python3_dot_thirteen_matches() {
        let target = target_with_shebang(Shebang::Line("#!/usr/bin/env python3.13".to_string()));
        let finding = Pip.detect(&env(), &target).unwrap();
        assert_eq!(finding.detail.get("python"), Some(&json!("python3.13")));
    }

    #[test]
    fn non_python_shebang_does_not_match() {
        let target = target_with_shebang(Shebang::Line("#!/bin/bash".to_string()));
        assert!(Pip.detect(&env(), &target).is_none());
    }

    #[test]
    fn python_looking_but_unrelated_word_does_not_match() {
        // "pythonic" starts with "python" but isn't the interpreter itself.
        let target = target_with_shebang(Shebang::Line("#!/usr/bin/pythonic".to_string()));
        assert!(Pip.detect(&env(), &target).is_none());
    }

    #[test]
    fn binary_file_does_not_match() {
        let target = target_with_shebang(Shebang::Binary);
        assert!(Pip.detect(&env(), &target).is_none());
    }

    #[test]
    fn unreadable_file_does_not_match() {
        let target = target_with_shebang(Shebang::Unreadable);
        assert!(Pip.detect(&env(), &target).is_none());
    }
}
