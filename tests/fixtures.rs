use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaseManifest {
    args: Vec<String>,
    exit_code: u8,
}

#[derive(Debug)]
struct FixtureCase {
    args: Vec<String>,
    exit_code: u8,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    files: Vec<(PathBuf, PathBuf)>,
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/cases")
        .join(name)
}

fn load_fixture(name: &str, directory: &Path) -> Result<FixtureCase, String> {
    let case = format!("case {name}");
    let metadata = fs::symlink_metadata(directory)
        .map_err(|error| format!("{case}: cannot inspect {}: {error}", directory.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(format!("{case}: fixture path must be a real directory"));
    }

    let mut files = Vec::new();
    let mut names = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("{case}: cannot read {}: {error}", directory.display()))?
    {
        let entry =
            entry.map_err(|error| format!("{case}: cannot read directory entry: {error}"))?;
        let file_type = entry.file_type().map_err(|error| {
            format!("{case}: cannot inspect {}: {error}", entry.path().display())
        })?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(format!(
                "{case}: fixture entries must be regular files directly under the case directory: {}",
                entry.path().display()
            ));
        }
        names.push(entry.file_name());
        files.push((entry.path(), PathBuf::from(entry.file_name())));
    }

    for required in ["message.txt", "case.toml", "stdout.txt", "stderr.txt"] {
        if !names.iter().any(|name| name == required) {
            return Err(format!("{case}: missing required file {required}"));
        }
    }

    let manifest_path = directory.join("case.toml");
    let manifest_text = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("{case}: cannot read case.toml: {error}"))?;
    let manifest: CaseManifest = toml::from_str(&manifest_text)
        .map_err(|error| format!("{case}: invalid case.toml: {error}"))?;

    let read_expected = |filename: &str| {
        fs::read(directory.join(filename))
            .map_err(|error| format!("{case}: cannot read {filename}: {error}"))
    };
    Ok(FixtureCase {
        args: manifest.args,
        exit_code: manifest.exit_code,
        stdout: read_expected("stdout.txt")?,
        stderr: read_expected("stderr.txt")?,
        files,
    })
}

fn run_fixture_case(name: &str) {
    let case = load_fixture(name, &fixture_path(name)).unwrap_or_else(|error| panic!("{error}"));
    let temporary = tempfile::Builder::new()
        .prefix("commit-msg-lint-case-")
        .tempdir()
        .unwrap_or_else(|error| panic!("case {name}: cannot create temporary directory: {error}"));
    let working_directory = temporary
        .path()
        .canonicalize()
        .unwrap_or_else(|error| panic!("case {name}: cannot resolve temporary directory: {error}"));
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .expect("resolve package directory");
    assert!(
        !working_directory.starts_with(&repository),
        "case {name}: temporary working directory must be outside the repository"
    );

    for (source, destination) in &case.files {
        fs::copy(source, working_directory.join(destination)).unwrap_or_else(|error| {
            panic!("case {name}: cannot copy {}: {error}", source.display())
        });
    }

    let home = working_directory.join("isolated-home");
    fs::create_dir(&home)
        .unwrap_or_else(|error| panic!("case {name}: cannot create isolated home: {error}"));
    let output = execute_cli(name, &case.args, &working_directory, &home);

    assert_eq!(
        output.status.code(),
        Some(i32::from(case.exit_code)),
        "case {name}: unexpected exit status; stderr: {:?}",
        output.stderr
    );
    assert_eq!(output.stdout, case.stdout, "case {name}: stdout differs");
    assert_eq!(output.stderr, case.stderr, "case {name}: stderr differs");
}

fn git_ceiling_path(ceiling: &Path) -> Option<&Path> {
    let separator = if cfg!(windows) { b';' } else { b':' };
    (!ceiling.as_os_str().as_encoded_bytes().contains(&separator)).then_some(ceiling)
}

fn execute_cli(name: &str, args: &[String], working_directory: &Path, home: &Path) -> Output {
    let stdin = File::open(working_directory.join("message.txt")).unwrap_or_else(|error| {
        panic!("case {name}: cannot open message.txt for standard input: {error}")
    });
    let mut command = Command::new(env!("CARGO_BIN_EXE_commit-msg-lint"));
    command
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_GLOBAL", home.join("no-global-config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .current_dir(working_directory);
    let ceiling = working_directory
        .parent()
        .expect("temporary working directory has a parent");
    if let Some(ceiling_path) = git_ceiling_path(ceiling) {
        command.env("GIT_CEILING_DIRECTORIES", ceiling_path);
    } else {
        // Git cannot represent a ceiling path containing its path-list separator.
        command.env("GIT_DIR", home.join("no-repository"));
    }
    command
        .args(args)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
    }

    let child = command
        .spawn()
        .unwrap_or_else(|error| panic!("case {name}: cannot launch CLI: {error}"));
    child
        .wait_with_output()
        .unwrap_or_else(|error| panic!("case {name}: cannot collect CLI output: {error}"))
}

include!(concat!(env!("OUT_DIR"), "/fixture_cases.rs"));

#[test]
fn compile_time_discovery_includes_every_case_directory() {
    let cases_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases");
    let mut current_cases = fs::read_dir(&cases_root)
        .unwrap_or_else(|error| panic!("cannot read fixture root: {error}"))
        .map(|entry| {
            let entry = entry.expect("read fixture root entry");
            let file_type = entry.file_type().expect("inspect fixture root entry");
            if file_type.is_dir() {
                Some(
                    entry
                        .file_name()
                        .into_string()
                        .expect("fixture directory names are UTF-8"),
                )
            } else {
                assert!(
                    file_type.is_file(),
                    "unexpected fixture root entry: {}",
                    entry.path().display()
                );
                None
            }
        })
        .flatten()
        .collect::<Vec<_>>();
    let mut discovered = DISCOVERED_CASES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    current_cases.sort();
    discovered.sort();
    assert_eq!(
        discovered, current_cases,
        "fixture case discovery is stale or incomplete"
    );
}

#[cfg(test)]
mod runner_validation_tests {
    use super::*;
    fn write_minimum_fixture(directory: &Path, manifest: &str) {
        fs::create_dir_all(directory).unwrap();
        fs::write(directory.join("message.txt"), b"feat: change\n").unwrap();
        fs::write(directory.join("case.toml"), manifest).unwrap();
        fs::write(directory.join("stdout.txt"), b"").unwrap();
        fs::write(directory.join("stderr.txt"), b"").unwrap();
    }

    #[test]
    fn help_exits_cleanly_with_large_stdin_file() {
        let temporary = tempfile::tempdir().unwrap();
        let working_directory = temporary.path().canonicalize().unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .canonicalize()
            .unwrap();
        assert!(!working_directory.starts_with(repository));
        let home = working_directory.join("isolated-home");
        fs::create_dir(&home).unwrap();
        fs::write(
            working_directory.join("message.txt"),
            vec![b'x'; 16 * 1024 * 1024],
        )
        .unwrap();

        let output = execute_cli("large-help", &["--help".into()], &working_directory, &home);

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            output.stdout,
            b"Usage: commit-msg-lint [--edit <path>] [--config <path>] [--strict]\n"
        );
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn git_ceiling_accepts_the_non_separator_character_and_rejects_the_separator() {
        #[cfg(windows)]
        let (representable, unrepresentable) =
            (r"C:\fixtures\repository", r"C:\fixtures\repo;name");
        #[cfg(not(windows))]
        let (representable, unrepresentable) = ("/fixtures/repo;name", "/fixtures/repo:name");

        let ceiling = Path::new(representable);
        assert_eq!(git_ceiling_path(ceiling), Some(ceiling));
        assert_eq!(git_ceiling_path(Path::new(unrepresentable)), None);
    }

    #[rstest::rstest]
    #[case::ordinary_parent(false)]
    #[case::path_list_separator_in_parent(true)]
    fn default_config_ignores_ancestor_repository_configuration(
        #[case] include_path_list_separator: bool,
    ) {
        let temporary = tempfile::tempdir().unwrap();
        let separator = if cfg!(windows) { ';' } else { ':' };
        let repository_name = if include_path_list_separator {
            format!("ancestor{separator}repository")
        } else {
            "ancestor-repository".to_owned()
        };
        let repository = temporary.path().join(repository_name);
        fs::create_dir(&repository).unwrap();
        initialize_git_repository(&repository);
        fs::write(
            repository.join("commit-msg-lint.toml"),
            "version = 1\n[conventional]\ntypes = ['fix']\n",
        )
        .unwrap();

        let working_directory = repository.join("case-work");
        fs::create_dir(&working_directory).unwrap();
        let working_directory = working_directory.canonicalize().unwrap();
        fs::write(working_directory.join("message.txt"), b"feat: add parser\n").unwrap();
        let home = working_directory.join("isolated-home");
        fs::create_dir(&home).unwrap();
        let discovered_root = git_command_in(&working_directory, &["rev-parse", "--show-toplevel"])
            .output()
            .unwrap();
        assert!(discovered_root.status.success());
        let discovered_root =
            PathBuf::from(String::from_utf8_lossy(&discovered_root.stdout).trim())
                .canonicalize()
                .unwrap();
        assert_eq!(discovered_root, repository.canonicalize().unwrap());

        let ceiling = working_directory.parent().unwrap();
        assert_eq!(
            git_ceiling_path(ceiling).is_none(),
            include_path_list_separator,
            "test path does not exercise the requested Git isolation case: {}",
            ceiling.display()
        );

        let output = execute_cli("ancestor-repository-config", &[], &working_directory, &home);

        assert_eq!(output.status.code(), Some(0));
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }

    fn initialize_git_repository(directory: &Path) {
        let status = git_command_in(directory, &["init", "-q"]).status().unwrap();
        assert!(status.success(), "git init failed");
    }

    fn git_command_in(directory: &Path, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .args(args)
            .current_dir(directory);
        #[cfg(windows)]
        if let Some(system_root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", system_root);
        }
        command
    }

    #[test]
    fn missing_required_files_fail_with_case_context() {
        let temporary = tempfile::tempdir().unwrap();
        fs::write(temporary.path().join("message.txt"), b"feat: change\n").unwrap();
        let error = load_fixture("incomplete", temporary.path()).unwrap_err();
        assert!(error.contains("case incomplete"));
        assert!(error.contains("case.toml"));
    }

    #[rstest::rstest]
    #[case::missing_field("args = []\n", "exit_code")]
    #[case::unknown_field("args = []\nexit_code = 0\nextra = true\n", "extra")]
    #[case::wrong_type("args = 'not-an-array'\nexit_code = 0\n", "args")]
    fn invalid_manifest_schema_fails_with_case_context(
        #[case] manifest: &str,
        #[case] detail: &str,
    ) {
        let temporary = tempfile::tempdir().unwrap();
        write_minimum_fixture(temporary.path(), manifest);
        let error = load_fixture("bad-schema", temporary.path()).unwrap_err();
        assert!(error.contains("case bad-schema"));
        assert!(error.contains("case.toml"));
        assert!(error.contains(detail), "{error}");
    }

    #[test]
    fn nested_directories_are_rejected_with_case_context() {
        let temporary = tempfile::tempdir().unwrap();
        write_minimum_fixture(temporary.path(), "args = []\nexit_code = 0\n");
        fs::create_dir(temporary.path().join("nested")).unwrap();
        let error = load_fixture("nested", temporary.path()).unwrap_err();
        assert!(error.contains("case nested"));
        assert!(error.contains("regular files directly"));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_fixture_files_are_rejected_with_case_context() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().unwrap();
        write_minimum_fixture(temporary.path(), "args = []\nexit_code = 0\n");
        let target = temporary.path().join("message.txt");
        symlink(target, temporary.path().join("linked.txt")).unwrap();
        let error = load_fixture("symlink", temporary.path()).unwrap_err();
        assert!(error.contains("case symlink"));
        assert!(error.contains("regular files directly"));
    }

    #[test]
    fn optional_regular_files_are_loaded_for_copy() {
        let temporary = tempfile::tempdir().unwrap();
        write_minimum_fixture(temporary.path(), "args = []\nexit_code = 0\n");
        fs::write(temporary.path().join("config.toml"), "version = 1\n").unwrap();
        let loaded = load_fixture("optional-files", temporary.path()).unwrap();
        assert_eq!(loaded.files.len(), 5);
        assert!(loaded.files.iter().any(|(_, name)| name == "config.toml"));
    }
}
