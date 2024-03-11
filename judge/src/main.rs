pub mod config;
pub mod program;
pub mod server;

const SOURCE_CODE: &'static str = r#"
use std::io;
fn main() {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    println!("Input: {}", buf);
}
"#;

fn main() {
    eprintln!("{:?}", program::Program::new("test", &SOURCE_CODE).unwrap().run("hello").unwrap());
}
