//! PATH resolution, symlink-chain walking, and shebang parsing.
//! SPEC.md §2.1, §5, §5.2, §6.

use crate::error::ResolveError;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Maximum number of symlink hops to follow before declaring a cycle.
/// SPEC.md §6.
const MAX_HOPS: usize = 40;

/// Maximum number of bytes read while looking for a shebang line.
/// SPEC.md §5.2.
const SHEBANG_READ_CAP: usize = 4096;

/// A file the resolver is willing to classify: a regular file or a
/// (possibly dangling) symlink. Directories, sockets, etc. are rejected
/// during candidate search. Execute permission is never required
/// (SPEC.md §2.1).
fn is_candidate_file_type(path: &Path) -> bool {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            let ft = meta.file_type();
            ft.is_file() || ft.is_symlink()
        }
        Err(_) => false,
    }
}

/// Find candidate absolute paths for `arg`.
///
/// - If `arg` contains `/`, it is treated as a path (relative paths are
///   resolved against `cwd`); exactly one candidate is returned.
/// - Otherwise `arg` is searched across `path_dirs` (in order). With
///   `all == false`, only the first match is returned; with `all == true`,
///   every match is returned in `PATH` order.
///
/// SPEC.md §2.1.
pub fn candidates(
    arg: &str,
    path_dirs: &[PathBuf],
    cwd: &Path,
    all: bool,
) -> Result<Vec<PathBuf>, ResolveError> {
    if arg.contains('/') {
        let path = if Path::new(arg).is_absolute() {
            PathBuf::from(arg)
        } else {
            cwd.join(arg)
        };

        return match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_dir() => Err(ResolveError::NoSuchFile(path)),
            Ok(_) => Ok(vec![path]),
            Err(_) => Err(ResolveError::NoSuchFile(path)),
        };
    }

    let mut found = Vec::new();
    for dir in path_dirs {
        let candidate = dir.join(arg);
        if is_candidate_file_type(&candidate) {
            found.push(candidate);
            if !all {
                break;
            }
        }
    }

    if found.is_empty() {
        return Err(ResolveError::CommandNotFound(arg.to_string()));
    }
    Ok(found)
}

/// State of the shebang line for a resolved target's final file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shebang {
    /// First line starting with `#!`, stored verbatim (including `#!`).
    Line(String),
    /// File does not start with `#!`.
    Binary,
    /// File could not be opened for reading.
    Unreadable,
}

impl Shebang {
    /// The interpreter token: the first whitespace-separated token after
    /// `#!`, or — if that token's basename is `env` — the next token
    /// (skipping a leading `-S` flag). `None` unless this is `Line(..)`.
    /// SPEC.md §5.2.
    // Used by the pip detector (Task 10) and the JSON renderer (Task 5).
    #[allow(dead_code)]
    pub fn interpreter(&self) -> Option<&str> {
        let Shebang::Line(line) = self else {
            return None;
        };
        let rest = line.strip_prefix("#!")?;
        let mut tokens = rest.split_whitespace();
        let first = tokens.next()?;
        let first_basename = first.rsplit('/').next().unwrap_or(first);
        if first_basename == "env" {
            let mut next = tokens.next()?;
            if next == "-S" {
                next = tokens.next()?;
            }
            Some(next)
        } else {
            Some(first)
        }
    }
}

fn read_shebang(path: &Path) -> Shebang {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Shebang::Unreadable,
    };

    let mut buf = vec![0u8; SHEBANG_READ_CAP];
    let n = match file.read(&mut buf) {
        Ok(n) => n,
        Err(_) => return Shebang::Unreadable,
    };
    buf.truncate(n);

    if buf.len() < 2 || &buf[0..2] != b"#!" {
        return Shebang::Binary;
    }

    let line_bytes = buf.split(|&b| b == b'\n').next().unwrap_or(&buf);
    Shebang::Line(String::from_utf8_lossy(line_bytes).into_owned())
}

/// A fully-walked symlink chain for one starting path.
// Only `hops_abs` is read by detectors so far; the rest feed the
// `--detail`/`--json` renderers added in Task 5.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Target {
    /// The starting path, as passed to [`chain`] (absolute).
    pub path: PathBuf,
    /// Every hop as written on disk: `hops_display[0]` is `path` itself;
    /// each subsequent entry is the verbatim `readlink` target of the
    /// previous hop (relative targets are kept relative). Empty beyond the
    /// first entry if `path` is not a symlink.
    pub hops_display: Vec<String>,
    /// The same chain, with every hop absolutized against its parent
    /// directory, for use by detectors. Always has the same length as
    /// `hops_display`.
    pub hops_abs: Vec<PathBuf>,
    /// The final path in the chain, canonicalized components-wise but
    /// *without* requiring the target to exist (so a dangling symlink's
    /// target is still reported). Equal to `path` if not a symlink.
    pub resolved_path: PathBuf,
    /// True if the final hop does not exist on disk.
    pub dangling: bool,
    /// Shebang state of the final file (only meaningful if `!dangling`).
    pub shebang: Shebang,
}

/// Lexically normalize a path: resolve `.` and `..` components without
/// touching the filesystem (the target may not exist, e.g. a dangling
/// symlink). Does not resolve further symlinks.
fn normalize(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                if !matches!(
                    out.components().next_back(),
                    None | Some(Component::RootDir)
                ) {
                    out.pop();
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Walk the symlink chain starting at `path` (must be absolute).
///
/// Every hop is recorded twice: verbatim (`hops_display`, for rendering)
/// and absolutized (`hops_abs`, for detector matching). A dangling link is
/// not an error — it is classified by its target string, with
/// `dangling = true`. A cycle or a chain longer than [`MAX_HOPS`] is an
/// error. SPEC.md §5, §6.
pub fn chain(path: &Path) -> Result<Target, ResolveError> {
    let mut hops_display = vec![path.to_string_lossy().into_owned()];
    let mut hops_abs = vec![path.to_path_buf()];
    let mut current = path.to_path_buf();
    let mut dangling = false;

    loop {
        let meta = match fs::symlink_metadata(&current) {
            Ok(meta) => meta,
            Err(_) => {
                dangling = true;
                break;
            }
        };

        if !meta.file_type().is_symlink() {
            break;
        }

        if hops_abs.len() > MAX_HOPS {
            return Err(ResolveError::ChainTooDeep(path.to_path_buf()));
        }

        let raw_target =
            fs::read_link(&current).map_err(|_| ResolveError::ChainTooDeep(path.to_path_buf()))?;
        let abs_target = if raw_target.is_absolute() {
            normalize(&raw_target)
        } else {
            let parent = current.parent().unwrap_or_else(|| Path::new("/"));
            normalize(&parent.join(&raw_target))
        };

        if hops_abs.contains(&abs_target) {
            return Err(ResolveError::ChainTooDeep(path.to_path_buf()));
        }

        hops_display.push(raw_target.to_string_lossy().into_owned());
        hops_abs.push(abs_target.clone());
        current = abs_target;
    }

    let resolved_path = current;
    let shebang = if dangling {
        Shebang::Unreadable
    } else {
        read_shebang(&resolved_path)
    };

    Ok(Target {
        path: path.to_path_buf(),
        hops_display,
        hops_abs,
        resolved_path,
        dangling,
        shebang,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use tempfile::tempdir;

    fn write_file(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    // ---- candidates() ----

    #[test]
    fn candidates_path_arg_absolute() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("tool");
        write_file(&file, b"x");

        let result = candidates(file.to_str().unwrap(), &[], tmp.path(), false).unwrap();
        assert_eq!(result, vec![file]);
    }

    #[test]
    fn candidates_path_arg_relative_uses_cwd() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("tool");
        write_file(&file, b"x");

        let result = candidates("./tool", &[], tmp.path(), false).unwrap();
        assert_eq!(result, vec![file]);
    }

    #[test]
    fn candidates_path_arg_missing_is_error() {
        let tmp = tempdir().unwrap();
        let err = candidates("./nope", &[], tmp.path(), false).unwrap_err();
        assert!(matches!(err, ResolveError::NoSuchFile(_)));
    }

    #[test]
    fn candidates_path_arg_directory_is_error() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("subdir");
        fs::create_dir(&dir).unwrap();

        let err = candidates(dir.to_str().unwrap(), &[], tmp.path(), false).unwrap_err();
        assert!(matches!(err, ResolveError::NoSuchFile(_)));
    }

    #[test]
    fn candidates_name_first_match_only() {
        let tmp = tempdir().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        write_file(&dir_a.join("jq"), b"a");
        write_file(&dir_b.join("jq"), b"b");

        let dirs = vec![dir_a.clone(), dir_b.clone()];
        let result = candidates("jq", &dirs, tmp.path(), false).unwrap();
        assert_eq!(result, vec![dir_a.join("jq")]);
    }

    #[test]
    fn candidates_name_all_matches_in_path_order() {
        let tmp = tempdir().unwrap();
        let dir_a = tmp.path().join("a");
        let dir_b = tmp.path().join("b");
        write_file(&dir_a.join("jq"), b"a");
        write_file(&dir_b.join("jq"), b"b");

        let dirs = vec![dir_a.clone(), dir_b.clone()];
        let result = candidates("jq", &dirs, tmp.path(), true).unwrap();
        assert_eq!(result, vec![dir_a.join("jq"), dir_b.join("jq")]);
    }

    #[test]
    fn candidates_name_not_found() {
        let tmp = tempdir().unwrap();
        let err = candidates("nope", &[], tmp.path(), false).unwrap_err();
        assert!(matches!(err, ResolveError::CommandNotFound(name) if name == "nope"));
    }

    #[test]
    fn candidates_name_skips_directories_on_path() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("bin");
        fs::create_dir_all(dir.join("jq")).unwrap(); // "jq" is a directory here

        let err = candidates("jq", &[dir], tmp.path(), false).unwrap_err();
        assert!(matches!(err, ResolveError::CommandNotFound(_)));
    }

    #[test]
    fn candidates_name_no_exec_bit_required() {
        let tmp = tempdir().unwrap();
        let dir = tmp.path().join("bin");
        let file = dir.join("jq");
        write_file(&file, b"x");
        // freshly written files are not executable by default in this setup;
        // no chmod is performed, proving exec bit is not required.

        let result = candidates("jq", &[dir], tmp.path(), false).unwrap();
        assert_eq!(result, vec![file]);
    }

    // ---- chain() ----

    #[test]
    fn chain_non_symlink_regular_file() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("jq");
        write_file(&file, b"\x7fELF binary data");

        let target = chain(&file).unwrap();
        assert_eq!(target.hops_display, vec![file.to_string_lossy()]);
        assert_eq!(target.hops_abs, vec![file.clone()]);
        assert_eq!(target.resolved_path, file);
        assert!(!target.dangling);
        assert_eq!(target.shebang, Shebang::Binary);
    }

    #[test]
    fn chain_relative_symlink_hop() {
        let tmp = tempdir().unwrap();
        let real = tmp.path().join("Cellar/jq/1.7.1/bin/jq");
        write_file(&real, b"binary");
        let link = tmp.path().join("bin/jq");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../Cellar/jq/1.7.1/bin/jq", &link).unwrap();

        let target = chain(&link).unwrap();
        assert_eq!(
            target.hops_display,
            vec![
                link.to_string_lossy().into_owned(),
                "../Cellar/jq/1.7.1/bin/jq".to_string(),
            ]
        );
        assert_eq!(target.hops_abs, vec![link, real.clone()]);
        assert_eq!(target.resolved_path, real);
        assert!(!target.dangling);
    }

    #[test]
    fn chain_two_stage_symlink() {
        let tmp = tempdir().unwrap();
        let real = tmp.path().join("Cellar/jq/1.7.1/bin/jq");
        write_file(&real, b"binary");
        let opt_link = tmp.path().join("opt/jq/bin/jq");
        fs::create_dir_all(opt_link.parent().unwrap()).unwrap();
        symlink("../../../Cellar/jq/1.7.1/bin/jq", &opt_link).unwrap();
        let bin_link = tmp.path().join("bin/jq");
        fs::create_dir_all(bin_link.parent().unwrap()).unwrap();
        symlink("../opt/jq/bin/jq", &bin_link).unwrap();

        let target = chain(&bin_link).unwrap();
        assert_eq!(target.hops_abs, vec![bin_link, opt_link, real.clone()]);
        assert_eq!(target.resolved_path, real);
    }

    #[test]
    fn chain_dangling_symlink_classified_by_target_string() {
        let tmp = tempdir().unwrap();
        let link = tmp.path().join("bin/ghost");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        symlink("../Cellar/ghost/1.0/bin/ghost", &link).unwrap();

        let target = chain(&link).unwrap();
        assert!(target.dangling);
        assert_eq!(
            target.resolved_path,
            tmp.path().join("Cellar/ghost/1.0/bin/ghost")
        );
        assert_eq!(target.shebang, Shebang::Unreadable);
    }

    #[test]
    fn chain_loop_is_error() {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        symlink(&b, &a).unwrap();
        symlink(&a, &b).unwrap();

        let err = chain(&a).unwrap_err();
        assert!(matches!(err, ResolveError::ChainTooDeep(_)));
    }

    #[test]
    fn chain_over_40_hops_is_error() {
        let tmp = tempdir().unwrap();
        let base = tmp.path();
        let final_file = base.join("hop_final");
        write_file(&final_file, b"end");

        let mut prev = final_file.clone();
        for i in 0..45 {
            let link = base.join(format!("hop_{i}"));
            symlink(&prev, &link).unwrap();
            prev = link;
        }

        let err = chain(&prev).unwrap_err();
        assert!(matches!(err, ResolveError::ChainTooDeep(_)));
    }

    #[test]
    fn chain_self_loop_is_error() {
        let tmp = tempdir().unwrap();
        let a = tmp.path().join("a");
        symlink(&a, &a).unwrap();

        let err = chain(&a).unwrap_err();
        assert!(matches!(err, ResolveError::ChainTooDeep(_)));
    }

    // ---- shebang ----

    #[test]
    fn shebang_plain_interpreter() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("black");
        write_file(
            &file,
            b"#!/usr/local/opt/python@3.12/bin/python3.12\nprint()\n",
        );

        let target = chain(&file).unwrap();
        assert_eq!(
            target.shebang,
            Shebang::Line("#!/usr/local/opt/python@3.12/bin/python3.12".to_string())
        );
        assert_eq!(
            target.shebang.interpreter(),
            Some("/usr/local/opt/python@3.12/bin/python3.12")
        );
    }

    #[test]
    fn shebang_env_form() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("script");
        write_file(&file, b"#!/usr/bin/env python3\n");

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang.interpreter(), Some("python3"));
    }

    #[test]
    fn shebang_env_dash_s_form() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("script");
        write_file(&file, b"#!/usr/bin/env -S python3 -u\n");

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang.interpreter(), Some("python3"));
    }

    #[test]
    fn shebang_binary_file() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("jq");
        write_file(&file, &[0x7f, b'E', b'L', b'F', 0, 0, 0, 0]);

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang, Shebang::Binary);
        assert_eq!(target.shebang.interpreter(), None);
    }

    #[test]
    fn shebang_short_file_not_binary_crash() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("x");
        write_file(&file, b"a");

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang, Shebang::Binary);
    }

    #[test]
    fn shebang_unreadable_permission_denied() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("secret");
        write_file(&file, b"#!/bin/sh\n");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang, Shebang::Unreadable);

        // restore so tempdir cleanup can remove it
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    }

    #[test]
    fn shebang_empty_file_is_binary() {
        let tmp = tempdir().unwrap();
        let file = tmp.path().join("empty");
        write_file(&file, b"");

        let target = chain(&file).unwrap();
        assert_eq!(target.shebang, Shebang::Binary);
    }
}
