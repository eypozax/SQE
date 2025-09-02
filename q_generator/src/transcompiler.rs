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
            if line.contains('{') && line.contains('}') {
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.rfind('}') {
                        let inner = line[start + 1..end].trim().to_string();
                        let insert_node = Insert::parse(&inner);
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
                }
            }

            if line.contains('{') {
                let block = parse_block(&mut lines_iter, "}")?;
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

            let block = if line.contains('{') && line.contains('}') {
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.rfind('}') {
                        line[start + 1..end].to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else if line.contains('{') {
                parse_block(&mut lines_iter, "}")?
            } else {
                String::new()
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
            let block = if line.contains('{') && line.contains('}') {
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.rfind('}') {
                        line[start + 1..end].to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            } else if line.contains('{') {
                parse_block(&mut lines_iter, "}")?
            } else {
                String::new()
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
