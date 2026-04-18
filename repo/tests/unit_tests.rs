// =============================================================================
// tests/unit_tests.rs
//
// Rust-native unit tests for KnowledgeOps service-layer and utility logic.
// These run via `cargo test` and produce measurable line coverage.
//
// Test classification: Unit / module-level (no HTTP, no DB)
// Covers: crypto, search normalization, import_export helpers, masking
// =============================================================================

// ---- Crypto: password hashing ----

#[test]
fn test_password_hash_and_verify() {
    let hash = knowledgeops::crypto::hash_password("changeme123!").unwrap();
    assert!(hash.starts_with("$argon2id$"));
    assert!(knowledgeops::crypto::verify_password("changeme123!", &hash));
    assert!(!knowledgeops::crypto::verify_password("wrongpassword", &hash));
}

#[test]
fn test_password_hash_different_each_time() {
    let h1 = knowledgeops::crypto::hash_password("same").unwrap();
    let h2 = knowledgeops::crypto::hash_password("same").unwrap();
    assert_ne!(h1, h2, "Argon2id salts should differ");
    assert!(knowledgeops::crypto::verify_password("same", &h1));
    assert!(knowledgeops::crypto::verify_password("same", &h2));
}

#[test]
fn test_verify_password_bad_hash() {
    assert!(!knowledgeops::crypto::verify_password("any", "not-a-valid-hash"));
}

// ---- Crypto: token hashing ----

#[test]
fn test_token_hash_deterministic() {
    let h1 = knowledgeops::crypto::hash_token("abc123");
    let h2 = knowledgeops::crypto::hash_token("abc123");
    assert_eq!(h1, h2);
    assert_eq!(h1.len(), 64, "SHA-256 hex should be 64 chars");
}

#[test]
fn test_token_hash_different_inputs() {
    let h1 = knowledgeops::crypto::hash_token("token_a");
    let h2 = knowledgeops::crypto::hash_token("token_b");
    assert_ne!(h1, h2);
}

// ---- Crypto: session token generation ----

#[test]
fn test_generate_session_token_format() {
    let token = knowledgeops::crypto::generate_session_token();
    assert_eq!(token.len(), 64, "Session token should be 64 hex chars");
    assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_generate_session_token_unique() {
    let t1 = knowledgeops::crypto::generate_session_token();
    let t2 = knowledgeops::crypto::generate_session_token();
    assert_ne!(t1, t2, "Tokens should be unique");
}

// ---- Crypto: encryption/decryption ----

#[test]
fn test_encrypt_decrypt_roundtrip() {
    let key = [42u8; 32]; // 32-byte key
    let plaintext = "sensitive-email@example.com";
    let encrypted = knowledgeops::crypto::encrypt_value(plaintext, &key).unwrap();
    assert_ne!(encrypted, plaintext);
    let decrypted = knowledgeops::crypto::decrypt_value(&encrypted, &key).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_wrong_key_fails() {
    let key1 = [1u8; 32];
    let key2 = [2u8; 32];
    let encrypted = knowledgeops::crypto::encrypt_value("secret", &key1).unwrap();
    let result = knowledgeops::crypto::decrypt_value(&encrypted, &key2);
    assert!(result.is_err(), "Decryption with wrong key should fail");
}

#[test]
fn test_encrypt_bad_key_length() {
    let short_key = [0u8; 16];
    assert!(knowledgeops::crypto::encrypt_value("data", &short_key).is_err());
    assert!(knowledgeops::crypto::decrypt_value("data", &short_key).is_err());
}

#[test]
fn test_decrypt_invalid_base64() {
    let key = [0u8; 32];
    assert!(knowledgeops::crypto::decrypt_value("not-base64!!!", &key).is_err());
}

#[test]
fn test_decrypt_too_short() {
    let key = [0u8; 32];
    // Valid base64 but less than 12 bytes decoded
    let short = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &[0u8; 5]);
    assert!(knowledgeops::crypto::decrypt_value(&short, &key).is_err());
}

// ---- Search normalization ----

#[test]
fn test_normalize_query_trims_and_lowercases() {
    assert_eq!(knowledgeops::search::normalize_query("  Hello World  "), "hello world");
}

#[test]
fn test_normalize_query_collapses_whitespace() {
    assert_eq!(knowledgeops::search::normalize_query("a   b\tc"), "a b c");
}

#[test]
fn test_normalize_query_empty() {
    assert_eq!(knowledgeops::search::normalize_query(""), "");
    assert_eq!(knowledgeops::search::normalize_query("   "), "");
}

#[test]
fn test_to_tsquery_basic() {
    let result = knowledgeops::search::to_tsquery("hello world");
    assert_eq!(result, "hello:* & world:*");
}

#[test]
fn test_to_tsquery_filters_short_words() {
    let result = knowledgeops::search::to_tsquery("a be see");
    // "a" is 1 char (filtered), "be" is 2 chars (kept), "see" is 3 chars (kept)
    assert_eq!(result, "be:* & see:*");
}

#[test]
fn test_to_tsquery_strips_quotes() {
    let result = knowledgeops::search::to_tsquery("it's fine");
    assert!(!result.contains('\''), "Single quotes should be stripped");
}

#[test]
fn test_to_tsquery_empty() {
    assert_eq!(knowledgeops::search::to_tsquery(""), "");
    assert_eq!(knowledgeops::search::to_tsquery("a b"), ""); // all too short
}

// ---- Import/export: file signature checking ----

#[test]
fn test_check_file_signature_csv_valid() {
    let data = b"title,body\nhello,world\n";
    assert_eq!(
        knowledgeops::import_export::check_file_signature(data, "data.csv").unwrap(),
        "csv"
    );
}

#[test]
fn test_check_file_signature_xlsx_valid() {
    // PK ZIP signature (0x504B)
    let data = [0x50u8, 0x4B, 0x03, 0x04, 0x00, 0x00];
    assert_eq!(
        knowledgeops::import_export::check_file_signature(&data, "data.xlsx").unwrap(),
        "xlsx"
    );
}

#[test]
fn test_check_file_signature_xlsx_invalid_signature() {
    let data = [0x00u8, 0x00, 0x00, 0x00];
    assert!(knowledgeops::import_export::check_file_signature(&data, "data.xlsx").is_err());
}

#[test]
fn test_check_file_signature_unsupported_type() {
    assert!(knowledgeops::import_export::check_file_signature(b"data", "data.pdf").is_err());
    assert!(knowledgeops::import_export::check_file_signature(b"data", "data.exe").is_err());
}

#[test]
fn test_check_file_signature_csv_too_small() {
    assert!(knowledgeops::import_export::check_file_signature(b"x", "data.csv").is_err());
}

// ---- Import/export: title normalization ----

#[test]
fn test_normalize_title_basic() {
    assert_eq!(
        knowledgeops::import_export::normalize_title("  Hello  World  "),
        "hello world"
    );
}

#[test]
fn test_normalize_title_empty() {
    assert_eq!(knowledgeops::import_export::normalize_title(""), "");
    assert_eq!(knowledgeops::import_export::normalize_title("   "), "");
}

// ---- Import/export: sensitive value masking ----

#[test]
fn test_mask_email() {
    let result = knowledgeops::import_export::mask_sensitive_value("Contact: user@example.com");
    assert!(result.contains("***@***.***"));
    assert!(!result.contains("user@example.com"));
}

#[test]
fn test_mask_phone() {
    let result = knowledgeops::import_export::mask_sensitive_value("Call 123-456-7890");
    assert!(result.contains("***-***-****"));
    assert!(!result.contains("123-456-7890"));
}

#[test]
fn test_mask_ssn() {
    let result = knowledgeops::import_export::mask_sensitive_value("SSN: 123-45-6789");
    assert!(result.contains("***-**-****"));
    assert!(!result.contains("123-45-6789"));
}

#[test]
fn test_mask_no_sensitive_data() {
    let input = "This is plain text with no sensitive data";
    assert_eq!(knowledgeops::import_export::mask_sensitive_value(input), input);
}

#[test]
fn test_mask_multiple_patterns() {
    let input = "Email: a@b.com, Phone: 555-123-4567, SSN: 999-88-7777";
    let result = knowledgeops::import_export::mask_sensitive_value(input);
    assert!(!result.contains("a@b.com"));
    assert!(!result.contains("555-123-4567"));
    assert!(!result.contains("999-88-7777"));
}

// ---- Audit: sensitive field redaction ----

#[test]
fn test_audit_redact_event_payload() {
    let payload = serde_json::json!({
        "username": "admin",
        "password": "secret123",
        "token": "abc",
        "nested": {"secret_key": "val"}
    });
    let redacted = knowledgeops::audit::redact_event_payload(&payload);
    assert_eq!(redacted["username"], "admin");
    assert_eq!(redacted["password"], "[REDACTED]");
    assert_eq!(redacted["token"], "[REDACTED]");
    assert_eq!(redacted["nested"]["secret_key"], "[REDACTED]");
}

#[test]
fn test_audit_redact_preserves_non_sensitive() {
    let payload = serde_json::json!({"action": "login", "role": "Admin", "count": 5});
    let redacted = knowledgeops::audit::redact_event_payload(&payload);
    assert_eq!(redacted, payload);
}

// ---- Models: PaginationParams ----

#[test]
fn test_pagination_defaults() {
    let p = knowledgeops::models::PaginationParams { page: None, page_size: None };
    assert_eq!(p.offset(), 0);
    assert_eq!(p.page_size(), 20);
}

#[test]
fn test_pagination_custom() {
    let p = knowledgeops::models::PaginationParams { page: Some(3), page_size: Some(50) };
    assert_eq!(p.offset(), 100); // (3-1) * 50
    assert_eq!(p.page_size(), 50);
}

#[test]
fn test_pagination_clamped() {
    let p = knowledgeops::models::PaginationParams { page: Some(0), page_size: Some(500) };
    assert_eq!(p.offset(), 0); // page 0 treated as 1
    assert_eq!(p.page_size(), 100); // clamped to max 100
}

#[test]
fn test_pagination_negative_page() {
    let p = knowledgeops::models::PaginationParams { page: Some(-5), page_size: Some(10) };
    assert_eq!(p.offset(), 0);
}

// ---- Standardization: z-score ----

#[test]
fn test_z_score_normal_value() {
    let z = knowledgeops::standardization::compute_z_score(5.0, 5.0, 1.0);
    assert!((z - 0.0).abs() < 0.001);
}

#[test]
fn test_z_score_outlier() {
    // value=15, mean=5, stddev=2 => z = (15-5)/2 = 5.0
    let z = knowledgeops::standardization::compute_z_score(15.0, 5.0, 2.0);
    assert!(z.abs() >= 3.0, "z={} should be >= 3", z);
}

#[test]
fn test_z_score_negative_outlier() {
    let z = knowledgeops::standardization::compute_z_score(-10.0, 5.0, 2.0);
    assert!(z.abs() >= 3.0);
    assert!(z < 0.0);
}

#[test]
fn test_z_score_zero_stddev() {
    let z = knowledgeops::standardization::compute_z_score(5.0, 5.0, 0.0);
    assert_eq!(z, 0.0, "Zero stddev should return 0");
}

// ---- Standardization: fingerprint determinism ----

#[test]
fn test_fingerprint_deterministic() {
    let fields = serde_json::json!({"category": "tech", "score": 85});
    let keys = vec!["category".to_string()];
    let fp1 = knowledgeops::standardization::compute_fingerprint_pub("Test Title", &fields, &keys);
    let fp2 = knowledgeops::standardization::compute_fingerprint_pub("Test Title", &fields, &keys);
    assert_eq!(fp1, fp2, "Same input should produce same fingerprint");
}

#[test]
fn test_fingerprint_differs_on_title() {
    let fields = serde_json::json!({"category": "tech"});
    let keys = vec!["category".to_string()];
    let fp1 = knowledgeops::standardization::compute_fingerprint_pub("Title A", &fields, &keys);
    let fp2 = knowledgeops::standardization::compute_fingerprint_pub("Title B", &fields, &keys);
    assert_ne!(fp1, fp2);
}

#[test]
fn test_fingerprint_case_insensitive_title() {
    let fields = serde_json::json!({"category": "tech"});
    let keys = vec!["category".to_string()];
    let fp1 = knowledgeops::standardization::compute_fingerprint_pub("Hello World", &fields, &keys);
    let fp2 = knowledgeops::standardization::compute_fingerprint_pub("hello world", &fields, &keys);
    assert_eq!(fp1, fp2, "Fingerprint should be case-insensitive on title");
}

// ---- Error rate tracking ----

#[test]
fn test_error_rate_stats_returns_tuple() {
    let (total, e4xx, e5xx) = knowledgeops::errors::get_error_rate_stats();
    // Just verify the function returns and the counters are non-negative
    assert!(total >= 0);
    assert!(e4xx >= 0);
    assert!(e5xx >= 0);
}

// ---- Search: tsquery safety ----

#[test]
fn test_tsquery_output_is_safe_chars_only() {
    // The tsquery function should only produce alphanumeric + :*& characters
    let ts = knowledgeops::search::to_tsquery("hello world test");
    assert!(
        ts.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '*' || c == '&' || c == ' '),
        "tsquery output '{}' contains unsafe characters", ts
    );
}

#[test]
fn test_tsquery_strips_single_quotes() {
    let ts = knowledgeops::search::to_tsquery("hello'; DROP TABLE items; --");
    assert!(!ts.contains('\''), "Single quotes should be stripped");
}

#[test]
fn test_tsquery_injection_blocked_by_safety_check() {
    // to_tsquery may pass through some special chars, but the search service
    // validates that output only contains [alphanumeric :*& ] before using it.
    let ts = knowledgeops::search::to_tsquery("hello'; DROP TABLE items; --");
    let is_safe = ts.chars().all(|c| c.is_alphanumeric() || c == ':' || c == '*' || c == '&' || c == ' ');
    assert!(!is_safe, "Injection attempt should fail safety check: '{}'", ts);
}

#[test]
fn test_tsquery_empty_for_short_words() {
    let ts = knowledgeops::search::to_tsquery("a b c");
    assert_eq!(ts, "", "All words < 2 chars should produce empty tsquery");
}

// ---- DB instrumentation: timed_query ----

#[test]
fn test_timed_query_returns_result() {
    let result = knowledgeops::db_instrumentation::timed_query("test_op", 500, || 42);
    assert_eq!(result, 42);
}

#[test]
fn test_timed_query_returns_error_result() {
    let result: Result<i32, &str> =
        knowledgeops::db_instrumentation::timed_query("test_op", 500, || Err("fail"));
    assert!(result.is_err());
}

#[test]
fn test_timed_query_does_not_alter_closure_result() {
    let val = knowledgeops::db_instrumentation::timed_query("noop", 1000, || {
        "hello".to_string()
    });
    assert_eq!(val, "hello");
}

// ---- Analytics filter validation ----

#[test]
fn test_analytics_filter_valid_status() {
    let f = knowledgeops::services::analytics::AnalyticsFilter {
        channel_id: None,
        status: Some("Published".to_string()),
        from: None,
        to: None,
        owner_user_id: None,
    };
    assert!(f.validate().is_ok());
}

#[test]
fn test_analytics_filter_invalid_status() {
    let f = knowledgeops::services::analytics::AnalyticsFilter {
        channel_id: None,
        status: Some("InvalidStatus".to_string()),
        from: None,
        to: None,
        owner_user_id: None,
    };
    assert!(f.validate().is_err());
}

#[test]
fn test_analytics_filter_valid_date_range() {
    let f = knowledgeops::services::analytics::AnalyticsFilter {
        channel_id: None,
        status: None,
        from: Some("2024-01-01".to_string()),
        to: Some("2024-12-31".to_string()),
        owner_user_id: None,
    };
    assert!(f.validate().is_ok());
}

#[test]
fn test_analytics_filter_invalid_from_date() {
    let f = knowledgeops::services::analytics::AnalyticsFilter {
        channel_id: None,
        status: None,
        from: Some("not-a-date".to_string()),
        to: None,
        owner_user_id: None,
    };
    assert!(f.validate().is_err());
}

#[test]
fn test_analytics_filter_empty_is_valid() {
    let f = knowledgeops::services::analytics::AnalyticsFilter::default();
    assert!(f.validate().is_ok());
}

// ---- Import: normalize_title consistency ----

#[test]
fn test_normalize_title_consistency_for_duplicate_check() {
    // Simulates the fix: both sides of the duplicate check use the same normalization
    let stored_title = "  Hello   World  ";
    let import_title = "hello world";
    let norm_stored = knowledgeops::import_export::normalize_title(stored_title);
    let norm_import = knowledgeops::import_export::normalize_title(import_title);
    assert_eq!(norm_stored, norm_import, "Normalized titles should match for duplicate detection");
}

#[test]
fn test_normalize_title_mixed_case_whitespace() {
    assert_eq!(
        knowledgeops::import_export::normalize_title("  My  TITLE   Here  "),
        "my title here"
    );
}

use base64;
use serde_json;
