mod convert;
mod items;

#[macro_use]
extern crate lazy_static;

#[path = "./transcompiler.rs"]
mod transcompiler;

use std::env;

fn main() -> std::io::Result<()> {
    // Grab the first argument (file path)
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run <input-file>");
        return Ok(());
    }

    let input_path = &args[1];

    // Print + write
    println!("{}", input_path);

    transcompiler::compile(input_path)?;

    Ok(())
}
