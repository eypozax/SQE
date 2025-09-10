// === src/items/qrcode.rs ===


pub fn return_qrcode(web_address: &str) -> Result<String, Box<dyn std::error::Error>> {
    use qrcode::QrCode;  // Import the qrcode crate
    use qrcode::render::unicode::Dense1x2;

    // Convert the qrcode crate error into a boxed error for our return type
    let code = QrCode::new(web_address).map_err(|e| Box::<dyn std::error::Error>::from(e))?;
    let qr_string = code.render::<Dense1x2>().module_dimensions(2, 1).build();
    Ok(qr_string)
}

/// Convenience helper to print a terminal-rendered QR code and handle errors in one place.
/// Keeps callers simple and avoids repeating the same match block in multiple places.
pub fn print_qrcode(web_address: &str) {
    match return_qrcode(web_address) {
        Ok(qr) => {
            println!("\nTerminal QR code for {}\n", web_address);
            println!("{}", qr);
        }
        Err(e) => {
            eprintln!("Failed to generate QR code: {}", e);
        }
    }
}
