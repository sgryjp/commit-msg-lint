use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn main() {
    let manifest_dir =
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR");
    std::env::set_current_dir(&manifest_dir).expect("change to package directory");

    let cases_root = Path::new("tests/cases");
    println!("cargo:rerun-if-changed={}", cases_root.display());

    let mut cases = Vec::new();
    let mut identifiers = BTreeSet::new();
    for path in glob::glob("tests/cases/*").expect("valid fixture glob") {
        let path = path.expect("read fixture path");
        let metadata = fs::symlink_metadata(&path).unwrap_or_else(|error| {
            panic!("cannot inspect fixture path {}: {error}", path.display())
        });
        if metadata.file_type().is_symlink() {
            panic!("fixture path must not be a symlink: {}", path.display());
        }
        if metadata.is_file() {
            continue;
        }
        if !metadata.is_dir() {
            panic!(
                "fixture root entries must be regular files or case directories: {}",
                path.display()
            );
        }

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_else(|| panic!("fixture directory name is not UTF-8: {}", path.display()))
            .to_owned();
        let identifier = case_identifier(&name);
        if !identifiers.insert(identifier.clone()) {
            panic!("fixture directory names collide after test-name normalization: {name}");
        }

        println!("cargo:rerun-if-changed={}", path.display());
        for entry in fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("cannot read fixture case {name}: {error}"))
        {
            let entry =
                entry.unwrap_or_else(|error| panic!("cannot read fixture case {name}: {error}"));
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
        cases.push((name, identifier));
    }

    if cases.is_empty() {
        panic!("no fixture cases found in {}", cases_root.display());
    }

    cases.sort_by(|left, right| left.0.cmp(&right.0));
    let mut generated = String::new();
    generated.push_str("pub const DISCOVERED_CASES: &[&str] = &[\n");
    for (name, _) in &cases {
        generated.push_str(&format!("    {name:?},\n"));
    }
    generated.push_str("];\n\n#[rstest::rstest]\n");
    for (name, identifier) in cases {
        generated.push_str(&format!("#[case::{identifier}({name:?})]\n"));
    }
    generated.push_str("fn fixture_cases(#[case] name: &str) {\n    run_fixture_case(name);\n}\n");

    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("Cargo sets OUT_DIR"));
    fs::write(output.join("fixture_cases.rs"), generated).expect("write generated fixture tests");
}

fn case_identifier(name: &str) -> String {
    let mut identifier = String::from("case_");
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            identifier.push(character);
        } else {
            identifier.push('_');
        }
    }
    identifier
}
