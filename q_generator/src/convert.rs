// convert.rs
use std::sync::Mutex;

lazy_static! {
    // Global HTML skeleton
    static ref HTML: Mutex<String> = Mutex::new(String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>Rust Generated Page</title>
</head>
<body>
</body>
</html>"#
    ));
}

/// Injects an HTML snippet into the global template
pub fn insert(object: &str) {
    let mut html = HTML.lock().unwrap();

    // Insert just before </body>
    if let Some(pos) = html.rfind("</body>") {
        html.insert_str(pos, &format!("    {}\n", object));
    }
}

/// Returns the full HTML as a string
pub fn build_page() -> String {
    let html = HTML.lock().unwrap().clone(); // get the string

    // Write to file
    if let Err(e) = std::fs::write("output.html", &html) {
        eprintln!("Failed to write HTML file: {}", e);
    }

    html // return the HTML string
}

/// Optional: Reset the global template (useful if generating multiple pages)
pub fn reset() {
    let mut html = HTML.lock().unwrap();
    *html = String::from(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width,initial-scale=1" />
    <title>Rust Generated Page</title>
</head>
<body>
</body>
</html>"#,
    );
}
