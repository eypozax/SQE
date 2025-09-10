use std::process::Command;

fn main() {
    // Try to obtain the current git commit (short). Fall back to "unknown".
    let git_commit = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| if o.status.success() { String::from_utf8(o.stdout).ok() } else { None })
        .map(|s| s.trim().to_owned())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=GIT_COMMIT={}", git_commit);

    // Build time in RFC3339 (UTC). Using chrono from build-dependencies.
    let build_time = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
}