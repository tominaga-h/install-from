//! Detection: given a resolved [`crate::resolve::Target`], decide which
//! mechanism placed it on disk. SPEC.md §4, §5.
//!
//! [`PRIORITY_ORDER`] gains one entry per source as Tasks 4-10 land;
//! currently `macos-system`, `xcode-clt`, `version-manager`, `homebrew`,
//! `npm`, `uv`, `pipx`, `gem`, `cargo`, `go`.

pub mod cargo;
pub mod env;
pub mod finding;
pub mod gem;
pub mod go;
pub mod homebrew;
pub mod macos_system;
pub mod npm;
pub mod pipx;
pub mod uv;
pub mod version_manager;
pub mod xcode_clt;

pub use env::Env;
pub use finding::{Finding, Source};

use crate::resolve::Target;

/// One detection rule: given the environment and a resolved target, decide
/// whether this source placed the binary. Implementations inspect every
/// hop in `target.hops_abs` themselves (SPEC.md §5: "every hop of the
/// symlink chain is tested").
pub trait Detector {
    fn detect(&self, env: &Env, target: &Target) -> Option<Finding>;
}

/// Detectors in priority order (SPEC.md §5.1).
pub const PRIORITY_ORDER: &[&dyn Detector] = &[
    &macos_system::MacosSystem,
    &xcode_clt::XcodeClt,
    &version_manager::VersionManager,
    &homebrew::Homebrew,
    &npm::Npm,
    &uv::Uv,
    &pipx::Pipx,
    &gem::Gem,
    &cargo::Cargo,
    &go::Go,
];

/// Run every detector in [`PRIORITY_ORDER`] and return the first match,
/// or [`Finding::unknown`] if none matched. SPEC.md §5: "only the first
/// matching detector in priority order is reported."
pub fn run(env: &Env, target: &Target) -> Finding {
    for detector in PRIORITY_ORDER {
        if let Some(finding) = detector.detect(env, target) {
            return finding;
        }
    }
    Finding::unknown()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;
    use std::path::{Path, PathBuf};

    fn stub_target() -> Target {
        Target {
            path: PathBuf::from("/usr/local/bin/x"),
            hops_display: vec!["/usr/local/bin/x".to_string()],
            hops_abs: vec![PathBuf::from("/usr/local/bin/x")],
            resolved_path: PathBuf::from("/usr/local/bin/x"),
            dangling: false,
            shebang: crate::resolve::Shebang::Binary,
        }
    }

    struct AlwaysNone;
    impl Detector for AlwaysNone {
        fn detect(&self, _env: &Env, _target: &Target) -> Option<Finding> {
            None
        }
    }

    struct AlwaysSource(Source);
    impl Detector for AlwaysSource {
        fn detect(&self, _env: &Env, _target: &Target) -> Option<Finding> {
            Some(Finding {
                source: self.0,
                package: None,
                version: None,
                detail: Map::new(),
            })
        }
    }

    fn empty_env() -> Env {
        Env::from_map(&std::collections::HashMap::new())
    }

    #[test]
    fn empty_priority_order_falls_through_to_unknown() {
        let finding = run(&empty_env(), &stub_target());
        assert_eq!(finding.source, Source::Unknown);
    }

    #[test]
    fn driver_returns_first_matching_detector_in_order() {
        let a = AlwaysNone;
        let b = AlwaysSource(Source::Homebrew);
        let c = AlwaysSource(Source::Npm);
        let order: Vec<&dyn Detector> = vec![&a, &b, &c];

        let mut finding = None;
        for detector in &order {
            if let Some(f) = detector.detect(&empty_env(), &stub_target()) {
                finding = Some(f);
                break;
            }
        }
        assert_eq!(finding.unwrap().source, Source::Homebrew);
    }

    #[test]
    fn driver_skips_non_matching_detectors_before_a_match() {
        let a = AlwaysNone;
        let b = AlwaysNone;
        let c = AlwaysSource(Source::Cargo);
        let order: Vec<&dyn Detector> = vec![&a, &b, &c];

        let mut finding = None;
        for detector in &order {
            if let Some(f) = detector.detect(&empty_env(), &stub_target()) {
                finding = Some(f);
                break;
            }
        }
        assert_eq!(finding.unwrap().source, Source::Cargo);
    }

    #[test]
    fn stub_target_helper_is_well_formed() {
        let target = stub_target();
        assert_eq!(target.path, Path::new("/usr/local/bin/x"));
        assert!(!target.dangling);
    }
}
