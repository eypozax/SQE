pub fn return_text(input: &str) -> String {
    let converted_input = format!("<p>{}</p>", input.replace("\n", "<br>"));
    converted_input
}
