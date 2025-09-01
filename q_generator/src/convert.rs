use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;

use crate::transcompiler::{Entry, Question};

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#x27;")
        .replace("\r\n", "\n")
        .replace('\n', "<br>")
}

pub fn build_pages(ast: &[Entry], out_dir: &str) -> io::Result<()> {
    create_dir_all(out_dir)?;

    let mut pages: Vec<(String, &Vec<Question>)> = Vec::new();
    for entry in ast {
        if let Entry::Page { title, content } = entry {
            pages.push((title.clone(), content));
        }
    }

    let index_path = Path::new(out_dir).join("index.html");
    let mut f = File::create(&index_path)?;

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
        "fieldset.question{{border:1px solid #ddd;padding:10px;border-radius:6px;}}"
    )?;
    writeln!(f, ".text-block{{margin:8px 0;}}")?;
    writeln!(
        f,
        ".page-indicator{{text-align:center;margin-top:12px;color:#666}}"
    )?;
    writeln!(f, "button:disabled{{opacity:.5;cursor:not-allowed}}")?;
    writeln!(f, "</style>")?;
    writeln!(f, "</head><body>")?;

    writeln!(f, "<h1>Survey</h1>")?;
    writeln!(f, "<div id=\"pages\">")?;

    let mut page_scripts: Vec<String> = Vec::new();

    for (i, (title, content)) in pages.iter().enumerate() {
        writeln!(f, "<section class=\"page\" data-index=\"{}\">", i)?;
        writeln!(f, "<h2>{}</h2>", escape_html(title))?;

        let mut scripts_for_page: Vec<String> = Vec::new();
        let mut q_local_idx = 0usize;

        for q in content.iter() {
            match q {
                Question::Text { text } => {
                    writeln!(f, "<div class=\"text-block\">{}</div>", escape_html(text))?;
                }
                Question::Choose(choose) => {
                    let (html_frag, maybe_js) = choose.render_html(i, q_local_idx);
                    writeln!(f, "{}", html_frag)?;
                    if let Some(js) = maybe_js {
                        scripts_for_page.push(js);
                    }
                    q_local_idx += 1;
                }
            }
        }

        if scripts_for_page.is_empty() {
            page_scripts.push(String::new());
        } else {
            page_scripts.push(scripts_for_page.join("\n\n"));
        }

        writeln!(f, "</section>")?;
    }

    writeln!(f, "</div>")?;
    writeln!(f, "<div class=\"controls\">")?;
    writeln!(f, "<div><button id=\"prevBtn\">Previous</button></div>")?;
    writeln!(f, "<div><button id=\"nextBtn\">Next</button></div>")?;
    writeln!(f, "</div>")?;
    writeln!(
        f,
        "<div class=\"page-indicator\" id=\"pageIndicator\"></div>"
    )?;

    writeln!(f, "<script>")?;
    writeln!(f, "const PAGE_COUNT = {};", pages.len())?;

    write!(f, "const PAGE_SCRIPTS = [")?;
    for (idx, s) in page_scripts.iter().enumerate() {
        let esc = s.replace("\\", "\\\\").replace('`', "\\`");
        if idx > 0 {
            write!(f, ",")?;
        }
        write!(f, "`{}`", esc)?;
    }
    writeln!(f, "];")?;

    writeln!(
        f,
        "{}",
        r#"document.addEventListener('DOMContentLoaded', () => {
    const pages = Array.from(document.querySelectorAll('.page'));
    const prevBtn = document.getElementById('prevBtn');
    const nextBtn = document.getElementById('nextBtn');
    const pageIndicator = document.getElementById('pageIndicator');

    let currentIndex = 0;
    const ran = new Array(PAGE_COUNT).fill(false);

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

    if (prevBtn) {
        prevBtn.addEventListener('click', () => { showPage(currentIndex - 1); });
    }
    if (nextBtn) {
        nextBtn.addEventListener('click', () => { showPage(currentIndex + 1); });
    }

    showPage(0);
});"#
    )?;

    writeln!(f, "</script>")?;
    writeln!(f, "</body></html>")?;
    Ok(())
}
