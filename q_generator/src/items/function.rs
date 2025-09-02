// === src/items/function.rs ===

#[derive(Debug, Clone)]
pub struct Function {
    pub script: String,
}

impl Function {
    pub fn parse(block: &str) -> Self {
        Function {
            script: block.trim().to_string(),
        }
    }

    /// Returns (html_fragment, optional_js). For Function we return empty html and the JS.
    pub fn render_html(&self) -> (String, Option<String>) {
        (String::new(), Some(self.script.clone()))
    }
}
