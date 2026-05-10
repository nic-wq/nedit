use std::process::{Command, Output};
use tempfile::TempDir;

/// Returns a temporary directory for tests dealing with files
pub fn tmp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temporary directory")
}

/// Fixture: path to the project binary
pub fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_nedit")
}

/// Executes the binary and returns the Output
pub fn run_bin(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("Failed to execute the binary")
}

/// Executes the binary and returns (stdout, stderr, exit_code)
pub fn exec_bin(args: &[&str]) -> (String, String, i32) {
    let output = run_bin(args);
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}
