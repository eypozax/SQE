// === src/items/html.rs ===

#[derive(Debug, Clone)]
pub struct Html {
    pub html: String,
}

impl Html {
    pub fn parse(block: &str) -> Self {
        Html {
            // We preserve the user's HTML as-is; trimming to remove leading/trailing whitespace.
            html: block.trim().to_string(),
        }
    }

    /// Returns (html_fragment, optional_js). For Html we return the HTML fragment and no JS.
    pub fn render_html(&self) -> (String, Option<String>) {
        (self.html.clone(), None)
    }
}