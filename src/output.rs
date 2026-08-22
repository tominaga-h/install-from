//! Rendering a [`Finding`] as the one-line default text output.
//! SPEC.md §3.1, §4.
//!
//! `--detail` and `--json` renderers land in Task 5.

use crate::detect::{Finding, Source};

/// Render `<name>: <source-description>` per SPEC.md §3.1, §4.
pub fn render_line(name: &str, finding: &Finding) -> String {
    format!("{name}: {}", render_source(finding))
}

fn render_source(finding: &Finding) -> String {
    match finding.source {
        Source::MacosSystem => {
            let shell_command = finding
                .detail
                .get("shell_command")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if shell_command {
                "macos-system (shell command)".to_string()
            } else {
                "macos-system".to_string()
            }
        }
        Source::XcodeClt => "xcode-clt".to_string(),
        Source::VersionManager => {
            let manager = finding
                .detail
                .get("manager")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("version-manager {manager} shim")
        }
        Source::Homebrew => {
            let kind = finding
                .detail
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("formula");
            let name = finding.package.as_deref().unwrap_or("?");
            let version = finding.version.as_deref().unwrap_or("?");
            format!("homebrew {kind} {name} {version}")
        }
        Source::Npm => {
            let client = finding
                .detail
                .get("client")
                .and_then(|v| v.as_str())
                .unwrap_or("npm");
            let pkg = finding.package.as_deref().unwrap_or("?");
            format!("{client} {pkg}")
        }
        Source::Uv => {
            let pkg = finding.package.as_deref().unwrap_or("?");
            format!("uv {pkg}")
        }
        Source::Pipx => {
            let pkg = finding.package.as_deref().unwrap_or("?");
            format!("pipx {pkg}")
        }
        Source::Gem => {
            let pkg = finding.package.as_deref().unwrap_or("?");
            match &finding.version {
                Some(version) => format!("gem {pkg} {version}"),
                None => format!("gem {pkg}"),
            }
        }
        Source::Cargo => "cargo".to_string(),
        Source::Go => "go".to_string(),
        Source::Pip => {
            let python = finding
                .detail
                .get("python")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            format!("pip (python: {python})")
        }
        Source::Unknown => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Map, json};

    fn finding(source: Source, package: Option<&str>, version: Option<&str>) -> Finding {
        Finding {
            source,
            package: package.map(str::to_string),
            version: version.map(str::to_string),
            detail: Map::new(),
        }
    }

    #[test]
    fn macos_system_plain() {
        let f = finding(Source::MacosSystem, None, None);
        assert_eq!(render_line("ls", &f), "ls: macos-system");
    }

    #[test]
    fn macos_system_shell_command() {
        let mut f = finding(Source::MacosSystem, None, None);
        f.detail.insert("shell_command".to_string(), json!(true));
        assert_eq!(render_line("git", &f), "git: macos-system (shell command)");
    }

    #[test]
    fn homebrew_formula() {
        let mut f = finding(Source::Homebrew, Some("jq"), Some("1.7.1"));
        f.detail.insert("kind".to_string(), json!("formula"));
        assert_eq!(render_line("jq", &f), "jq: homebrew formula jq 1.7.1");
    }

    #[test]
    fn homebrew_cask() {
        let mut f = finding(Source::Homebrew, Some("some-app"), Some("2.0.0"));
        f.detail.insert("kind".to_string(), json!("cask"));
        assert_eq!(
            render_line("some-app", &f),
            "some-app: homebrew cask some-app 2.0.0"
        );
    }

    #[test]
    fn version_manager_shim() {
        let mut f = finding(Source::VersionManager, None, None);
        f.detail.insert("manager".to_string(), json!("asdf"));
        assert_eq!(render_line("node", &f), "node: version-manager asdf shim");
    }

    #[test]
    fn pip_with_interpreter() {
        let mut f = finding(Source::Pip, None, None);
        f.detail.insert(
            "python".to_string(),
            json!("/usr/local/opt/python@3.12/bin/python3.12"),
        );
        assert_eq!(
            render_line("black", &f),
            "black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)"
        );
    }

    #[test]
    fn cargo_and_go_have_no_extra_words() {
        assert_eq!(
            render_line("rg", &finding(Source::Cargo, None, None)),
            "rg: cargo"
        );
        assert_eq!(
            render_line("gopls", &finding(Source::Go, None, None)),
            "gopls: go"
        );
    }

    #[test]
    fn npm_default_client() {
        let f = finding(Source::Npm, Some("prettier"), None);
        assert_eq!(render_line("prettier", &f), "prettier: npm prettier");
    }

    #[test]
    fn npm_yarn_client() {
        let mut f = finding(Source::Npm, Some("prettier"), None);
        f.detail.insert("client".to_string(), json!("yarn"));
        assert_eq!(render_line("prettier", &f), "prettier: yarn prettier");
    }

    #[test]
    fn gem_with_version() {
        let f = finding(Source::Gem, Some("bundler"), Some("2.5.6"));
        assert_eq!(render_line("bundler", &f), "bundler: gem bundler 2.5.6");
    }

    #[test]
    fn gem_binstub_without_version() {
        let f = finding(Source::Gem, Some("bundler"), None);
        assert_eq!(render_line("bundler", &f), "bundler: gem bundler");
    }

    #[test]
    fn unknown_renders_unknown() {
        let f = Finding::unknown();
        assert_eq!(render_line("slim", &f), "slim: unknown");
    }
}
