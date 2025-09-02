use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use crate::items::{Choose, Function, Insert};

#[derive(Debug)]
pub enum Entry {
    Import {
        path: String,
    },
    Page {
        title: String,
        content: Vec<Question>,
    },
}

#[derive(Debug)]
pub enum Question {
    Choose(Choose),
    Insert(Insert),
    Function(Function),
}

// --- new helper: reads a brace-delimited block while ignoring braces inside strings ---
fn read_brace_block<I>(
    lines: &mut std::iter::Peekable<I>,
    first_after_open: &str,
) -> io::Result<String>
where
    I: Iterator<Item = io::Result<String>>,
{
    let mut out = String::new();

    // We start *after* the initial '{' (first_after_open is the substring after the first '{' on that line).
    // We'll treat the stream as though we've seen a starting brace, so depth starts at 1.
    let mut depth: i32 = 1;
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut escape = false;

    // Helper to process a single line's characters
    let mut process_chars = |s: &str| -> Option<()> {
        for ch in s.chars() {
            if escape {
                // previous char was backslash, consume this char literally
                out.push(ch);
                escape = false;
                continue;
            }

            match ch {
                '\\' => {
                    // begin escape sequence
                    out.push(ch);
                    escape = true;
                }
                '\'' => {
                    out.push(ch);
                    if !in_double && !in_backtick {
                        in_single = !in_single;
                    }
                }
                '"' => {
                    out.push(ch);
                    if !in_single && !in_backtick {
                        in_double = !in_double;
                    }
                }
                '`' => {
                    out.push(ch);
                    if !in_single && !in_double {
                        in_backtick = !in_backtick;
                    }
                }
                '{' => {
                    if !in_single && !in_double && !in_backtick {
                        depth += 1;
                        out.push(ch);
                    } else {
                        out.push(ch);
                    }
                }
                '}' => {
                    if !in_single && !in_double && !in_backtick {
                        depth -= 1;
                        if depth == 0 {
                            // we reached the matching closing brace — DONE (do NOT include this brace)
                            return None; // signal done for caller to stop
                        } else {
                            out.push(ch);
                        }
                    } else {
                        out.push(ch);
                    }
                }
                _ => {
                    out.push(ch);
                }
            }
        }
        // still more to read
        out.push('\n');
        Some(())
    };

    // process the remainder of the line after the opening brace first
    if !first_after_open.is_empty() {
        if process_chars(first_after_open).is_none() {
            return Ok(out);
        }
    }

    // then continue reading subsequent lines until depth returns to 0 or EOF
    while let Some(line_res) = lines.next() {
        let line = line_res?;
        if process_chars(&line).is_none() {
            return Ok(out);
        }
    }

    // If we exhausted input without closing, return an error
    Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "unterminated brace block",
    ))
}

// --- old parse_block kept for compatibility when simple closing marker is desired ---
// (You can keep it or remove it. It's still used nowhere after this change.)
pub fn parse_block<I>(
    lines: &mut std::iter::Peekable<I>,
    closing_marker: &str,
) -> io::Result<String>
where
    I: Iterator<Item = io::Result<String>>,
{
    let mut out = String::new();
    while let Some(line_res) = lines.next() {
        let line = line_res?;
        if line.contains(closing_marker) {
            if let Some(pos) = line.find(closing_marker) {
                let before = &line[..pos];
                if !before.trim().is_empty() {
                    out.push_str(before);
                    out.push('\n');
                }
            }
            break;
        } else {
            out.push_str(&line);
            out.push('\n');
        }
    }
    Ok(out)
}

// ... then the compile() function follows but with updated block handling ...
pub fn compile<P: AsRef<Path>>(path: P) -> io::Result<Vec<Entry>> {
    let file = File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut lines_iter = reader.lines().peekable();

    let mut ast: Vec<Entry> = Vec::new();
    let mut current_page: Option<(String, Vec<Question>)> = None;

    while let Some(line_res) = lines_iter.next() {
        let raw = line_res?;
        let line = raw.trim();

        if line.is_empty() || line.starts_with("//") {
            continue;
        }

        if line.starts_with("@p") {
            if let Some((title, content)) = current_page.take() {
                ast.push(Entry::Page { title, content });
            }
            let title = line.split('"').nth(1).unwrap_or("untitled").to_string();
            current_page = Some((title, Vec::new()));
            continue;
        }

        if line.starts_with("import") {
            let path = line.split('"').nth(1).unwrap_or("").to_string();
            ast.push(Entry::Import { path });
            continue;
        }

        if line.starts_with("insert") {
            // handle inline { ... } and block { ... } with brace-aware parser
            if let Some(open_pos) = raw.find('{') {
                // take substring after first '{'
                let after = &raw[open_pos + 1..];
                let block = if after.contains('}') {
                    // still use the brace-aware read to correctly ignore } inside strings
                    read_brace_block(&mut lines_iter, after)?
                } else {
                    read_brace_block(&mut lines_iter, after)?
                };
                let text = block.trim().to_string();
                let insert_node = Insert::parse(&text);
                if let Some((_title, content)) = current_page.as_mut() {
                    content.push(Question::Insert(insert_node));
                } else {
                    ast.push(Entry::Page {
                        title: "untitled".to_string(),
                        content: vec![Question::Insert(insert_node)],
                    });
                }
                continue;
            }

            // fallback: single-line insert without braces
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() > 1 {
                let text = words[1..].join(" ");
                let insert_node = Insert::parse(&text);
                if let Some((_title, content)) = current_page.as_mut() {
                    content.push(Question::Insert(insert_node));
                } else {
                    ast.push(Entry::Page {
                        title: "untitled".to_string(),
                        content: vec![Question::Insert(insert_node)],
                    });
                }
            }
            continue;
        }

        if line.starts_with("choice") {
            let mut id: Option<String> = None;
            if line.contains('{') {
                let before_brace = line.split('{').next().unwrap_or("");
                let parts: Vec<&str> = before_brace.split_whitespace().collect();
                if parts.len() >= 2 {
                    id = Some(parts[1].to_string());
                }
            } else {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    id = Some(parts[1].to_string());
                }
            }

            // get substring after first '{', if any
            let block = if let Some(open_pos) = raw.find('{') {
                let after = &raw[open_pos + 1..];
                read_brace_block(&mut lines_iter, after)?
            } else {
                // block starts on following lines
                read_brace_block(&mut lines_iter, "")?
            };

            let choose_node = Choose::parse(&block, id);

            if let Some((_title, content)) = current_page.as_mut() {
                content.push(Question::Choose(choose_node));
            } else {
                ast.push(Entry::Page {
                    title: "untitled".to_string(),
                    content: vec![Question::Choose(choose_node)],
                });
            }

            continue;
        }

        // NEW: function block `f { ... }`
        if line.starts_with("f") {
            let block = if let Some(open_pos) = raw.find('{') {
                let after = &raw[open_pos + 1..];
                read_brace_block(&mut lines_iter, after)?
            } else {
                read_brace_block(&mut lines_iter, "")?
            };

            let fn_node = Function::parse(&block);

            if let Some((_title, content)) = current_page.as_mut() {
                content.push(Question::Function(fn_node));
            } else {
                ast.push(Entry::Page {
                    title: "untitled".to_string(),
                    content: vec![Question::Function(fn_node)],
                });
            }

            continue;
        }

        // unknown lines ignored
    }

    if let Some((title, content)) = current_page.take() {
        ast.push(Entry::Page { title, content });
    }

    Ok(ast)
}
