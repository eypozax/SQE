use crate::items::*;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[path = "./convert.rs"]
mod convert;

/// Reads a block from the iterator until a line containing `}`
/// Returns the concatenated content inside the block
fn read_block<I>(lines: &mut I) -> io::Result<String>
where
    I: Iterator<Item = io::Result<String>>,
{
    let mut content = String::new();

    while let Some(line_res) = lines.next() {
        let line = line_res?;
        if line.contains("}") {
            break; // Stop at closing brace
        }
        content.push_str(&line);
        content.push('\n');
    }

    Ok(content)
}

/// Compiles a `.sqe`-style file into HTML using convert.rs
pub fn compile(direction: &str) -> io::Result<()> {
    let path = Path::new(direction);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);
    let mut pointer: i32 = 1;

    // Make the iterator peekable so we can consume multi-line blocks
    let mut lines_iter = reader.lines().peekable();

    while let Some(line_res) = lines_iter.next() {
        let line = line_res?;

        // Split line into words
        let words: Vec<&str> = line.split_whitespace().collect();

        if let Some(first_word) = words.get(0) {
            match *first_word {
                // Insert block or single-line
                "insert" => {
                    if line.contains("{") {
                        let block_content = read_block(&mut lines_iter)?;
                        let html = return_text(&block_content);
                        convert::insert(&html);
                    } else if words.len() > 1 {
                        // Single-line insert: everything after "insert"
                        let single_line_content = words[1..].join(" ");
                        let html = return_text(&single_line_content);
                        convert::insert(&html);
                    } else {
                        // Nothing to insert
                        println!(
                            "Warning: 'insert' command without content on line {}",
                            pointer
                        );
                    }
                }

                // Catch-all for other commands
                _ => println!("{}", first_word),
            }
        }

        pointer += 1;
    }

    // Write final HTML
    convert::build_page();

    Ok(())
}
