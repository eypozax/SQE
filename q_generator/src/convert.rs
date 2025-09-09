// === src/convert.rs ===

use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;

use crate::items::common::{escape_html, to_js_string};
use crate::transcompiler::{Entry, Question};

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
                Question::Insert(insert) => {
                    let (html_frag, _maybe_js) = insert.render_html();
                    writeln!(f, "{}", html_frag)?;
                }
                Question::Choose(choose) => {
                    let (html_frag, maybe_js) = choose.render_html(i, q_local_idx);
                    writeln!(f, "{}", html_frag)?;
                    if let Some(js) = maybe_js {
                        scripts_for_page.push(js);
                    }
                    q_local_idx += 1;
                }
                Question::Function(func) => {
                    // Functions produce JS only (no HTML)
                    let (_html_frag, maybe_js) = func.render_html();
                    if let Some(js) = maybe_js {
                        scripts_for_page.push(js);
                    }
                }
            }
        }

        if scripts_for_page.is_empty() {
            page_scripts.push(String::new());
        } else {
            page_scripts.push(scripts_for_page.join("\n"));
        }

        writeln!(f, "</section>")?;
    }

    writeln!(f, "</div>")?;
    writeln!(f, "<div class=\"controls\">")?;
    writeln!(f, "<div><button id=\"prevBtn\">Previous</button></div>")?;
    writeln!(f, "<div><button id=\"nextBtn\">Next</button></div>")?;
    writeln!(f, "</div>")?;
    writeln!(f, "<div id=\"saveBtnContainer\" style=\"text-align:center; margin-top:20px; display:none;\">")?;
    writeln!(f, "<button id=\"saveBtn\">Save Answers</button>")?;
    writeln!(f, "</div>")?;
    writeln!(
        f,
        "<div class=\"page-indicator\" id=\"pageIndicator\"></div>"
    )?;

    writeln!(f, "<script>")?;
    writeln!(f, "const PAGE_COUNT = {};", pages.len())?;

    write!(f, "const PAGE_SCRIPTS = [")?;
    for (idx, s) in page_scripts.iter().enumerate() {
        if idx > 0 {
            write!(f, ",")?;
        }
        write!(f, "{}", to_js_string(s))?;
    }
    writeln!(f, "];")?;

    // Updated nav / runtime JS: defines SQE API and runs page scripts robustly (supports async and return values)
    let nav_js = r#"document.addEventListener("DOMContentLoaded", () => {
    // tiny runtime API for f { ... } scripts
    window.SQE = window.SQE || {};
    const SQE = window.SQE;

    // insert plain text as a .text-block (escaped by using textContent)
    SQE.insert = SQE.insert || function(text) {
        const page = document.querySelector(".page.active");
        if (!page) return null;
        const div = document.createElement("div");
        div.className = "text-block";
        div.textContent = String(text);
        page.appendChild(div);
        return div;
    };

    // insert raw HTML (use with care)
    SQE.insertHTML = SQE.insertHTML || function(html) {
        const page = document.querySelector(".page.active");
        if (!page) return null;
        const div = document.createElement("div");
        div.className = "text-block";
        div.innerHTML = String(html);
        page.appendChild(div);
        return div;
    };

    SQE.getAnswer = SQE.getAnswer || function(key) {
        return (window.SQE_ANSWERS || {})[key];
    };

    SQE.setAnswer = SQE.setAnswer || function(key, val) {
        window.SQE_ANSWERS = window.SQE_ANSWERS || {};
        window.SQE_ANSWERS[key] = val;
        // dispatch the same event that choice-rendered inputs use
        document.dispatchEvent(new CustomEvent('sqe:answer', { detail: { id: key, value: val } }));
    };

    const pages = Array.from(document.querySelectorAll(".page"));
    const prevBtn = document.getElementById("prevBtn");
    const nextBtn = document.getElementById("nextBtn");
    const pageIndicator = document.getElementById("pageIndicator");

    let currentIndex = 0;
    const ran = new Array(PAGE_COUNT).fill(false);

    function runScriptForPage(idx, scriptText) {
        if (!scriptText || !scriptText.trim()) return;
        try {
            // Wrap the user's script inside an async IIFE so `await` is supported.
            // The wrapped function may return a value (string or DOM Node) or a Promise resolving to one.
            const execPromise = new Function('return (async function(){\n' + scriptText + '\n})()')();

            const handleResult = (res) => {
                try {
                    if (typeof res === 'string') {
                        SQE.insert(res);
                    } else if (res instanceof Node) {
                        const page = pages[idx];
                        if (page) page.appendChild(res);
                    } else if (res && Array.isArray(res)) {
                        // array of strings or nodes
                        res.forEach(item => {
                            if (typeof item === 'string') SQE.insert(item);
                            else if (item instanceof Node) {
                                const page = pages[idx];
                                if (page) page.appendChild(item);
                            }
                        });
                    }
                    // otherwise ignore undefined/null/other results
                } catch (e) {
                    console.error("Error handling script result for page", idx + 1, e);
                }
            };

            if (execPromise && typeof execPromise.then === 'function') {
                execPromise.then(handleResult).catch(e => {
                    console.error("Error running page script (async) for page", idx + 1, e);
                });
            } else {
                handleResult(execPromise);
            }
        } catch (e) {
            console.error("Error running page script for page", idx + 1, e);
        }
    }

    function showPage(idx) {
        if (idx < 0) idx = 0;
        if (idx >= PAGE_COUNT) idx = PAGE_COUNT - 1;
        currentIndex = idx;

        pages.forEach((p, i) => {
            if (i === idx) {
                p.classList.add("active");
                p.style.display = "";
            } else {
                p.classList.remove("active");
                p.style.display = "none";
            }
        });

        if (prevBtn) prevBtn.disabled = (idx === 0);
        if (nextBtn) nextBtn.disabled = (idx === PAGE_COUNT - 1);
        if (pageIndicator) pageIndicator.textContent = "Page " + (idx + 1) + " of " + PAGE_COUNT;
        const saveContainer = document.getElementById("saveBtnContainer");
        if (saveContainer) {
            if (idx === PAGE_COUNT - 1) {
                saveContainer.style.display = "block";
            } else {
                saveContainer.style.display = "none";
            }
        }

        try {
            const scriptText = PAGE_SCRIPTS[idx];
            if (!ran[idx] && scriptText && scriptText.trim().length > 0) {
                runScriptForPage(idx, scriptText);
                ran[idx] = true;
            }
        } catch (e) {
            console.error("Error running page script for page", idx + 1, e);
        }
    }

    if (prevBtn) {
        prevBtn.addEventListener("click", () => { showPage(currentIndex - 1); });
    }
    if (nextBtn) {
        nextBtn.addEventListener("click", () => { showPage(currentIndex + 1); });
    }

    showPage(0);
});"#;

    writeln!(f, "{}", nav_js)?;

    writeln!(f, "{}", r#"
document.addEventListener("DOMContentLoaded", () => {
    const saveBtn = document.getElementById("saveBtn");
    if (saveBtn) {
        saveBtn.addEventListener("click", () => {
            const data = JSON.stringify(window.SQE_ANSWERS || {}, null, 2);
            const blob = new Blob([data], { type: "application/json" });
            const url = URL.createObjectURL(blob);
            const a = document.createElement("a");
            a.href = url;
            a.download = "answers.json";
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        });
    }
});
"#)?;
    writeln!(f, "</script>")?;
    writeln!(f, "</body></html>")?;
    Ok(())
}
