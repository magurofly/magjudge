pub mod config;
pub mod compile;
pub mod execute;

const SOURCE_CODE: &'static str = r#"
use std::io;
fn main() {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();
    println!("Input: {}", buf);
}
"#;

fn main() {
    println!("save source");
    compile::save_source("test", &SOURCE_CODE.bytes().collect::<Vec<_>>()).unwrap();
    println!("compile");
    compile::compile("test").unwrap();
    println!("run");
    eprintln!("{:?}", execute::Execution::new("test").unwrap().execute("hello").unwrap());
}
