use std::fs;
use std::io::Write;
use uuid::Uuid;
use chrono::Utc;

pub fn write_alert(spool_dir: &str, alert_type: &str, message: &str, details: &serde_json::Value) {
    let now = Utc::now();
    let alert_id = Uuid::new_v4();
    let filename = format!("{}_{}.json", now.format("%Y%m%d_%H%M%S"), alert_id);
    let tmp_path = format!("{}/{}.tmp", spool_dir, alert_id);
    let final_path = format!("{}/{}", spool_dir, filename);

    let alert = serde_json::json!({
        "id": alert_id.to_string(),
        "type": alert_type,
        "message": message,
        "details": details,
        "timestamp": now.to_rfc3339(),
    });

    // Ensure spool directory exists
    if let Err(e) = fs::create_dir_all(spool_dir) {
        log::error!("Failed to create alerts spool directory: {}", e);
        return;
    }

    // Atomic write: write to tmp, fsync, rename
    match fs::File::create(&tmp_path) {
        Ok(mut file) => {
            if let Err(e) = file.write_all(
                serde_json::to_string_pretty(&alert)
                    .unwrap_or_default()
                    .as_bytes(),
            ) {
                log::error!("Failed to write alert spool file: {}", e);
                let _ = fs::remove_file(&tmp_path);
                return;
            }
            if let Err(e) = file.sync_all() {
                log::error!("Failed to fsync alert spool file: {}", e);
            }
            if let Err(e) = fs::rename(&tmp_path, &final_path) {
                log::error!("Failed to rename alert spool file: {}", e);
                let _ = fs::remove_file(&tmp_path);
            }
        }
        Err(e) => {
            log::error!("Failed to create alert spool temp file: {}", e);
        }
    }
}

pub fn list_alerts(spool_dir: &str) -> Vec<serde_json::Value> {
    let mut alerts = Vec::new();
    if let Ok(entries) = fs::read_dir(spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(alert) = serde_json::from_str::<serde_json::Value>(&content) {
                        alerts.push(alert);
                    }
                }
            }
        }
    }
    alerts.sort_by(|a, b| {
        let ta = a.get("timestamp").and_then(|t| t.as_str()).unwrap_or("");
        let tb = b.get("timestamp").and_then(|t| t.as_str()).unwrap_or("");
        tb.cmp(ta)
    });
    alerts
}

pub fn ack_alert(spool_dir: &str, alert_id: &str) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains(alert_id) {
                    let ack_path = path.with_extension("acked.json");
                    fs::rename(&path, &ack_path)
                        .map_err(|e| format!("Failed to ack: {}", e))?;
                    return Ok(());
                }
            }
        }
    }
    Err("Alert not found".to_string())
}
