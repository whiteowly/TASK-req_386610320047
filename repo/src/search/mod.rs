pub fn normalize_query(query: &str) -> String {
    query
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn to_tsquery(normalized: &str) -> String {
    normalized
        .split_whitespace()
        .filter(|w| w.len() > 1)
        .map(|w| format!("{}:*", w.replace('\'', "")))
        .collect::<Vec<_>>()
        .join(" & ")
}
