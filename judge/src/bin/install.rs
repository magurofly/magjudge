use std::{error::Error, process::Command};

fn main() -> Result<(), Box<dyn Error>> {
    let install_dir = prompt("Install dir: ")?;

    run_command(&format!("mkdir -p {install_dir}"))?;
    run_command(&format!("cp -T default.toml {install_dir}/config.toml"))?;
    run_command(&format!("cp -TR public {install_dir}/public"))?;

    run_command("cargo build --release --bin judge")?;
    run_command(&format!("cp -T target/release/judge {install_dir}/judge"))?;
    run_command(&format!("sudo setcap CAP_NET_BIND_SERVICE+ep {install_dir}/judge"))?;

    run_command("cargo build --release --bin safe_run")?;
    run_command(&format!("cp -T target/release/safe_run {install_dir}/safe_run"))?;
    run_command(&format!("sudo chown root:root {install_dir}/safe_run"))?;
    run_command(&format!("sudo chmod u+s {install_dir}/safe_run"))?;

    Ok(())
}

fn prompt(ask: &str) -> Result<String, Box<dyn Error>> {
    use std::io::*;
    let mut stdout = stdout();
    stdout.write(ask.as_bytes())?;
    stdout.flush()?;
    let mut input = String::new();
    stdin().read_to_string(&mut input)?;
    Ok(input)
}

fn run_command(command: &str) -> Result<(), Box<dyn Error>> {
    println!("$ {}", command);
    let mut command = command.split_ascii_whitespace();
    let program = command.next().unwrap().to_string();
    let args = command.map(str::to_string).collect::<Vec<_>>();
    Command::new(program).args(&args).spawn()?.wait()?;
    Ok(())
}