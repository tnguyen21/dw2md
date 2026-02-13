use std::process::Command;

fn dw2md() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dw2md"))
}

#[test]
fn test_help_flag() {
    let output = dw2md().arg("--help").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("DeepWiki"));
    assert!(stdout.contains("--output"));
    assert!(stdout.contains("--format"));
}

#[test]
fn test_version_flag() {
    let output = dw2md().arg("--version").output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("0.2.0"));
}

#[test]
fn test_invalid_repo_format() {
    let output = dw2md().arg("not-a-repo").output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Invalid repository identifier"));
}

#[test]
fn test_wrong_host_url() {
    let output = dw2md()
        .arg("https://github.com/owner/repo")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("deepwiki.com"));
}

#[test]
fn test_help_shows_new_flags() {
    let output = dw2md().arg("--help").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--list"));
    assert!(stdout.contains("--interactive"));
}
