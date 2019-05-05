use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let config_path = Path::new("config");
    let builder = config_path.join("build.rs");
    let demo = config_path.join("demo.png");
    let out_path = Path::new(&out_dir).join("demo_output.rs");

    // rebuild if any of these changed
    println!("cargo:rerun-if-changed={}", demo.display());
    println!("cargo:rerun-if-changed={}", builder.display());

    let mut bytes = vec![];
    let mut fd = File::open(&demo).unwrap();
    fd.read_to_end(&mut bytes).unwrap();

    let contents = format!(
        "
    const DEMO_DATA: [u8; {}] = {:?};
",
        bytes.len(),
        bytes
    );

    let mut fd = File::create(&out_path).unwrap();
    fd.write_all(contents.as_bytes()).unwrap();
}
