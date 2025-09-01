// === src/main.rs ===

mod convert;
mod items;
mod transcompiler;

use std::env;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run -- <input-file>");
        return Ok(());
    }
    let input_path = &args[1];

    let ast = transcompiler::compile(input_path)?;
    println!(
        "Parsed AST:
{:#?}",
        ast
    );

    convert::build_pages(&ast, "out")?;
    println!("Wrote HTML files to ./out (open out/index.html)");

    Ok(())
}
