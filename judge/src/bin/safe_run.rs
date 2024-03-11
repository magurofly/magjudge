/*
コマンドを安全に実行するためのプログラム。

このプログラムは第二引数以降で与えられたプログラムを実行する。
このとき:
- プログラムは呼び出したユーザーの実効 UID を持って実行される。
- プログラムは第一引数で与えられたディレクトリからの相対パスとして指定する必要がある。
- プログラムは、第一引数で与えられたディレクトリをルートディレクトリとして認識するため、ディレクトリ外へのアクセスが不可能となる。
- プログラムはネットワークにアクセスできない状態となる。
- 実行時、一時的に /lib, /lib64 を lib, lib64 にマウントするため、マウントポイントとして lib, lib64 という空のディレクトリが存在する必要がある。
*/

fn main() -> Result<(), Box<dyn Error>> {
    // 引数の取得
    let (root_dir, command, argv, envp) = {
        let mut args = args();
        args.next(); // ignore program name
        let root_dir = args.next().unwrap_or_else(|| usage() );
        let command = CString::new(args.next().unwrap_or_else(|| usage() ))?;
        let mut argv = args.map(|s| CString::new(s) ).collect::<Result<Vec<_>, _>>()?;
        argv.insert(0, command.clone());
        let envp = vec![CString::new("PATH=")?];
        (root_dir, command, argv, envp)
    };

    // 呼び出したユーザーの取得
    let parent_uid = {
        let parent_pid = Pid::parent();
        let mut parent_status = String::new();
        File::open(&format!("/proc/{parent_pid}/status"))?.read_to_string(&mut parent_status)?;
        let uid_row = parent_status.lines().find(|line| line.split_ascii_whitespace().next() == Some("Uid:") ).expect("couldn't get uid row from /proc/<caller_pid>/status");
        uid_row.split_ascii_whitespace().skip(2).next().expect("effective uid").parse::<u32>()?
    };

    // unshare
    {
        let unshare_flags = CloneFlags::CLONE_NEWUTS
                          | CloneFlags::CLONE_NEWPID
                          | CloneFlags::CLONE_NEWNS
                          | CloneFlags::CLONE_NEWIPC
                          | CloneFlags::CLONE_NEWNET
                          ;
        unshare(unshare_flags)?;
    };

    // 第一引数のディレクトリへ移動する
    chdir(root_dir.as_str())?;

    // マウントプロパゲーションの無効化
    mount(None::<&str>, "/", None::<&str>, MsFlags::MS_REC | MsFlags::MS_PRIVATE, None::<&str>)?;

    // lib, lib64 のマウント
    mount(Some("/lib"), "lib", None::<&str>, MsFlags::MS_BIND | MsFlags::MS_PRIVATE | MsFlags::MS_NOSUID | MsFlags::MS_RDONLY, None::<&str>)?;
    mount(Some("/lib64"), "lib64", None::<&str>, MsFlags::MS_BIND | MsFlags::MS_PRIVATE | MsFlags::MS_NOSUID | MsFlags::MS_RDONLY, None::<&str>)?;

    // chroot
    chroot(".")?;

    // setuid
    setuid(Uid::from_raw(parent_uid))?;

    // exec
    execve(&command, &argv, &envp)?;

    Err("maybe execv failed".into())
}

fn usage() -> ! {
    println!("Usage: {} ROOT_DIR COMMAND ...", current_exe().unwrap().to_string_lossy());
    println!("Note: this program must be suid of root and called by non-root user");
    println!("Note: COMMAND must be specified as a relative path from ROOT_DIR");
    println!("Note: there must exist dir ROOT_DIR/lib, ROOT_DIR/lib64 as mount points");
    panic!()
}

use std::env::*;
use std::error::*;
use std::ffi::*;
use std::fs::*;
use std::io::Read;
use nix::mount::*;
use nix::sched::*;
use nix::unistd::*;