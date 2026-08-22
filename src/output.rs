//! Rendering a [`Finding`] as text (default one-line / `--detail`) or JSON.
//! SPEC.md §3.1, §3.2, §3.3, §4.

use crate::detect::{Finding, Source};
use crate::resolve::{Shebang, Target};
use serde_json::{Value, json};

/// Render `<name>: <source-description>` per SPEC.md §3.1, §4.
pub fn render_line(name: &str, finding: &Finding) -> String {
    format!("{name}: {}", render_source(finding))
}

/// Render the full `--detail` block: the one-line summary followed by
/// indented `path:` / `symlink:` / `shebang:` lines. SPEC.md §3.2.
pub fn render_detail(name: &str, finding: &Finding, target: &Target) -> String {
    format!(
        "{}\n  path:    {}\n  symlink: {}\n  shebang: {}",
        render_line(name, finding),
        target.path.display(),
        render_symlink(target),
        render_shebang_detail(target),
    )
}

/// `symlink:` value: `(none)` if not a symlink, otherwise every hop as
/// written on disk joined by ` -> `, ending with the canonicalized
/// absolute path, with ` (dangling)` appended for a broken link.
/// SPEC.md §3.2.
fn render_symlink(target: &Target) -> String {
    if target.hops_display.len() <= 1 {
        return "(none)".to_string();
    }

    let mut parts = target.hops_display.clone();
    parts.push(target.resolved_path.to_string_lossy().into_owned());
    let mut rendered = parts.join(" -> ");
    if target.dangling {
        rendered.push_str(" (dangling)");
    }
    rendered
}

/// `shebang:` value for `--detail`: the literal line, `(binary)`, or
/// `(unreadable)`. SPEC.md §3.2.
fn render_shebang_detail(target: &Target) -> String {
    match &target.shebang {
        Shebang::Line(line) => line.clone(),
        Shebang::Binary => "(binary)".to_string(),
        Shebang::Unreadable => "(unreadable)".to_string(),
    }
}

/// Render the full `--json` object. SPEC.md §3.3.
pub fn render_json(name: &str, finding: &Finding, target: &Target) -> Value {
    let symlink_chain: Vec<String> = if target.hops_display.len() <= 1 {
        Vec::new()
    } else {
        target.hops_display.clone()
    };

    let shebang = match &target.shebang {
        Shebang::Line(line) => Value::String(line.clone()),
        Shebang::Binary | Shebang::Unreadable => Value::Null,
    };

    json!({
        "name": name,
        "path": target.path.to_string_lossy(),
        "resolved_path": target.resolved_path.to_string_lossy(),
        "symlink_chain": symlink_chain,
        "dangling": target.dangling,
        "shebang": shebang,
        "source": finding.source,
        "package": finding.package,
        "version": finding.version,
        "detail": finding.detail,
    })
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
    use std::path::PathBuf;

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

    // ---- render_detail() ----

    fn non_symlink_target(path: &str, shebang: Shebang) -> Target {
        Target {
            path: PathBuf::from(path),
            hops_display: vec![path.to_string()],
            hops_abs: vec![PathBuf::from(path)],
            resolved_path: PathBuf::from(path),
            dangling: false,
            shebang,
        }
    }

    #[test]
    fn detail_non_symlink_binary() {
        let target = non_symlink_target("/bin/ls", Shebang::Binary);
        let f = finding(Source::MacosSystem, None, None);
        assert_eq!(
            render_detail("ls", &f, &target),
            "ls: macos-system\n  path:    /bin/ls\n  symlink: (none)\n  shebang: (binary)"
        );
    }

    #[test]
    fn detail_symlink_chain_ends_with_canonical_path() {
        let target = Target {
            path: PathBuf::from("/usr/local/bin/jq"),
            hops_display: vec![
                "/usr/local/bin/jq".to_string(),
                "../Cellar/jq/1.7.1/bin/jq".to_string(),
            ],
            hops_abs: vec![
                PathBuf::from("/usr/local/bin/jq"),
                PathBuf::from("/usr/local/Cellar/jq/1.7.1/bin/jq"),
            ],
            resolved_path: PathBuf::from("/usr/local/Cellar/jq/1.7.1/bin/jq"),
            dangling: false,
            shebang: Shebang::Binary,
        };
        let mut f = finding(Source::Homebrew, Some("jq"), Some("1.7.1"));
        f.detail.insert("kind".to_string(), json!("formula"));

        assert_eq!(
            render_detail("jq", &f, &target),
            "jq: homebrew formula jq 1.7.1\n  \
             path:    /usr/local/bin/jq\n  \
             symlink: /usr/local/bin/jq -> ../Cellar/jq/1.7.1/bin/jq -> /usr/local/Cellar/jq/1.7.1/bin/jq\n  \
             shebang: (binary)"
        );
    }

    #[test]
    fn detail_shebang_line_shown_verbatim() {
        let target = non_symlink_target(
            "/usr/local/bin/black",
            Shebang::Line("#!/usr/local/opt/python@3.12/bin/python3.12".to_string()),
        );
        let mut f = finding(Source::Pip, None, None);
        f.detail.insert(
            "python".to_string(),
            json!("/usr/local/opt/python@3.12/bin/python3.12"),
        );
        assert_eq!(
            render_detail("black", &f, &target),
            "black: pip (python: /usr/local/opt/python@3.12/bin/python3.12)\n  \
             path:    /usr/local/bin/black\n  \
             symlink: (none)\n  \
             shebang: #!/usr/local/opt/python@3.12/bin/python3.12"
        );
    }

    #[test]
    fn detail_unreadable_shebang() {
        let target = non_symlink_target("/usr/local/bin/secret", Shebang::Unreadable);
        let f = finding(Source::Unknown, None, None);
        assert!(render_detail("secret", &f, &target).ends_with("shebang: (unreadable)"));
    }

    #[test]
    fn detail_dangling_symlink_appends_marker() {
        let target = Target {
            path: PathBuf::from("/usr/local/bin/ghost"),
            hops_display: vec![
                "/usr/local/bin/ghost".to_string(),
                "../Cellar/ghost/1.0/bin/ghost".to_string(),
            ],
            hops_abs: vec![
                PathBuf::from("/usr/local/bin/ghost"),
                PathBuf::from("/usr/local/Cellar/ghost/1.0/bin/ghost"),
            ],
            resolved_path: PathBuf::from("/usr/local/Cellar/ghost/1.0/bin/ghost"),
            dangling: true,
            shebang: Shebang::Unreadable,
        };
        let f = Finding::unknown();
        let rendered = render_detail("ghost", &f, &target);
        assert!(rendered.contains(
            "symlink: /usr/local/bin/ghost -> ../Cellar/ghost/1.0/bin/ghost -> /usr/local/Cellar/ghost/1.0/bin/ghost (dangling)"
        ));
    }

    // ---- render_json() ----

    #[test]
    fn json_non_symlink_has_empty_chain_and_null_shebang() {
        let target = non_symlink_target("/bin/ls", Shebang::Binary);
        let f = finding(Source::MacosSystem, None, None);
        let value = render_json("ls", &f, &target);

        assert_eq!(value["name"], json!("ls"));
        assert_eq!(value["path"], json!("/bin/ls"));
        assert_eq!(value["resolved_path"], json!("/bin/ls"));
        assert_eq!(value["symlink_chain"], json!([]));
        assert_eq!(value["dangling"], json!(false));
        assert_eq!(value["shebang"], Value::Null);
        assert_eq!(value["source"], json!("macos-system"));
        assert_eq!(value["package"], Value::Null);
        assert_eq!(value["version"], Value::Null);
    }

    #[test]
    fn json_symlink_matches_spec_example() {
        let target = Target {
            path: PathBuf::from("/usr/local/bin/jq"),
            hops_display: vec![
                "/usr/local/bin/jq".to_string(),
                "../Cellar/jq/1.7.1/bin/jq".to_string(),
            ],
            hops_abs: vec![
                PathBuf::from("/usr/local/bin/jq"),
                PathBuf::from("/usr/local/Cellar/jq/1.7.1/bin/jq"),
            ],
            resolved_path: PathBuf::from("/usr/local/Cellar/jq/1.7.1/bin/jq"),
            dangling: false,
            shebang: Shebang::Binary,
        };
        let mut f = finding(Source::Homebrew, Some("jq"), Some("1.7.1"));
        f.detail.insert("kind".to_string(), json!("formula"));
        f.detail.insert("prefix".to_string(), json!("/usr/local"));

        let value = render_json("jq", &f, &target);

        assert_eq!(
            value,
            json!({
                "name": "jq",
                "path": "/usr/local/bin/jq",
                "resolved_path": "/usr/local/Cellar/jq/1.7.1/bin/jq",
                "symlink_chain": ["/usr/local/bin/jq", "../Cellar/jq/1.7.1/bin/jq"],
                "dangling": false,
                "shebang": null,
                "source": "homebrew",
                "package": "jq",
                "version": "1.7.1",
                "detail": { "kind": "formula", "prefix": "/usr/local" }
            })
        );
    }

    #[test]
    fn json_shebang_is_full_line_when_present() {
        let target = non_symlink_target(
            "/usr/local/bin/black",
            Shebang::Line("#!/usr/local/opt/python@3.12/bin/python3.12".to_string()),
        );
        let f = finding(Source::Pip, None, None);
        let value = render_json("black", &f, &target);
        assert_eq!(
            value["shebang"],
            json!("#!/usr/local/opt/python@3.12/bin/python3.12")
        );
    }

    #[test]
    fn json_unreadable_shebang_is_null_not_a_string() {
        let target = non_symlink_target("/usr/local/bin/secret", Shebang::Unreadable);
        let f = Finding::unknown();
        let value = render_json("secret", &f, &target);
        assert_eq!(value["shebang"], Value::Null);
    }

    #[test]
    fn json_dangling_true_for_broken_symlink() {
        let target = Target {
            path: PathBuf::from("/usr/local/bin/ghost"),
            hops_display: vec![
                "/usr/local/bin/ghost".to_string(),
                "../Cellar/ghost/1.0/bin/ghost".to_string(),
            ],
            hops_abs: vec![
                PathBuf::from("/usr/local/bin/ghost"),
                PathBuf::from("/usr/local/Cellar/ghost/1.0/bin/ghost"),
            ],
            resolved_path: PathBuf::from("/usr/local/Cellar/ghost/1.0/bin/ghost"),
            dangling: true,
            shebang: Shebang::Unreadable,
        };
        let f = Finding::unknown();
        let value = render_json("ghost", &f, &target);
        assert_eq!(value["dangling"], json!(true));
        assert_eq!(value["source"], json!("unknown"));
    }

    #[test]
    fn json_unknown_source_serializes_as_unknown_string() {
        let target = non_symlink_target("/usr/local/bin/slim", Shebang::Binary);
        let f = Finding::unknown();
        let value = render_json("slim", &f, &target);
        assert_eq!(value["source"], json!("unknown"));
        assert_eq!(value["package"], Value::Null);
        assert_eq!(value["version"], Value::Null);
        assert_eq!(value["detail"], json!({}));
    }
}
