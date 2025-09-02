// === src/items/common.rs ===

/// Small shared helpers used by item renderers.
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

pub fn escape_attr(s: &str) -> String {
    escape_html(s)
}

pub fn js_literal_for_key(k: &str) -> String {
    let esc = k
        .replace("\\", "\\\\")
        .replace("\"", "\\\"")
        .replace("\n", "\\n")
        .replace("\r", "\\r");
    format!("\"{}\"", esc)
}

pub fn to_js_string(s: &str) -> String {
    let esc = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace("</script>", "<\\/script>");
    format!("\"{}\"", esc)
}
