use crate::items::common::{escape_attr, escape_html, js_literal_for_key};

/// A Choose node: covers both multiple-choice and boolean-style questions.
#[derive(Debug, Clone)]
pub struct Choose {
    pub id: Option<String>,
    pub question: String,
    /// options = vec![(label, value_string), ...]
    pub options: Vec<(String, String)>,
    pub addons: Vec<String>,
    pub script_lines: Vec<String>,
}

impl Choose {
    pub fn parse(block: &str, id: Option<String>) -> Self {
        let mut lines = block.lines().map(|l| l.trim()).filter(|l| !l.is_empty());
        let question = lines.next().unwrap_or("⚠ no question").to_string();

        let mut options: Vec<(String, String)> = Vec::new();
        let mut addons: Vec<String> = Vec::new();
        let mut script_lines: Vec<String> = Vec::new();
        let mut auto_idx: usize = 0;

        let lines_vec: Vec<String> = lines.map(String::from).collect();
        let mut i = 0usize;
        while i < lines_vec.len() {
            let ln = lines_vec[i].trim();
            if ln.starts_with(".addons") {
                i += 1;
                while i < lines_vec.len() {
                    let inner = lines_vec[i].trim();
                    if inner == "]" {
                        break;
                    }
                    if inner.starts_with(".script") {
                        // handle inline or multi-line .script[ ... ]
                        if inner.contains('[') && inner.contains(']') {
                            if let Some(start) = inner.find('[') {
                                if let Some(end) = inner.rfind(']') {
                                    let s = inner[start + 1..end].trim();
                                    if !s.is_empty() {
                                        script_lines.push(s.to_string());
                                    }
                                }
                            }
                        } else if inner.contains('[') {
                            if let Some(pos) = inner.find('[') {
                                let after = inner[pos + 1..].trim();
                                if !after.is_empty() {
                                    script_lines.push(after.to_string());
                                }
                            }
                            i += 1;
                            while i < lines_vec.len() {
                                let scr = lines_vec[i].trim();
                                if scr.contains(']') {
                                    if let Some(pos) = scr.find(']') {
                                        let before = scr[..pos].trim();
                                        if !before.is_empty() {
                                            script_lines.push(before.to_string());
                                        }
                                    }
                                    break;
                                } else {
                                    script_lines.push(scr.to_string());
                                }
                                i += 1;
                            }
                        }
                    } else {
                        addons.push(inner.to_string());
                    }
                    i += 1;
                }
                i += 1;
                continue;
            }

            if ln.contains(">>") {
                let parts: Vec<&str> = ln.splitn(2, ">>").collect();
                let label = parts.get(0).map(|s| s.trim()).unwrap_or("").to_string();
                let val = parts.get(1).map(|s| s.trim()).unwrap_or("").to_string();
                options.push((label, val));
            } else {
                options.push((ln.to_string(), auto_idx.to_string()));
                auto_idx += 1;
            }

            i += 1;
        }

        Choose {
            id,
            question,
            options,
            addons,
            script_lines,
        }
    }

    pub fn render_html(&self, page_idx: usize, q_idx: usize) -> (String, Option<String>) {
        let qname = format!("p{}_q{}", page_idx, q_idx);
        let mut html = String::new();

        html.push_str(&format!(
            "<fieldset class=\"question\" data-q=\"{}\">",
            escape_html(&qname)
        ));
        html.push_str(&format!("<legend>{}</legend>", escape_html(&self.question)));

        for (opt_i, (label, value)) in self.options.iter().enumerate() {
            let input_id = format!("{}_opt{}", qname, opt_i);
            html.push_str(&format!(
                "<div><input type=\"radio\" id=\"{id}\" name=\"{qname}\" data-sqe-value=\"{val_esc}\"> <label for=\"{id}\">{label}</label></div>",
                id = escape_attr(&input_id),
                qname = escape_attr(&qname),
                val_esc = escape_attr(value),
                label = escape_html(label),
            ));
        }

        html.push_str("</fieldset>");

        let store_key = match &self.id {
            Some(s) if !s.is_empty() => s.clone(),
            _ => format!("{}_{}", page_idx, q_idx),
        };

        // Build JS with proper brace escaping for format!
        let mut js = format!(
            "(function() {{\n  if (!window.SQE_ANSWERS) window.SQE_ANSWERS = {{}};\n  const inputs = document.querySelectorAll(\"input[name='{}']\");\n  inputs.forEach(i => {{\n    i.addEventListener('change', function(e) {{\n      const raw = this.dataset.sqeValue;\n      const num = Number(raw);\n      const val = (Number.isFinite(num) && raw !== '') ? num : raw;\n      window.SQE_ANSWERS[{}] = val;\n      document.dispatchEvent(new CustomEvent('sqe:answer', {{ detail: {{ id: {}, value: val }} }}));\n    }});\n  }});\n}}());",
            qname,
            js_literal_for_key(&store_key),
            js_literal_for_key(&store_key)
        );

        if !self.script_lines.is_empty() {
            js.push_str("\n// user .script lines for this question\n");
            for line in &self.script_lines {
                js.push_str(line);
                js.push_str("\n");
            }
        }

        (html, Some(js))
    }
}
