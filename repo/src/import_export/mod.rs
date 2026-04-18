pub fn check_file_signature(data: &[u8], filename: &str) -> Result<&'static str, String> {
    let ext = filename
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "csv" => {
            // CSV: text-based, check it's valid UTF-8 text
            if data.len() >= 2
                && std::str::from_utf8(&data[..std::cmp::min(data.len(), 1024)]).is_ok()
            {
                Ok("csv")
            } else {
                Err("Invalid CSV file signature".to_string())
            }
        }
        "xlsx" => {
            // XLSX: ZIP-based, check PK signature
            if data.len() >= 4 && data[0] == 0x50 && data[1] == 0x4B {
                Ok("xlsx")
            } else {
                Err("Invalid XLSX file signature".to_string())
            }
        }
        _ => Err(format!("Unsupported file type: {}", ext)),
    }
}

pub fn normalize_title(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn mask_sensitive_value(value: &str) -> String {
    // Email pattern
    let email_re =
        regex::Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap();
    let result = email_re.replace_all(value, "***@***.***").to_string();

    // Phone pattern
    let phone_re = regex::Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap();
    let result = phone_re.replace_all(&result, "***-***-****").to_string();

    // SSN pattern
    let ssn_re = regex::Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap();
    ssn_re.replace_all(&result, "***-**-****").to_string()
}
