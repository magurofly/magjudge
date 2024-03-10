#[derive(Clone, Debug)]
pub struct ExecutionResult {
    pub status: i32,
    pub run_time: time::Duration,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Copy, Debug)]
pub enum ExecutionFailingReason {
    TimeLimitExceeded,
    // OutputLimitExceeded, //TODO
    SystemError,
    BadRequest,
}

pub struct Execution {
    submission_id: String,
}
impl Execution {
    pub fn new(submission_id: &str) -> Result<Self, Box<dyn Error>> {
        // validate
        if !submission_id.chars().all(char::is_alphanumeric) {
            return Err("invalid submission id".into());
        }

        // prepare chroot directory
        let base_path = PathBuf::from(EXECUTE_DIR).join(submission_id);
        create_dir_all(base_path.join("lib"))?;
        create_dir_all(base_path.join("lib64"))?;
        Ok(Self { submission_id: submission_id.to_string() })
    }

    pub fn execute(&self, input: &str) -> Result<ExecutionResult, ExecutionFailingReason> {
        // 計測開始
        let time_start = time::Instant::now();

        // 子プロセスを起動
        let base_path = PathBuf::from(EXECUTE_DIR).join(&self.submission_id);
        let Ok(mut process) = Command::new("timeout")
            .args(["-s9", &TIME_LIMIT.to_string(), "./safe_run", base_path.to_str().unwrap(), "main"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() else { return Err(SystemError) };

        // cgroup に追加
        let pid = process.id();
        for cgroup_task_path in &CGROUP_TASKS_PATHS {
            let Ok(mut tasks) = OpenOptions::new().append(true).open(cgroup_task_path) else { return Err(SystemError) };
            let Ok(_) = tasks.write(format!("{}\n", pid).as_bytes()) else { return Err(SystemError) };
        }

        // 入力を書き込み
        if let Some(mut stdin) = process.stdin.take() {
            let _ = stdin.write(input.as_bytes());
            // drop により自動で stdin が閉じる
        }

        // 終了を待つ
        let Ok(Output { status, stdout, stderr }) = process.wait_with_output() else { return Err(SystemError) };
    
        // 計測終了
        let run_time = time_start.elapsed();

        let status = status.into_raw();
        let Ok(stdout) = String::from_utf8(stdout) else { return Err(SystemError) };
        let Ok(stderr) = String::from_utf8(stderr) else { return Err(SystemError) };
    
        Ok(ExecutionResult {
            status,
            run_time,
            stdout,
            stderr,
        })
    }
}
impl Drop for Execution {
    fn drop(&mut self) {
        let base_path = PathBuf::from(EXECUTE_DIR).join(&self.submission_id);
        let _ = remove_dir(&base_path.join("lib"));
        let _ = remove_dir(&base_path.join("lib64"));
    }
}

const CGROUP_TASKS_PATHS: [&'static str; 1] = [
    "/sys/fs/cgroup/memory/judge/tasks",
];

use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::*;
use std::time;
use std::fs::*;
use std::path::PathBuf;
use std::error::*;

use ExecutionFailingReason::*;
use crate::config::*;