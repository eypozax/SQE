// === src/items/css.rs ===

#[derive(Debug, Clone)]
pub struct Css {
    pub css: String,
}

impl Css {
    pub fn parse(block: &str) -> Self {
        Css {
            css: block.trim().to_string(),
        }
    }

    /// Returns (html_fragment, optional_js). For Css we return the CSS as an HTML fragment wrapped in <style>
    /// so the converter can inject it into the document head or inline in the page.
    pub fn render_html(&self) -> (String, Option<String>) {
        let wrapped = format!("<style>\n{}\n</style>", self.css);
        (wrapped, None)
    }
}