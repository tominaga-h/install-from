//! The result of detection: which mechanism placed a binary, and the
//! source-specific detail that goes with it. SPEC.md §4.

use serde::Serialize;
use serde_json::{Map, Value};

/// Which mechanism placed a binary on disk. Serializes to exactly the
/// twelve strings in SPEC.md §4.
// Variants beyond MacosSystem/Homebrew/Unknown aren't constructed until
// their detectors land in Tasks 6-10.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    MacosSystem,
    XcodeClt,
    VersionManager,
    Homebrew,
    Npm,
    Uv,
    Pipx,
    Gem,
    Cargo,
    Go,
    Pip,
    Unknown,
}

/// The outcome of detection for one target: which source, what package
/// info could be read from the path, and source-specific detail keys.
/// SPEC.md §3.3, §4.
#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub source: Source,
    pub package: Option<String>,
    pub version: Option<String>,
    pub detail: Map<String, Value>,
}

impl Finding {
    /// The `unknown` finding: nothing matched. SPEC.md §4.
    pub fn unknown() -> Self {
        Finding {
            source: Source::Unknown,
            package: None,
            version: None,
            detail: Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_serializes_to_spec_strings() {
        let cases = [
            (Source::MacosSystem, "\"macos-system\""),
            (Source::XcodeClt, "\"xcode-clt\""),
            (Source::VersionManager, "\"version-manager\""),
            (Source::Homebrew, "\"homebrew\""),
            (Source::Npm, "\"npm\""),
            (Source::Uv, "\"uv\""),
            (Source::Pipx, "\"pipx\""),
            (Source::Gem, "\"gem\""),
            (Source::Cargo, "\"cargo\""),
            (Source::Go, "\"go\""),
            (Source::Pip, "\"pip\""),
            (Source::Unknown, "\"unknown\""),
        ];
        for (source, expected) in cases {
            assert_eq!(serde_json::to_string(&source).unwrap(), expected);
        }
    }

    #[test]
    fn unknown_finding_has_no_package_version_and_empty_detail() {
        let finding = Finding::unknown();
        assert_eq!(finding.source, Source::Unknown);
        assert_eq!(finding.package, None);
        assert_eq!(finding.version, None);
        assert!(finding.detail.is_empty());
    }
}
