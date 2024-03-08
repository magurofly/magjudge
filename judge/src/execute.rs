use nix::sys::wait;
use nix::unistd;
use nix::mount;
use nix::sched::{clone, CloneFlags};
use nix::sys::signal::Signal;
use std::env::current_dir;
use std::ffi::CString;
use std::io::Read;
use std::time;
use std::fs::*;
use std::os::fd::AsRawFd;
use std::os::fd::IntoRawFd;
use std::path::PathBuf;
use std::error::*;

use crate::config::*;

const CGROUP_TASKS_PATHS: [&'static str; 1] = [
    "/sys/fs/cgroup/memory/judge/tasks",
];

const JUDGE_UID: unistd::Uid = unistd::Uid::from_raw(JUDGE_UID_RAW);
const JUDGE_GID: unistd::Gid = unistd::Gid::from_raw(JUDGE_GID_RAW);

pub fn run(submission_id: &str) -> Result<i32, Box<dyn Error>> {
    // validate
    if !submission_id.chars().all(char::is_alphanumeric) {
        return Err("submission_id contains invalid characters".into());
    }

    // chroot 先を構築
    println!("mkdir chroot");
    let base_path = PathBuf::from(EXECUTE_DIR).join(submission_id);
    create_dir_all(&base_path)?;
    unistd::chown(&base_path, Some(JUDGE_UID), Some(JUDGE_GID))?;
    create_dir_all(base_path.join("home/judge"))?;
    create_dir_all(base_path.join("usr/bin"))?;
    mount::mount(Some("/usr/bin"), &base_path.join("usr/bin"), None::<&str>, mount::MsFlags::MS_BIND, None::<&str>)?;

    // 計測開始
    let time_start = time::Instant::now();

    // 子プロセスを走らせる
    println!("run subprocess");
    let mut stack = vec![0; 1024 * 1024];
    let clone_flags = CloneFlags::CLONE_NEWUTS | CloneFlags::CLONE_NEWPID | CloneFlags::CLONE_NEWNS | CloneFlags::CLONE_NEWIPC | CloneFlags::CLONE_NEWNET /* CLONE_NEWNET でネットワークから切断 */;
    let pid = unsafe { clone(Box::new(|| wrap_child_process(submission_id) ), &mut stack, clone_flags, Some(Signal::SIGCHLD as i32)) }?;
    println!("waitpid");
    let status = wait::waitpid(pid, None)?;

    // 計測終了
    let duration = time_start.elapsed();
    println!("running time = {:?}", duration); //FIXME: for test

    // debug
    let mut stdout = String::new();
    File::open(base_path.join("home/judge/stdout.txt"))?.read_to_string(&mut stdout)?;
    println!("stdout = {:?}", stdout);

    // chroot 先を消す
    println!("rmdir chroot");
    mount::umount(&base_path.join("usr/bin"))?;
    remove_dir_all(&base_path)?;

    match status {
        wait::WaitStatus::Exited(_, status) => Ok(status),
        _ => Err("runtime error".into())
    }
}

fn wrap_child_process(submission_id: &str) -> isize {
    run_child_process(submission_id).ok().unwrap_or(1)
}

fn run_child_process(submission_id: &str) -> Result<isize, Box<dyn Error>> {
    // 現在のプロセス（子プロセス）を cgroup に登録する
    println!("register to cgroup");
    let pid = unistd::getpid().as_raw().to_string();
    for path in &CGROUP_TASKS_PATHS {
        write(path, &pid)?;
    }

    // chroot
    let base_path = PathBuf::from(EXECUTE_DIR).join(submission_id);
    println!("chroot");
    unistd::chdir(&base_path.join("home/judge")).unwrap();
    println!("pwd1 = {:?}", current_dir());
    unistd::chroot(&base_path).unwrap();
    println!("pwd2 = {:?}", current_dir());

    // 入出力リダイレクト
    println!("redirect");
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
    println!("setuid");
    unistd::setuid(JUDGE_UID)?;
    unistd::setgid(JUDGE_GID)?;

    // exec
    println!("exec");
    let argv = vec!["-s", "9", "10", "main"].into_iter().map(|s| CString::new(s).unwrap() ).collect::<Vec<_>>();
    unistd::execv(&CString::new("timeout")?, &argv)?;

    Err("exec() failed".into())
}