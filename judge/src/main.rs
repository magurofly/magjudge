pub mod config;
pub mod compile;
pub mod execute;

const SOURCE_CODE: &'static str = r#"
fn main() {
    println!("hello, world");
}
"#;

fn main() {
    println!("save source");
    compile::save_source("test", &SOURCE_CODE.bytes().collect::<Vec<_>>()).unwrap();
    println!("compile");
    compile::compile("test").unwrap();
    println!("run");
    execute::run("test").unwrap();
}
