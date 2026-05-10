use std::process::{Command, Output};
use tempfile::TempDir;

/// Retorna um diretório temporário para testes que lidam com arquivos
pub fn tmp_dir() -> TempDir {
    tempfile::tempdir().expect("Falha ao criar diretório temporário")
}

/// Fixture: caminho para o binário do projeto
pub fn bin_path() -> &'static str {
    env!("CARGO_BIN_EXE_nedit")
}

/// Executa o binário e retorna o Output
pub fn run_bin(args: &[&str]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .expect("Falha ao executar o binário")
}

/// Executa o binário e retorna (stdout, stderr, código_de_saída)
pub fn exec_bin(args: &[&str]) -> (String, String, i32) {
    let output = run_bin(args);
    (
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
        output.status.code().unwrap_or(-1),
    )
}
