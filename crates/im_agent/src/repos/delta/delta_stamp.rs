// Path: crates/im_agent/src/repos/delta/delta_stamp.rs
// Description: RFC 3339 mtime stamps for one delta - now, a SystemTime, or the milliseconds a settled read reported

use std::time::SystemTime;

/// The stamp for a payload whose own mtime is unknown or moot: a deletion, an
/// unreadable path, or a clock the filesystem would not answer for.
pub(super) fn now_stamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// The stamp a `metadata` call answered with (the image arm).
pub(super) fn stamp_of(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()
}

/// The stamp the settled read proved the file held across the read. A value no
/// calendar can hold falls back to now rather than inventing a date.
pub(super) fn stamp_from_ms(mtime_ms: u64) -> String {
    i64::try_from(mtime_ms)
        .ok()
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map_or_else(now_stamp, |time| time.to_rfc3339())
}
