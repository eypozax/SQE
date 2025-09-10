// === src/items/js.rs ===

#[derive(Debug, Clone)]
pub struct Js {
    pub script: String,
}

impl Js {
    pub fn parse(block: &str) -> Self {
        Js {
            script: block.trim().to_string(),
        }
    }

    /// Returns (html_fragment, optional_js). For Js we return no HTML and the JS as the optional_js
    /// so the converter can include it in the PAGE_SCRIPTS array or in a global setup area.
    pub fn render_html(&self) -> (String, Option<String>) {
        (String::new(), Some(self.script.clone()))
    }
}