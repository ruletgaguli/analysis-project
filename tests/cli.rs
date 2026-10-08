use std::process::Command;

fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cli"));
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command
}

#[test]
fn example_output_matches_original() {
    let output = cli().arg("example.log").output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let expected =
        include_str!("fixtures/example.stdout").replace("{cwd}", env!("CARGO_MANIFEST_DIR"));
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn missing_argument_is_an_error_not_a_panic() {
    let output = cli().output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("укажите файл логов"));
    assert!(!stderr.contains("panicked"));
}

#[test]
fn missing_file_is_an_error_not_a_panic() {
    let output = cli()
        .arg("tests/fixtures/does-not-exist.log")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Ошибка:"));
    assert!(!stderr.contains("panicked"));
}

#[test]
fn malformed_file_is_an_error_not_a_partial_success() {
    let output = cli().arg("Cargo.toml").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("ошибка разбора строки 1"));
    assert!(!stderr.contains("panicked"));
}
