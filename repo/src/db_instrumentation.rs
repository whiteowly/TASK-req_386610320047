/// Lightweight DB query timing instrumentation.
/// Logs a warning when a DB operation exceeds the configured threshold.

use std::time::Instant;

/// Execute a closure and log a warning if it exceeds `threshold_ms`.
/// Returns the closure's result unchanged.
pub fn timed_query<F, T>(operation: &str, threshold_ms: u64, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = f();
    let elapsed_ms = start.elapsed().as_millis() as u64;
    if elapsed_ms >= threshold_ms {
        log::warn!(
            "Slow DB query: operation={}, elapsed_ms={}, threshold_ms={}",
            operation,
            elapsed_ms,
            threshold_ms
        );
    }
    result
}
