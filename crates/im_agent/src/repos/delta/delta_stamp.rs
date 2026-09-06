// Path: crates/im_agent/src/repos/delta/delta_stamp.rs
// Description: RFC 3339 mtime stamps for one delta - now, a SystemTime, or the milliseconds a settled read reported

use std::time::{SystemTime, UNIX_EPOCH};

/// The stamp for a payload whose own mtime is unknown or moot: a deletion, an
/// unreadable path, or a clock the filesystem would not answer for.
pub(super) fn now_stamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// The stamp a `metadata` call answered with (the image arm).
pub(super) fn stamp_of(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()
}

/// The same `metadata` answer as the epoch milliseconds the wire carries, so
/// an image payload and a later `readImageFile` can be matched byte for byte.
/// A clock before the epoch or one no `u64` can hold reads as zero.
pub(super) fn ms_of(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| u64::try_from(since.as_millis()).ok())
        .unwrap_or(0)
}

/// The stamp the settled read proved the file held across the read. A value no
/// calendar can hold falls back to now rather than inventing a date.
pub(super) fn stamp_from_ms(mtime_ms: u64) -> String {
    i64::try_from(mtime_ms)
        .ok()
        .and_then(chrono::DateTime::<chrono::Utc>::from_timestamp_millis)
        .map_or_else(now_stamp, |time| time.to_rfc3339())
}
