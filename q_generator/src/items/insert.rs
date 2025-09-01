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
        let html = format!(
            "<div class=\"text-block\">{}</div>",
            escape_html(&self.text)
        );
        (html, None)
    }
}
