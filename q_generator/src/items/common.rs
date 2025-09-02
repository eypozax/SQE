// === src/items/common.rs ===

/// Small shared helpers used by item renderers.
///
/// Keep these small and robust — they are relied on when producing HTML/JS.
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

/// Produce a JS string literal suitable for embedding directly into generated JS.
/// Uses `serde_json::to_string` for correct escaping where possible, and falls back
/// to a manual escape routine on error (never panics). Also neutralizes `</script>`.
pub fn to_js_string(s: &str) -> String {
    // Prefer serde_json for correctness; it's unlikely to fail, but handle errors safely.
    match serde_json::to_string(s) {
        Ok(mut js) => {
            if js.contains("</script>") {
                js = js.replace("</script>", "<\\/script>");
            }
            js
        }
        Err(_) => {
            // Fallback — escape typical problematic characters and wrap in quotes.
            let mut esc = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\r', "\\r")
                .replace('\n', "\\n")
                .replace('\t', "\\t");
            esc = esc.replace("</script>", "<\\/script>");
            format!("\"{}\"", esc)
        }
    }
}

/// Produce a literal for small keys that go into JS — delegates to `to_js_string`.
pub fn js_literal_for_key(k: &str) -> String {
    to_js_string(k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_js_string_escapes() {
        let raw = "hello \" world </script>\nnew";
        let js = to_js_string(raw);
        // must be a quoted JS string
        assert!(js.starts_with('"') && js.ends_with('"'));
        // must not contain a raw </script>
        assert!(!js.contains("</script>"));
        // basic escaped newline should appear
        assert!(js.contains("\\n"));
    }

    #[test]
    fn escape_html_basic() {
        let raw = "<a & '\">";
        let out = escape_html(raw);
        assert!(out.contains("&lt;") && out.contains("&amp;") && out.contains("&quot;"));
    }
}
