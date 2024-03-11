pub mod compile;
pub mod execute;

use crate::config::*;

use std::path::*;
use std::fs::*;
use std::error::*;

use self::compile::*;
use self::execute::*;

pub struct Program {
    submission_id: String,
    compile_result: CompilingResult,
}

impl Program {
    pub fn new(submission_id: &str, source_code: &str) -> Result<Self, Box<dyn Error>> {
        // 実行用ディレクトリを作成
        let exec_dir = execute_dir(submission_id);
        create_dir_all(&exec_dir)?;
        create_dir_all(&exec_dir.join("lib"))?;
        create_dir_all(&exec_dir.join("lib64"))?;

        // ソースコードを保存
        save_source(submission_id, source_code.as_bytes())?;

        // コンパイル
        let compile_result = compile(submission_id)?;

        // ソースコードを削除
        remove_file(&source_path(submission_id))?;

        Ok(Self { compile_result, submission_id: submission_id.to_string() })
    }

    pub fn run(&self, input: &str) -> Result<execute::ExecutionResult, Box<dyn Error>> {
        execute(&self.submission_id, input)
    }

    pub fn compile_result(&self) -> &CompilingResult {
        &self.compile_result
    }
}
impl Drop for Program {
    fn drop(&mut self) {
        let submission_id = &self.submission_id;
        let exec_dir = execute_dir(submission_id);

        // 実行用ディレクトリを削除
        let _ = remove_dir(&exec_dir.join("lib64"));
        let _ = remove_dir(&exec_dir.join("lib"));
        let _ = remove_dir_all(&exec_dir);
    }
}

fn source_path(submission_id: &str) -> PathBuf {
    let source_filename = format!("{submission_id}.rs");
    Path::new(SOURCE_DIR).join(source_filename)
}

fn execute_dir(submission_id: &str) -> PathBuf {
    PathBuf::from(EXECUTE_DIR).join(submission_id)
}