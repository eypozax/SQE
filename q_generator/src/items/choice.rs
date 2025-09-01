#[derive(Debug)]
pub struct Choice {
    pub question: String,
    pub options: Vec<String>,
}

pub fn return_choice(block: &str) -> Choice {
    let mut lines = block
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty());

    // first line = the question
    let question = lines
        .next()
        .unwrap_or_else(|| "⚠ no question found".to_string());

    // remaining lines = options
    let options: Vec<String> = lines.collect();

    Choice { question, options }
}
