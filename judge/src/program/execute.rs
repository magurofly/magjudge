#[derive(Clone, Debug, Serialize)]
pub struct ExecutionResult {
    pub status: i32,
    pub time_ms: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn execute(submission_id: &str, input: &str) -> Result<ExecutionResult, Box<dyn Error>> {
    // 計測開始
    let time_start = time::Instant::now();

    // 子プロセスを起動
    let base_path = PathBuf::from(EXECUTE_DIR).join(submission_id);
    let mut process = Command::new("timeout")
        .args(["-s9", &TIME_LIMIT.to_string(), "./safe_run", base_path.to_str().unwrap(), "main"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // cgroup に追加
    let pid = process.id();
    for cgroup_task_path in &CGROUP_TASKS_PATHS {
        let mut tasks = OpenOptions::new().append(true).open(cgroup_task_path)?;
        tasks.write(format!("{}\n", pid).as_bytes())?;
    }

    // 入力を書き込み
    if let Some(mut stdin) = process.stdin.take() {
        stdin.write(input.as_bytes())?;
        // drop により自動で stdin が閉じる
    }

    // 終了を待つ
    let Output { status, stdout, stderr } = process.wait_with_output()?;

    // 計測終了
    let run_time = time_start.elapsed();

    let status = status.into_raw();
    let stdout = String::from_utf8(stdout)?;
    let stderr = String::from_utf8(stderr)?;

    Ok(ExecutionResult {
        status,
        time_ms: run_time.as_millis() as i32,
        stdout,
        stderr,
    })
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

use serde::Serialize;

use crate::config::*;