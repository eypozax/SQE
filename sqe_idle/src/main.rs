use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, Box as GtkBox, TextView, ScrolledWindow, Orientation,
    Paned, WrapMode,
};
use webkit6::WebView;
use webkit6::prelude::WebViewExt; // for load_uri() and settings()
use std::process::Command;
use std::fs;
use std::path::{PathBuf, Path};
use std::time::{SystemTime, UNIX_EPOCH};
use std::thread;
use glib;

fn main() {
    let app = Application::builder()
        .application_id("com.mai.sqe_idle")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("SQE IDLE — editor + preview")
        .default_width(1100)
        .default_height(700)
        .build();

    // Horizontal split: editor | preview
    let paned = Paned::new(Orientation::Horizontal);

    // Left: editor
    let editor_box = GtkBox::new(Orientation::Vertical, 6);
    let text_view = TextView::new();
    text_view.set_wrap_mode(WrapMode::Word);
    let scrolled_editor = ScrolledWindow::builder()
        .child(&text_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    let run_button = Button::with_label("Run SQE → Render HTML");

    editor_box.append(&scrolled_editor);
    editor_box.append(&run_button);

    // Right: WebView preview
    let webview = WebView::new();
    if let Some(settings) = WebViewExt::settings(&webview) {
        settings.set_enable_developer_extras(true);
    }

    let scrolled_preview = ScrolledWindow::builder()
        .child(&webview)
        .vexpand(true)
        .hexpand(true)
        .build();

    // Put the two sides into the paned
    paned.set_start_child(Some(&editor_box));
    paned.set_end_child(Some(&scrolled_preview));

    // Channel: main thread receiver to get HTML path strings
    let (sender, receiver) = glib::MainContext::channel::<Option<String>>(glib::Priority::default());

    // When a path arrives, load it into the WebView
    let webview_clone = webview.clone();
    receiver.attach(None, move |html_path_opt: Option<String>| {
        if let Some(path_str) = html_path_opt {
            let uri = format!("file://{}", path_str);
            webview_clone.load_uri(&uri);
        }
        glib::Continue(true)
    });

    // Button click: spawn thread to run sqe-core and send result back
    let sender_for_button = sender.clone();
    let text_view_for_button = text_view.clone();
    run_button.connect_clicked(move |_| {
        let buffer = text_view_for_button.buffer();
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let sqe_code = buffer.text(&start, &end, true);

        // Make unique temp dir path (persistent until manual cleanup)
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let tmp_path = std::env::temp_dir().join(format!(
            "sqe_idle_{}_{}",
            ts,
            std::process::id()
        ));
        if let Err(e) = fs::create_dir_all(&tmp_path) {
            eprintln!("Failed to create temp dir {:?}: {}", tmp_path, e);
            let _ = sender_for_button.send(None);
            return;
        }

        // Write temp.sqe
        let sqe_file = tmp_path.join("temp.sqe");
        if let Err(e) = fs::write(&sqe_file, sqe_code.as_bytes()) {
            eprintln!("Failed to write temp.sqe: {}", e);
            let _ = sender_for_button.send(None);
            return;
        }

        // Spawn a thread to run the external process (so UI doesn't freeze)
        let sender_for_thread = sender_for_button.clone();
        let tmp_path_clone = tmp_path.clone();
        thread::spawn(move || {
            let run = Command::new("sqe-core")
                .arg("--input")
                .arg(&sqe_file)
                .arg("--output")
                .arg(&tmp_path_clone)
                .output();

            let run_out = match run {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("Failed to execute the sqe-core command: {}", e);
                    let _ = sender_for_thread.send(None);
                    return;
                }
            };

            if !run_out.status.success() {
                eprintln!(
                    "sqe-core failed with status {}:\n{}",
                    run_out.status,
                    String::from_utf8_lossy(&run_out.stderr)
                );
                let _ = sender_for_thread.send(None);
                return;
            }

            // Find first HTML file in tmp_path_clone
            eprintln!("Searching for HTML in {:?}", tmp_path_clone);
            match find_first_html(&tmp_path_clone) {
                Some(pathbuf) => {
                    eprintln!("Found HTML file: {:?}", pathbuf);
                    let path_str = pathbuf.to_string_lossy().to_string();
                    let _ = sender_for_thread.send(Some(path_str));
                }
                None => {
                    eprintln!("No HTML output found in {:?}", tmp_path_clone);
                    let _ = sender_for_thread.send(None);
                }
            }
        });
    });

    // Put paned into window and show
    let outer = GtkBox::new(Orientation::Vertical, 6);
    outer.append(&paned);
    window.set_child(Some(&outer));
    window.show();
}

/// Recursive search for the first .html/.htm file
fn find_first_html(dir: &Path) -> Option<PathBuf> {
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if let Ok(rd) = fs::read_dir(&path) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_file() {
                    if let Some(ext) = p.extension() {
                        if ext.eq_ignore_ascii_case("html")
                            || ext.eq_ignore_ascii_case("htm")
                        {
                            return Some(p);
                        }
                    }
                } else if p.is_dir() {
                    stack.push(p);
                }
            }
        }
    }
    None
}
