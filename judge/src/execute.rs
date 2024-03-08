#[derive(Clone, Debug)]
pub struct ExecutionResult {
    status: i32,
    run_time: time::Duration,
    stdout: String,
    stderr: String,
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
        create_dir_all(&base_path)?;
        unistd::chown(&base_path, Some(JUDGE_UID), Some(JUDGE_GID))?;
        create_dir_all(base_path.join("home/judge"))?;
        create_dir_all(base_path.join("usr/bin"))?;
        mount::mount(Some("/usr/bin"), &base_path.join("usr/bin"), None::<&str>, mount::MsFlags::MS_BIND, None::<&str>)?;
        Ok(Self { submission_id: submission_id.to_string() })
    }

    pub fn execute(&self, input: &str) -> Result<ExecutionResult, ExecutionFailingReason> {
        // 入力を配置
        let home_path = PathBuf::from(EXECUTE_DIR).join(&self.submission_id).join("home/judge");
        {
            let Ok(mut stdin) = File::create(&home_path.join("stdin.txt")) else { return Err(SystemError) };
            let Ok(_) = stdin.write(input.as_bytes()) else { return Err(SystemError) };
        }

        // 計測開始
        let time_start = time::Instant::now();
    
        // 子プロセスを走らせる
        let Ok(pid) = ({
            let mut stack = vec![0; 1024 * 1024];
            let clone_flags = CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_NEWNET /* CLONE_NEWNET でネットワークから切断 */;
            unsafe { clone(Box::new(|| wrap_child_process(&self.submission_id) ), &mut stack, clone_flags, Some(Signal::SIGCHLD as i32)) }
        }) else { return Err(SystemError) };
        
        // 終了を待つ
        let Ok(wait::WaitStatus::Exited(_, status)) = wait::waitpid(pid, None) else { return Err(SystemError) };
    
        // 計測終了
        let run_time = time_start.elapsed();
    
        // 出力を読む
        let mut stdout = String::new();
        let mut stderr = String::new();
        File::open(&home_path.join("stdout.txt")).and_then(|mut file| file.read_to_string(&mut stdout) ).map_err(|_| SystemError )?;
        File::open(&home_path.join("stderr.txt")).and_then(|mut file| file.read_to_string(&mut stderr) ).map_err(|_| SystemError )?;
    
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
        mount::umount(&base_path.join("usr/bin")).unwrap();
        remove_dir_all(&base_path).unwrap();
    }
}

use nix::sys::wait;
use nix::unistd;
use nix::mount;
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use std::ffi::CString;
use std::io::{Read, Write};
use std::time;
use std::fs::*;
use std::os::fd::AsRawFd;
use std::os::fd::IntoRawFd;
use std::path::PathBuf;
use std::error::*;

use ExecutionFailingReason::*;
use crate::config::*;

const CGROUP_TASKS_PATHS: [&'static str; 1] = [
    "/sys/fs/cgroup/memory/judge/tasks",
];

const JUDGE_UID: unistd::Uid = unistd::Uid::from_raw(JUDGE_UID_RAW);
const JUDGE_GID: unistd::Gid = unistd::Gid::from_raw(JUDGE_GID_RAW);

fn wrap_child_process(submission_id: &str) -> isize {
    run_child_process(submission_id).ok().unwrap_or(1)
}

fn run_child_process(submission_id: &str) -> Result<isize, Box<dyn Error>> {
    // 現在のプロセス（子プロセス）を cgroup に登録する
    let pid = unistd::getpid().as_raw().to_string();
    for path in &CGROUP_TASKS_PATHS {
        write(path, &pid)?;
    }

    // chroot
    let base_path = PathBuf::from(EXECUTE_DIR).join(submission_id);
    unistd::chdir(&base_path.join("home/judge")).unwrap();
    unistd::chroot(&base_path).unwrap();

    // 入出力リダイレクト
    let redirects = [
        (std::io::stdin().as_raw_fd(), File::open("/home/judge/stdin.txt")?.into_raw_fd()),
        (std::io::stdout().as_raw_fd(), File::create("/home/judge/stdout.txt")?.into_raw_fd()),
        (std::io::stderr().as_raw_fd(), File::create("/home/judge/stderr.txt")?.into_raw_fd()),
    ];
    for (fd, tmpfd) in redirects {
        unistd::close(fd)?;
        unistd::dup2(tmpfd, fd)?;
        unistd::close(tmpfd)?;
    }

    // setuid
    unistd::setuid(JUDGE_UID)?;
    unistd::setgid(JUDGE_GID)?;

    // exec
    let argv = vec!["-s", "9", "10", "main"].into_iter().map(|s| CString::new(s).unwrap() ).collect::<Vec<_>>();
    unistd::execv(&CString::new("timeout")?, &argv)?;

    Err("exec() failed".into())
}