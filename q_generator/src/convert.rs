// src/convert.rs
use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;

use crate::transcompiler::{Entry, Question};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
        .replace("\r\n", "\n")
        .replace('\n', "<br>")
}

pub fn build_pages(ast: &[Entry], out_dir: &str) -> io::Result<()> {
    create_dir_all(out_dir)?;

    // collect pages (title + content) in order
    let mut pages: Vec<(String, &Vec<Question>)> = Vec::new();
    for entry in ast {
        if let Entry::Page { title, content } = entry {
            pages.push((title.clone(), content));
        }
    }

    // prepare index.html
    let index_path = Path::new(out_dir).join("index.html");
    let mut f = File::create(&index_path)?;

    // --- HTML head & minimal styles ---
    writeln!(f, "<!doctype html>")?;
    writeln!(
        f,
        "<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">"
    )?;
    writeln!(f, "<title>Survey</title>")?;
    writeln!(f, "<style>")?;
    writeln!(
        f,
        "body{{font-family: system-ui, -apple-system, Roboto, 'Segoe UI', Arial; padding:20px; max-width:900px; margin:auto;}}"
    )?;
    writeln!(f, ".page{{display:none;}}")?;
    writeln!(f, ".page.active{{display:block;}}")?;
    writeln!(
        f,
        ".controls{{display:flex;justify-content:space-between;margin-top:18px;}}"
    )?;
    writeln!(
        f,
        ".question{{margin:12px 0;padding:10px;border-radius:8px;background:#f8f8f8;}}"
    )?;
    writeln!(
        f,
        "fieldset.question{{
        border:1px solid #ddd;
        padding:10px;
        border-radius:6px;
    }}"
    )?;
    writeln!(f, ".text-block{{margin:8px 0;}}")?;
    writeln!(
        f,
        ".page-indicator{{text-align:center;margin-top:12px;color:#666}}"
    )?;
    writeln!(f, "button:disabled{{opacity:.5;cursor:not-allowed}}")?;
    writeln!(f, "</style>")?;
    writeln!(f, "</head><body>")?;

    // header
    writeln!(f, "<h1>Survey</h1>")?;

    // container for pages
    writeln!(f, "<div id=\"pages\">")?;

    // collect page scripts (one string per page; null or empty string if none)
    let mut page_scripts: Vec<String> = Vec::new();

    for (i, (title, content)) in pages.iter().enumerate() {
        writeln!(f, "<section class=\"page\" data-index=\"{}\">", i)?;
        writeln!(f, "<h2>{}</h2>", escape_html(title))?;

        // render page content
        let mut scripts_for_page: Vec<String> = Vec::new();

        for q in content.iter() {
            match q {
                Question::Text { text } => {
                    writeln!(f, "<div class=\"text-block\">{}</div>", escape_html(text))?;
                }
                Question::Choice {
                    id: _,
                    question,
                    options,
                    script,
                } => {
                    writeln!(f, "<fieldset class=\"question\">")?;
                    writeln!(f, "<legend>{}</legend>", escape_html(question))?;
                    // radio options
                    for (opt_i, opt) in options.iter().enumerate() {
                        let input_id = format!(
                            "p{}_q{}_opt{}",
                            i, 0usize, /* not tracking question index globally */ opt_i
                        );
                        writeln!(
                            f,
                            "<div><input type=\"radio\" id=\"{}\" name=\"p{}_q{}\" value=\"{}\"> \
                             <label for=\"{}\">{}</label></div>",
                            input_id,
                            i,
                            0usize,
                            opt_i,
                            input_id,
                            escape_html(opt)
                        )?;
                    }
                    writeln!(f, "</fieldset>")?;

                    if let Some(s) = script {
                        // join with newline; store unescaped (will be injected as raw JS)
                        scripts_for_page.push(s.join("\n"));
                    }
                }
            }
        }

        // push combined script for this page (or empty string)
        if scripts_for_page.is_empty() {
            page_scripts.push(String::new());
        } else {
            page_scripts.push(scripts_for_page.join("\n\n"));
        }

        writeln!(f, "</section>")?;
    }

    writeln!(f, "</div>")?; // end pages container

    // navigation controls
    writeln!(f, "<div class=\"controls\">")?;
    writeln!(f, "<div><button id=\"prevBtn\">Previous</button></div>")?;
    writeln!(f, "<div><button id=\"nextBtn\">Next</button></div>")?;
    writeln!(f, "</div>")?;
    writeln!(
        f,
        "<div class=\"page-indicator\" id=\"pageIndicator\"></div>"
    )?;

    // embed scripts array and navigation logic
    writeln!(f, "<script>")?;

    // page count and scripts array (JSON safe string escaping)
    writeln!(f, "const PAGE_COUNT = {};", pages.len())?;

    // build JS array of page scripts, escaping backticks and closing script sequences
    write!(f, "const PAGE_SCRIPTS = [")?;
    for (idx, s) in page_scripts.iter().enumerate() {
        // escape backslashes and backticks for safe template literal
        let esc = s.replace("\\", "\\\\").replace('`', "\\`");
        if idx > 0 {
            write!(f, ",")?;
        }
        write!(f, "`{}`", esc)?;
    }
    writeln!(f, "];")?;

    // navigation logic
    writeln!(
        f,
        "{}",
        r#"
    document.addEventListener('DOMContentLoaded', () => {
        // grab elements after DOM ready
        const pages = Array.from(document.querySelectorAll('.page'));
        const prevBtn = document.getElementById('prevBtn');
        const nextBtn = document.getElementById('nextBtn');
        const pageIndicator = document.getElementById('pageIndicator');

        let currentIndex = 0;
        const ran = new Array(PAGE_COUNT).fill(false); // ensure page scripts run once

        function showPage(idx) {
            if (idx < 0) idx = 0;
            if (idx >= PAGE_COUNT) idx = PAGE_COUNT - 1;
            currentIndex = idx;

            pages.forEach((p, i) => {
                if (i === idx) {
                    p.classList.add('active');
                    p.style.display = '';
                } else {
                    p.classList.remove('active');
                    p.style.display = 'none';
                }
            });

            if (prevBtn) prevBtn.disabled = (idx === 0);
            if (nextBtn) nextBtn.disabled = (idx === PAGE_COUNT - 1);
            if (pageIndicator) pageIndicator.textContent = "Page " + (idx + 1) + " of " + PAGE_COUNT;

            // Run page script once (if present)
            try {
                const scriptText = PAGE_SCRIPTS[idx];
                if (!ran[idx] && scriptText && scriptText.trim().length > 0) {
                    new Function(scriptText)();
                    ran[idx] = true;
                }
            } catch (e) {
                console.error('Error running page script for page', idx + 1, e);
            }
        }

        // defensive listener attachment with debug logs
        if (prevBtn) {
            prevBtn.addEventListener('click', () => {
                console.log('prev clicked, currentIndex=', currentIndex);
                showPage(currentIndex - 1);
            });
        } else {
            console.warn('prevBtn not found');
        }

        if (nextBtn) {
            nextBtn.addEventListener('click', () => {
                console.log('next clicked, currentIndex=', currentIndex);
                showPage(currentIndex + 1);
            });
        } else {
            console.warn('nextBtn not found');
        }

        // initialize first page
        showPage(0);
    });
    "#
    )?;

    writeln!(f, "</script>")?;

    writeln!(f, "</body></html>")?;
    Ok(())
}
