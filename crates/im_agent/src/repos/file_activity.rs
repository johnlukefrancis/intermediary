// Path: crates/im_agent/src/repos/file_activity.rs
// Description: Activity metadata updates for recent file ranking

use chrono::{DateTime, Duration, Timelike, Utc};

use crate::protocol::{FileActivity, FileActivityBucket};

const BURST_WINDOW_SECONDS: i64 = 20 * 60;
const HISTORY_WINDOW_HOURS: i64 = 24;

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
            history: vec![new_bucket(observed_at, 1)],
        };
    };

    let previous_last_seen = parse_iso(&previous.last_seen_at_iso);
    let is_same_burst = previous_last_seen
        .map(|last_seen| observed_at.signed_duration_since(last_seen).num_seconds())
        .is_some_and(|seconds| (0..=BURST_WINDOW_SECONDS).contains(&seconds));

    FileActivity {
        first_seen_at_iso: previous.first_seen_at_iso.clone(),
        last_seen_at_iso: observed_iso.clone(),
        update_count: previous.update_count.saturating_add(1).max(1),
        burst_count: if is_same_burst {
            previous.burst_count.saturating_add(1).max(2)
        } else {
            1
        },
        history: record_history(&previous.history, observed_at),
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
        history: vec![new_bucket(observed_at, 1)],
    }
}

pub(crate) fn normalize_activity_history(
    activity: &mut FileActivity,
    fallback_mtime: &str,
) -> bool {
    if !activity.history.is_empty() {
        let last_seen = parse_iso(&activity.last_seen_at_iso).unwrap_or_else(Utc::now);
        let pruned = prune_history(activity.history.clone(), last_seen);
        if pruned == activity.history {
            return false;
        }
        activity.history = pruned;
        return true;
    }

    let observed_at = parse_iso(fallback_mtime)
        .or_else(|| parse_iso(&activity.last_seen_at_iso))
        .unwrap_or_else(Utc::now);
    let count = activity.update_count.max(1);
    activity.history = vec![new_bucket(observed_at, count)];
    true
}

pub(crate) fn observed_at_from_mtime(mtime: &str) -> DateTime<Utc> {
    parse_iso(mtime).unwrap_or_else(Utc::now)
}

fn record_history(
    previous_history: &[FileActivityBucket],
    observed_at: DateTime<Utc>,
) -> Vec<FileActivityBucket> {
    let bucket_start = bucket_start(observed_at).to_rfc3339();
    let mut history = prune_history(previous_history.to_vec(), observed_at);

    if let Some(bucket) = history
        .iter_mut()
        .find(|bucket| bucket.bucket_start_iso == bucket_start)
    {
        bucket.count = bucket.count.saturating_add(1).max(1);
    } else {
        history.push(FileActivityBucket {
            bucket_start_iso: bucket_start,
            count: 1,
        });
    }

    history.sort_by(|a, b| a.bucket_start_iso.cmp(&b.bucket_start_iso));
    history
}

fn prune_history(
    history: Vec<FileActivityBucket>,
    observed_at: DateTime<Utc>,
) -> Vec<FileActivityBucket> {
    let cutoff = bucket_start(observed_at - Duration::hours(HISTORY_WINDOW_HOURS - 1));
    history
        .into_iter()
        .filter(|bucket| {
            parse_iso(&bucket.bucket_start_iso)
                .map(|timestamp| timestamp >= cutoff)
                .unwrap_or(false)
                && bucket.count > 0
        })
        .collect()
}

fn new_bucket(observed_at: DateTime<Utc>, count: u32) -> FileActivityBucket {
    FileActivityBucket {
        bucket_start_iso: bucket_start(observed_at).to_rfc3339(),
        count: count.max(1),
    }
}

fn bucket_start(timestamp: DateTime<Utc>) -> DateTime<Utc> {
    timestamp
        .with_minute(0)
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(timestamp)
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
        assert_eq!(updated.history.len(), 1);
        assert_eq!(updated.history[0].count, 2);
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

    #[test]
    fn update_activity_prunes_history_to_recent_day() {
        let first = DateTime::parse_from_rfc3339("2026-05-22T11:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        let middle = DateTime::parse_from_rfc3339("2026-05-23T10:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);
        let current = DateTime::parse_from_rfc3339("2026-05-23T12:00:00Z")
            .expect("valid timestamp")
            .with_timezone(&Utc);

        let initial = update_activity(None, first);
        let middle_activity = update_activity(Some(&initial), middle);
        let updated = update_activity(Some(&middle_activity), current);

        assert_eq!(updated.history.len(), 2);
        assert_eq!(
            updated.history[0].bucket_start_iso,
            "2026-05-23T10:00:00+00:00"
        );
        assert_eq!(
            updated.history[1].bucket_start_iso,
            "2026-05-23T12:00:00+00:00"
        );
    }

    #[test]
    fn normalize_activity_history_seeds_missing_history() {
        let mut activity = FileActivity {
            first_seen_at_iso: "2026-05-23T12:18:00Z".to_string(),
            last_seen_at_iso: "2026-05-23T12:18:00Z".to_string(),
            update_count: 7,
            burst_count: 2,
            history: Vec::new(),
        };

        let changed = normalize_activity_history(&mut activity, "2026-05-23T12:18:00Z");

        assert!(changed);
        assert_eq!(activity.history.len(), 1);
        assert_eq!(
            activity.history[0].bucket_start_iso,
            "2026-05-23T12:00:00+00:00"
        );
        assert_eq!(activity.history[0].count, 7);
    }
}
