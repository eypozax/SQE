use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

/// AST (public so other modules can use it)
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
    Choice {
        id: Option<String>,
        question: String,
        options: Vec<String>,
        script: Option<Vec<String>>,
    },
    Text {
        text: String,
    },
}

/// Read a multiline block from the iterator until a line containing `closing_marker`.
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
            // include text before marker (if any) then stop
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

/// Parse the SQE-like file into an AST (Vec<Entry>)
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

        // page marker: @p "title"
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
            // inline `insert { text }`
            if line.contains('{') && line.contains('}') {
                if let Some(start) = line.find('{') {
                    if let Some(end) = line.rfind('}') {
                        let inner = line[start + 1..end].trim().to_string();
                        if let Some((_title, content)) = current_page.as_mut() {
                            content.push(Question::Text { text: inner });
                        } else {
                            ast.push(Entry::Page {
                                title: "untitled".to_string(),
                                content: vec![Question::Text { text: inner }],
                            });
                        }
                        continue;
                    }
                }
            }

            // multi-line block
            if line.contains('{') {
                let block = parse_block(&mut lines_iter, "}")?;
                let text = block.trim().to_string();
                if let Some((_title, content)) = current_page.as_mut() {
                    content.push(Question::Text { text });
                } else {
                    ast.push(Entry::Page {
                        title: "untitled".to_string(),
                        content: vec![Question::Text { text }],
                    });
                }
                continue;
            }

            // single-line: insert Hello
            let words: Vec<&str> = line.split_whitespace().collect();
            if words.len() > 1 {
                let text = words[1..].join(" ");
                if let Some((_title, content)) = current_page.as_mut() {
                    content.push(Question::Text { text });
                } else {
                    ast.push(Entry::Page {
                        title: "untitled".to_string(),
                        content: vec![Question::Text { text }],
                    });
                }
            }
            continue;
        }

        // choice [id]? { ... }
        if line.starts_with("choice") {
            // extract id (if present)
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

            // get block
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

            // split block into trimmed non-empty lines
            let mut block_lines: Vec<String> = block
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();

            let mut question_text = "⚠ no question found".to_string();
            let mut options: Vec<String> = Vec::new();
            let mut script: Option<Vec<String>> = None;

            if !block_lines.is_empty() {
                question_text = block_lines.remove(0);
                let mut i = 0;
                while i < block_lines.len() {
                    let ln = &block_lines[i];
                    if ln.starts_with(".addons") {
                        // iterate inside addons
                        i += 1;
                        while i < block_lines.len() {
                            let inner = &block_lines[i];
                            if inner == "]" {
                                break;
                            }
                            if inner.starts_with(".script") {
                                // inline .script[ ... ] or multiline
                                if inner.contains('[') && inner.contains(']') {
                                    if let Some(start) = inner.find('[') {
                                        if let Some(end) = inner.rfind(']') {
                                            let s = inner[start + 1..end].trim().to_string();
                                            script = Some(vec![s]);
                                        }
                                    }
                                } else if inner.contains('[') {
                                    let mut collected: Vec<String> = Vec::new();
                                    if let Some(pos) = inner.find('[') {
                                        let after = inner[pos + 1..].trim();
                                        if !after.is_empty() {
                                            collected.push(after.to_string());
                                        }
                                    }
                                    i += 1;
                                    while i < block_lines.len() {
                                        let scr_line = &block_lines[i];
                                        if scr_line.contains(']') {
                                            if let Some(pos) = scr_line.find(']') {
                                                let before = scr_line[..pos].trim();
                                                if !before.is_empty() {
                                                    collected.push(before.to_string());
                                                }
                                            }
                                            break;
                                        } else {
                                            collected.push(scr_line.clone());
                                        }
                                        i += 1;
                                    }
                                    if !collected.is_empty() {
                                        script = Some(collected);
                                    }
                                }
                            }
                            i += 1;
                        }
                        i += 1;
                        continue;
                    }

                    if ln.contains(">>") {
                        let left = ln.split(">>").next().unwrap_or("").trim().to_string();
                        if !left.is_empty() {
                            options.push(left);
                        }
                    } else {
                        options.push(ln.clone());
                    }

                    i += 1;
                }
            }

            // push to current page or make untitled page
            if let Some((_title, content)) = current_page.as_mut() {
                content.push(Question::Choice {
                    id,
                    question: question_text,
                    options,
                    script,
                });
            } else {
                ast.push(Entry::Page {
                    title: "untitled".to_string(),
                    content: vec![Question::Choice {
                        id,
                        question: question_text,
                        options,
                        script,
                    }],
                });
            }

            continue;
        }

        // ignore anything else for now
    }

    // flush final page
    if let Some((title, content)) = current_page.take() {
        ast.push(Entry::Page { title, content });
    }

    Ok(ast)
}
