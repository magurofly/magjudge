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
    let mut source_file = File::create(CONFIG.program.source_path.replace("{submission_id}", submission_id))?;
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
    let source_path = CONFIG.program.source_path.replace("{submission_id}", submission_id);
    let opt_dependency = format!("dependency={}", CONFIG.program.dependency_dir.replace("{submission_id}", submission_id));
    let codegen_opts = ["opt-level=3", "embed-bitcode=no"];
    
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
    for (name, path) in &CONFIG.program.externs {
        args.push("--extern".to_string());
        args.push(format!("{name}={path}"));
    }
    args.push(format!("--out-dir={}", CONFIG.program.execute_dir.replace("{submission_id}", submission_id)));
    args.push("-L".to_string());
    args.push(opt_dependency);
    args.push(source_path);
    args
}