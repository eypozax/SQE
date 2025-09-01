use std::io;

pub fn parse_block<I>(lines: &mut I, closing_tag: &str) -> io::Result<String>
where
    I: Iterator<Item = io::Result<String>>,
{
    let mut content = String::new();

    while let Some(line_res) = lines.next() {
        let line = line_res?;
        if line.contains(closing_tag) {
            break; // Stop at closing brace
        }
        content.push_str(&line);
        content.push('\n');
    }

    Ok(content)
}
