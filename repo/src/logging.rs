use crate::config::AppConfig;

pub fn init(_config: &AppConfig) {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format(|buf, record| {
            use std::io::Write;
            writeln!(
                buf,
                "{{\"timestamp\":\"{}\",\"level\":\"{}\",\"target\":\"{}\",\"message\":\"{}\"}}",
                chrono::Utc::now().to_rfc3339(),
                record.level(),
                record.target(),
                record.args().to_string().replace('\"', "\\\"")
            )
        })
        .init();
}
