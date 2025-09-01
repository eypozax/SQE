mod convert;
mod items;

#[macro_use]
extern crate lazy_static;

#[path = "./transcompiler.rs"]
mod transcompiler;

use std::env;
use std::io;

fn main() -> io::Result<()> {
    // Grab the first argument (file path)
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run <input-file>");
        return Ok(());
    }

    let input_path = &args[1];

    // Parse the input file into an AST
    let ast = transcompiler::compile(input_path)?;

    // Print + write
    println!("Parsed AST:\n{:#?}", ast);

    convert::build_pages(&ast, "out")?;
    println!("Wrote HTML files to ./out (open out/index.html)");

    Ok(())
}
