use std::fs;
use std::path::PathBuf;

#[test]
fn objetivo_files_use_expected_extensions_and_are_indexed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/objetivo");
    let index = fs::read_to_string(root.join("INDEX.md")).expect("INDEX.md exists");
    for entry in fs::read_dir(&root).expect("objective corpus is readable") {
        let path = entry.expect("directory entry is readable").path();
        let name = path.file_name().unwrap().to_string_lossy();
        if name.ends_with(".nx") {
            panic!("legacy .nx file in objective corpus: {name}");
        }
        let allowed = name == "INDEX.md"
            || name.ends_with(".n")
            || name.ends_with(".env.json")
            || name.ends_with(".out");
        assert!(allowed, "unexpected objective corpus file: {name}");
        if name.ends_with(".n") {
            assert!(
                index.contains(&*name),
                "program missing from INDEX.md: {name}"
            );
        }
    }
}
