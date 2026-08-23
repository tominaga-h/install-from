//! Smoke check: `src/` never spawns a subprocess. SPEC.md §1
//! ("Executing any external command ... Never."), §7, §9.10.

use std::fs;
use std::path::Path;

fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn src_never_references_process_command() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src_dir, &mut files);
    assert!(!files.is_empty(), "expected to find .rs files under src/");

    for file in files {
        let contents = fs::read_to_string(&file).unwrap();
        assert!(
            !contents.contains("process::Command") && !contents.contains("Command::new"),
            "{} references a subprocess API, but SPEC.md requires install-from \
             to never spawn external commands",
            file.display()
        );
    }
}
