//! `gem`: RubyGems installs, identified by `/gems/<name>-<ver>/` (the gem's
//! own directory) or `/gems/<ver>/bin/` (a RubyGems binstub directory,
//! which carries no name) anywhere in a hop's path. SPEC.md §5.1 row 8.

use super::{Detector, Env, Finding, Source};
use crate::resolve::Target;
use serde_json::Map;
use std::path::{Component, Path};

pub struct Gem;

struct Match {
    name: Option<String>,
    version: Option<String>,
}

/// Split `<name>-<ver>` on the last hyphen that is immediately followed by
/// a digit — gem names may themselves contain hyphens (`rubocop-rails`),
/// but a gem version always starts with a digit.
fn split_name_version(segment: &str) -> Option<(String, String)> {
    let bytes = segment.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'-' && bytes.get(i + 1).is_some_and(u8::is_ascii_digit) {
            let name = &segment[..i];
            let version = &segment[i + 1..];
            if !name.is_empty() {
                return Some((name.to_string(), version.to_string()));
            }
        }
    }
    None
}

fn find_match(hop: &Path) -> Option<Match> {
    let components: Vec<Component> = hop.components().collect();

    for (i, comp) in components.iter().enumerate() {
        if comp.as_os_str() != "gems" {
            continue;
        }

        let Component::Normal(next) = components.get(i + 1)? else {
            continue;
        };
        let next = next.to_str()?;

        // `/gems/<ver>/bin/<cmd>`: a RubyGems binstub directory. `next` is
        // just a bare version (all-digits/dots), no name.
        if components.get(i + 2).and_then(|c| c.as_os_str().to_str()) == Some("bin")
            && next.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            return Some(Match {
                name: None,
                version: None,
            });
        }

        if let Some((name, version)) = split_name_version(next) {
            return Some(Match {
                name: Some(name),
                version: Some(version),
            });
        }
    }
    None
}

impl Detector for Gem {
    fn detect(&self, _env: &Env, target: &Target) -> Option<Finding> {
        for hop in &target.hops_abs {
            if let Some(m) = find_match(hop) {
                return Some(Finding {
                    source: Source::Gem,
                    package: m.name,
                    version: m.version,
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
    fn simple_name_and_version() {
        let target = target_for("/usr/local/lib/ruby/gems/3.3.0/gems/bundler-2.5.6/exe/bundler");
        let finding = Gem.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::Gem);
        assert_eq!(finding.package.as_deref(), Some("bundler"));
        assert_eq!(finding.version.as_deref(), Some("2.5.6"));
    }

    #[test]
    fn name_containing_hyphens() {
        let target = target_for(
            "/usr/local/lib/ruby/gems/3.3.0/gems/rubocop-rails-2.24.0/exe/rubocop-rails",
        );
        let finding = Gem.detect(&env(), &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("rubocop-rails"));
        assert_eq!(finding.version.as_deref(), Some("2.24.0"));
    }

    #[test]
    fn version_with_platform_suffix() {
        let target =
            target_for("/usr/local/lib/ruby/gems/3.3.0/gems/nokogiri-1.16.0-x86_64-darwin/lib");
        let finding = Gem.detect(&env(), &target).unwrap();
        assert_eq!(finding.package.as_deref(), Some("nokogiri"));
        assert_eq!(finding.version.as_deref(), Some("1.16.0-x86_64-darwin"));
    }

    #[test]
    fn binstub_directory_has_no_name_or_version() {
        let target = target_for("/usr/local/lib/ruby/gems/3.3.0/bin/bundler");
        let finding = Gem.detect(&env(), &target).unwrap();
        assert_eq!(finding.source, Source::Gem);
        assert_eq!(finding.package, None);
        assert_eq!(finding.version, None);
    }

    #[test]
    fn unrelated_path_does_not_match() {
        let target = target_for("/usr/local/bin/jq");
        assert!(Gem.detect(&env(), &target).is_none());
    }

    #[test]
    fn gems_without_following_segment_does_not_match() {
        let target = target_for("/usr/local/lib/ruby/gems");
        assert!(Gem.detect(&env(), &target).is_none());
    }

    #[test]
    fn gems_segment_without_digit_after_hyphen_does_not_match() {
        // "foo-bar" has a hyphen but nothing digit-led after it.
        let target = target_for("/usr/local/lib/ruby/gems/3.3.0/gems/foo-bar/exe/foo-bar");
        assert!(Gem.detect(&env(), &target).is_none());
    }
}
