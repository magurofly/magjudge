use crate::config::*;
use crate::program::*;

use serde::*;
use std::os::unix::process::ExitStatusExt;
use std::process::*;
use std::fs::*;
use std::io::Write;
use std::error::*;

#[derive(Clone, Debug, Serialize)]
pub struct CompilingResult {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn save_source(submission_id: &str, source_code: &[u8]) -> Result<(), Box<dyn Error>> {
    let mut source_file = File::create(source_path(submission_id))?;
    source_file.write_all(source_code)?;
    source_file.flush()?;
    Ok(())
}

pub fn compile(submission_id: &str) -> Result<CompilingResult, Box<dyn Error>> {
    let output = Command::new("rustc")
        .args(&compile_args(submission_id))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let status = output.status.into_raw();
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;

    Ok(CompilingResult { status, stdout, stderr })
}

fn compile_args(submission_id: &str) -> Vec<String> {
    let source_path = source_path(submission_id);
    let opt_dependency = format!("dependency={DEPENDENCY_DIR}");
    let codegen_opts = ["opt-level=3", "embed-bitcode=no"];
    let libs: Vec<&str> = vec![];
    
    let mut args = vec![];
    args.push("--crate-name=main".to_string());
    args.push("--edition=2018".to_string());
    args.push("--error-format=json".to_string());
    args.push("--json=diagnostic-short".to_string());
    args.push("--crate-type=bin".to_string());
    args.push("--emit=link".to_string());
    for codegen_opt in &codegen_opts {
        args.push("-C".to_string());
        args.push(codegen_opt.to_string());
    }
    for lib in &libs {
        args.push(format!("--extern={lib}"));
    }
    args.push(format!("--out-dir={}", execute_dir(submission_id).to_str().unwrap()));
    args.push("-L".to_string());
    args.push(opt_dependency);
    args.push(source_path.to_str().unwrap().to_string());
    args
}