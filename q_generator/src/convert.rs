// === src/convert.rs ===

use std::fs::{File, create_dir_all};
use std::io::{self, Write};
use std::path::Path;

use crate::items::common::{escape_html, to_js_string};
use crate::transcompiler::{Entry, Question};

pub fn build_pages(ast: &[Entry], out_dir: &str) -> io::Result<()> {
    create_dir_all(out_dir)?;

    // Collect document-level title (if any) and pages.
    let mut pages: Vec<(String, &Vec<Question>)> = Vec::new();
    let mut doc_title_opt: Option<String> = None;
    for entry in ast {
        match entry {
            Entry::DocTitle(t) => {
                // first DocTitle wins; later ones overwrite previous
                doc_title_opt = Some(t.clone());
            }
            Entry::Page { title, content } => {
                pages.push((title.clone(), content));
            }
            _ => {}
        }
    }
 
    let index_path = Path::new(out_dir).join("index.html");
    let mut f = File::create(&index_path)?;
 
    // Determine document title: prefer explicit DocTitle, else first page title, else fallback.
    let doc_title = if let Some(ref t) = doc_title_opt {
        t.clone()
    } else if !pages.is_empty() {
        pages[0].0.clone()
    } else {
        "Survey".to_string()
    };
 
    writeln!(f, "<!doctype html>")?;
    writeln!(
        f,
        "<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">"
    )?;
    writeln!(f, "<title>{}</title>", escape_html(&doc_title))?;
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

    writeln!(f, "<h1>{}</h1>", escape_html(&doc_title))?;
    writeln!(f, "<div id=\"pages\">")?;

    // Collect per-page scripts as arrays of stringified objects (setup scripts and placeholders)
    let mut page_scripts: Vec<Vec<String>> = Vec::new();

    for (i, (title, content)) in pages.iter().enumerate() {
        writeln!(f, "<section class=\"page\" data-index=\"{}\">", i)?;
        // Show the per-page H2 normally. Only suppress the per-page H2 for the first page
        // when an explicit DocTitle exists and it exactly matches the page title (to avoid duplicate text).
        let show_page_header = !(i == 0 && doc_title_opt.is_some() && doc_title == *title);
        if show_page_header {
            writeln!(f, "<h2>{}</h2>", escape_html(title))?;
        }

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
                        // Wrap choose setup JS as an object with only "script"
                        let obj = format!("{{\"script\":{}}}", to_js_string(&js));
                        scripts_for_page.push(obj);
                    }
                    q_local_idx += 1;
                }
                Question::Html(node) => {
                    let (html_frag, _maybe_js) = node.render_html();
                    writeln!(f, "{}", html_frag)?;
                }
                Question::Css(node) => {
                    // Css.render_html returns a wrapped <style>...</style> fragment; insert it inline.
                    let (style_frag, _maybe_js) = node.render_html();
                    writeln!(f, "{}", style_frag)?;
                }
                Question::Js(node) => {
                    let (_html_frag, maybe_js) = node.render_html();
                    if let Some(js) = maybe_js {
                        // Treat user js as a setup/script entry for the page (no placeholder id)
                        let obj = format!("{{\"script\":{}}}", to_js_string(&js));
                        scripts_for_page.push(obj);
                    }
                }
            }
        }
 
        // push the per-page script objects (may be empty)
        page_scripts.push(scripts_for_page);
 
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
    for (pidx, page) in page_scripts.iter().enumerate() {
        if pidx > 0 {
            write!(f, ",")?;
        }
        write!(f, "[")?;
        for (sidx, obj) in page.iter().enumerate() {
            if sidx > 0 {
                write!(f, ",")?;
            }
            // `obj` may be either:
            //  - a JS object literal string like {"id":"p1_fn0","script":"..."}
            //  - or a raw JS snippet (legacy). Normalize by wrapping non-object literals
            //    into {"script": "<escaped-string>"} so runtime always sees objects.
            let trimmed = obj.trim_start();
            if trimmed.starts_with('{') {
                write!(f, "{}", obj)?;
            } else {
                write!(f, "{{\"script\":{}}}", to_js_string(obj))?;
            }
        }
        write!(f, "]")?;
    }
    writeln!(f, "];")?;

    // Updated nav / runtime JS: defines SQE API and runs page scripts robustly (supports async and return values)
    let nav_js = r#"document.addEventListener("DOMContentLoaded", () => {
    // tiny runtime API for f { ... } scripts
    window.SQE = window.SQE || {};
    const SQE = window.SQE;
    
    // Collect answers from DOM inputs marked with data-sqe-key.
    // This lets user scripts read window.SQE_ANSWERS immediately without needing to manually gather values.
    // Improvements: support checkbox groups as arrays, robustly coerce numbers, and debug-log collected values.
    SQE.collectAnswers = SQE.collectAnswers || function() {
      window.SQE_ANSWERS = window.SQE_ANSWERS || {};
      const els = document.querySelectorAll("[data-sqe-key]");
      const seen = new Set();
      els.forEach(el => {
        const key = el.getAttribute("data-sqe-key");
        if (!key || seen.has(key)) return;
        seen.add(key);
        const group = Array.from(document.querySelectorAll('[data-sqe-key="'+key+'"]'));
        // Determine element kinds in the group
        const types = new Set(group.map(g => (g.type || g.tagName || '').toLowerCase()));
        // collect values
        if (types.has('checkbox')) {
          // collect array of checked values
          const vals = [];
          group.forEach(g => {
            try {
              if (g.checked) {
                const v = g.getAttribute('data-sqe-value') ?? g.value;
                vals.push(v);
              }
            } catch(e){}
          });
          window.SQE_ANSWERS[key] = vals;
          console.debug("[SQE] collectAnswers:", key, "=", vals);
        } else if (types.has('radio')) {
          // single value
          let val = null;
          group.forEach(g => { try { if (g.checked) val = g.getAttribute('data-sqe-value') ?? g.value; } catch(e){} });
          const num = Number(val);
          const final = (val === null) ? null : (Number.isFinite(num) && val !== '') ? num : val;
          window.SQE_ANSWERS[key] = final;
          console.debug("[SQE] collectAnswers:", key, "=", final);
        } else {
          // fall back to last non-empty value in group (e.g., text inputs)
          let val = null;
          group.forEach(g => { try { if (typeof g.value !== 'undefined' && g.value !== '') val = g.value; } catch(e){} });
          const num = Number(val);
          const final = (val === null) ? null : (Number.isFinite(num) && val !== '') ? num : val;
          window.SQE_ANSWERS[key] = final;
          console.debug("[SQE] collectAnswers:", key, "=", final);
        }
      });
    };
    
    // Debounced runner to reduce excessive runs on rapid input events
    let _sqeRunTimer = null;
    function runAllFunctionsDebounced() {
      if (_sqeRunTimer) clearTimeout(_sqeRunTimer);
      _sqeRunTimer = setTimeout(() => { _sqeRunTimer = null; runAllFunctions(); }, 30);
    }
    
    // Automatically update answers when inputs change or when custom sqe:answer events are dispatched.
    // When answers change we re-run all function blocks (f { ... }) across all pages so
    // function placeholders update immediately and stay correct even when navigating back/forward.
    function runAllFunctions() {
      try {
        for (let pi = 0; pi < PAGE_SCRIPTS.length; pi++) {
          const scriptsForPage = PAGE_SCRIPTS[pi];
          if (scriptsForPage && scriptsForPage.length > 0) {
            runScriptsForPage(pi, scriptsForPage);
          }
        }
      } catch (e) {
        // don't let a failure block other handlers
        console.error("Error running all page functions", e);
      }
    }
    // listen for a wider set of events (input + change + explicit sqe:answer)
    document.addEventListener('input', function() { try { SQE.collectAnswers(); runAllFunctionsDebounced(); } catch(e){} }, true);
    document.addEventListener('change', function() { try { SQE.collectAnswers(); runAllFunctionsDebounced(); } catch(e){} }, true);
    document.addEventListener('sqe:answer', function() { try { SQE.collectAnswers(); runAllFunctionsDebounced(); } catch(e){} });
    
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
    
    // Run all scripts for a page (each script is an object { id: "...", script: "..." })
    function runScriptsForPage(idx, scriptsArray) {
        if (!scriptsArray || !Array.isArray(scriptsArray) || scriptsArray.length === 0) return;
        try {
            // Ensure latest answers are collected before running page scripts.
            try { if (typeof SQE.collectAnswers === 'function') SQE.collectAnswers(); } catch(e) {}
 
            scriptsArray.forEach(s => {
                try {
                    if (!s || typeof s !== 'object') return;
                    const id = s.id || null;
                    const scriptText = s.script;
                    if (!scriptText) return;
                    // Guard: avoid executing empty scripts
                    if (!String(scriptText).trim()) return;
 
                    // Logging: show which script is about to run (trim for brevity)
                    try { console.debug("[SQE] runScriptsForPage: page", idx, "id", id, "scriptSnippet", String(scriptText).slice(0,120)); } catch(e){}
 
                    if (id) {
                        // function block with a placeholder target
                        const selector = '[data-sqe-fn="'+String(id).replace(/"/g,'\\"')+'"]';
                        const target = document.querySelector(selector);
                        if (!target) return;
                        // Clear previous output in the placeholder
                        target.innerHTML = '';
 
                        let exec;
                        try {
                            exec = new Function('return (async function(){\n' + scriptText + '\n})()')();
                        } catch(e) {
                            console.error("Error constructing function for", id, e);
                            return;
                        }
 
                        const handleResult = (res) => {
                            try {
                                if (res === null || typeof res === 'undefined') {
                                    // nothing to render
                                    return;
                                } else if (typeof res === 'string' || typeof res === 'number' || typeof res === 'boolean') {
                                    const d = document.createElement('div');
                                    d.className = 'text-block';
                                    d.textContent = String(res);
                                    target.appendChild(d);
                                } else if (res instanceof Node) {
                                    target.appendChild(res);
                                } else if (Array.isArray(res)) {
                                    res.forEach(item => {
                                        if (typeof item === 'string' || typeof item === 'number' || typeof item === 'boolean') {
                                            const d = document.createElement('div');
                                            d.className = 'text-block';
                                            d.textContent = String(item);
                                            target.appendChild(d);
                                        } else if (item instanceof Node) {
                                            target.appendChild(item);
                                        } else {
                                            // fallback: stringify
                                            const d = document.createElement('pre');
                                            d.className = 'text-block';
                                            d.textContent = JSON.stringify(item, null, 2);
                                            target.appendChild(d);
                                        }
                                    });
                                } else {
                                    // object or other: stringify to pre
                                    const d = document.createElement('pre');
                                    d.className = 'text-block';
                                    d.textContent = JSON.stringify(res, null, 2);
                                    target.appendChild(d);
                                }
                            } catch (e) {
                                console.error("Error handling script result for function", id, e);
                            }
                        };
 
                        if (exec && typeof exec.then === 'function') {
                            exec.then(handleResult).catch(e => { console.error("Error running function", id, e); });
                        } else {
                            handleResult(exec);
                        }
                    } else {
                        // setup/side-effect script (no placeholder) — execute but ignore return value
                        try {
                            const exec = new Function('return (async function(){\n' + scriptText + '\n})()')();
                            if (exec && typeof exec.then === 'function') {
                                exec.catch(e => { console.error("Error running setup script", e); });
                            }
                        } catch (e) {
                            console.error("Error executing setup script", e);
                        }
                    }
                } catch(e){
                    console.error("Error executing script object", e);
                }
            });
        } catch (e) {
            console.error("Error running page scripts for page", idx + 1, e);
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
            const scriptsForPage = PAGE_SCRIPTS[idx];
            if (scriptsForPage && scriptsForPage.length > 0) {
                runScriptsForPage(idx, scriptsForPage);
                ran[idx] = true;
            }
        } catch (e) {
            console.error("Error running page scripts for page", idx + 1, e);
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
