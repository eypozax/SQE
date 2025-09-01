use crate::items::*;
use crate::transcompiler_old::read_block::parse_block;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

#[path = "./convert.rs"]
mod convert;

#[path = "./read_block.rs"]
mod read_block;

// --- AST-based transcompiler ---
#[derive(Debug)]
pub enum Node {
    Insert {
        text: Vec<Text>,
    },
    Choice {
        text: Vec<Text>,
        options: Vec<Button>,
        addons: Vec<Addon>,
    },
}

#[derive(Debug)]
pub struct Text {
    pub content: String,
}

#[derive(Debug)]
pub struct Button {
    pub label: String,
    pub value: i32,
}

#[derive(Debug)]
pub struct Addon {
    pub content: String,
    pub code: String,
}

/// Reads a block from the iterator until a line containing `}`
/// Returns the concatenated content inside the block

/// Compiles a `.sqe`-style file into HTML using convert.rs
pub fn compile(direction: &str) -> io::Result<()> {
    let path = Path::new(direction);
    let file = File::open(&path)?;
    let reader = io::BufReader::new(file);
    let mut pointer: i32 = 1;

    let mut ast: Vec<Node> = Vec::new();

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
                        let block_content = parse_block(&mut lines_iter, "}")?;
                    } else if words.len() > 1 {
                        // Single-line insert: everything after "insert"
                        let single_line_content = words[1..].join(" ");
                    } else {
                        // Nothing to insert
                        println!(
                            "Warning: 'insert' command without content on line {}",
                            pointer
                        );
                    }
                }
                "choice" => {
                    if line.contains("{") {
                        let block_content = parse_block(&mut lines_iter, "}")?;
                        println!("{:#}", block_content);
                    } else {
                        println!(
                            "Warning: 'choice' command without block on line {}",
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
