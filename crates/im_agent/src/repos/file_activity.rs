// Path: crates/im_agent/src/repos/file_activity.rs
// Description: Activity metadata updates for recent file ranking

use chrono::{DateTime, Utc};

use crate::protocol::FileActivity;

const BURST_WINDOW_SECONDS: i64 = 20 * 60;

pub(crate) fn update_activity(
    previous: Option<&FileActivity>,
    observed_at: DateTime<Utc>,
) -> FileActivity {
    let observed_iso = observed_at.to_rfc3339();

    let Some(previous) = previous else {
        return FileActivity {
            first_seen_at_iso: observed_iso.clone(),
            last_seen_at_iso: observed_iso,
            update_count: 1,
            burst_count: 1,
        };
    };

    let previous_last_seen = parse_iso(&previous.last_seen_at_iso);
    let is_same_burst = previous_last_seen
        .map(|last_seen| observed_at.signed_duration_since(last_seen).num_seconds())
        .is_some_and(|seconds| (0..=BURST_WINDOW_SECONDS).contains(&seconds));

    FileActivity {
        first_seen_at_iso: previous.first_seen_at_iso.clone(),
        last_seen_at_iso: observed_iso,
        update_count: previous.update_count.saturating_add(1).max(1),
        burst_count: if is_same_burst {
            previous.burst_count.saturating_add(1).max(2)
        } else {
            1
        },
    }
}

pub(crate) fn activity_from_mtime(mtime: &str) -> FileActivity {
    let observed_at = parse_iso(mtime).unwrap_or_else(Utc::now);
    let observed_iso = observed_at.to_rfc3339();
    FileActivity {
        first_seen_at_iso: observed_iso.clone(),
        last_seen_at_iso: observed_iso,
        update_count: 1,
        burst_count: 1,
    }
}

pub(crate) fn observed_at_from_mtime(mtime: &str) -> DateTime<Utc> {
    parse_iso(mtime).unwrap_or_else(Utc::now)
}

fn parse_iso(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_activity_counts_bursts_inside_window() {
        let first = DateTime::parse_from_rfc3339("2026-05-23T12:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        let second = DateTime::parse_from_rfc3339("2026-05-23T12:05:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);

        let initial = update_activity(None, first);
        let updated = update_activity(Some(&initial), second);

        assert_eq!(updated.first_seen_at_iso, initial.first_seen_at_iso);
        assert_eq!(updated.update_count, 2);
        assert_eq!(updated.burst_count, 2);
    }

    #[test]
    fn update_activity_resets_burst_after_window() {
        let first = DateTime::parse_from_rfc3339("2026-05-23T12:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        let later = DateTime::parse_from_rfc3339("2026-05-23T13:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);

        let initial = update_activity(None, first);
        let updated = update_activity(Some(&initial), later);

        assert_eq!(updated.update_count, 2);
        assert_eq!(updated.burst_count, 1);
    }
}
