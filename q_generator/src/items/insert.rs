// === src/items/insert.rs ===

use crate::items::common::escape_html;

#[derive(Debug, Clone)]
pub struct Insert {
    pub text: String,
}

impl Insert {
    pub fn parse(block: &str) -> Self {
        Insert {
            text: block.trim().to_string(),
        }
    }

    pub fn render_html(&self) -> (String, Option<String>) {
        // Preserve line breaks: escape each line and join with <br/> so multi-line inserts render
        // as separate lines in the resulting HTML.
        let lines: Vec<String> = self
            .text
            .lines()
            .map(|l| escape_html(l))
            .collect();
        let joined = lines.join("<br/>\n");
        let html = format!("<div class=\"text-block\">{}</div>", joined);
        (html, None)
    }
}
