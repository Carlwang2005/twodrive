use super::Database;
use crate::{DeleteFailureKind, now_unix};
use rusqlite::{OptionalExtension, TransactionBehavior, params};

impl Database {
    /// Keep the tombstone/cache/barriers; only change its durable retry schedule.
    pub fn defer_pending_delete(
        &self,
        id: &str,
        status: Option<u16>,
        kind: DeleteFailureKind,
        retry_after_seconds: u64,
    ) -> anyhow::Result<()> {
        self.defer_pending_delete_at(id, status, kind, retry_after_seconds, now_unix())
    }
    pub(crate) fn defer_pending_delete_at(
        &self,
        id: &str,
        status: Option<u16>,
        kind: DeleteFailureKind,
        retry_after_seconds: u64,
        now: i64,
    ) -> anyhow::Result<()> {
        let mut conn = self.connect()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: Option<u32> = tx
            .query_row(
                "SELECT attempt_count FROM pending_deletes WHERE remote_id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(count) = count {
            let attempts = count.saturating_add(1);
            let (base, cap) = match kind {
                DeleteFailureKind::Authentication
                | DeleteFailureKind::Permission
                | DeleteFailureKind::Rejected => (300_u64, 21600_u64),
                _ => (60_u64, 3600_u64),
            };
            let delay = base
                .saturating_mul(1_u64 << attempts.saturating_sub(1).min(16))
                .min(cap)
                .max(retry_after_seconds);
            let next = now.saturating_add(i64::try_from(delay).unwrap_or(i64::MAX));
            tx.execute("UPDATE pending_deletes SET attempt_count = ?2, next_retry_unix = ?3, last_status = ?4, failure_kind = ?5 WHERE remote_id = ?1",
                params![id, attempts, next, status, kind.as_str()])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A deliberate user action makes retained tasks eligible on the next scan.
    /// Preserve attempt history and failure classification until a successful retry.
    pub fn retry_pending_deletes(&self) -> anyhow::Result<usize> {
        Ok(self.connect()?.execute(
            "UPDATE pending_deletes SET next_retry_unix = 0 WHERE next_retry_unix != 0",
            [],
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_delete_rows_gain_due_defaults_without_changing_cloud_identity() {
        let root = std::env::temp_dir().join(format!(
            "td-old-delete-retry-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::new(root.join("db"));
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute_batch("CREATE TABLE pending_deletes(remote_id TEXT PRIMARY KEY, path TEXT NOT NULL, cache_path TEXT, queued_unix INTEGER NOT NULL); INSERT INTO pending_deletes VALUES ('legacy-cloud-id', '/legacy', NULL, 17);").unwrap();
        drop(conn);
        db.init().unwrap();
        db.init().unwrap();
        let entry = db.pending_deletes().unwrap().remove(0);
        assert_eq!(entry.remote_id, "legacy-cloud-id");
        assert_eq!(entry.path, "/legacy");
        assert_eq!(entry.queued_unix, 17);
        assert_eq!(entry.attempt_count, 0);
        assert_eq!(entry.next_retry_unix, 0);
        assert_eq!(entry.last_status, None);
        assert_eq!(entry.failure_kind, None);
        assert!(entry.retry_due(now_unix()));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn delete_backoff_survives_migration_and_manual_retry_keeps_identity() {
        let root = std::env::temp_dir().join(format!(
            "td-delete-backoff-{}-{}",
            std::process::id(),
            now_unix()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let db = Database::new(root.join("db"));
        db.init().unwrap();
        db.queue_remote_delete("old-cloud-id", "/same-path")
            .unwrap();
        db.defer_pending_delete_at(
            "old-cloud-id",
            Some(403),
            DeleteFailureKind::Permission,
            0,
            1000,
        )
        .unwrap();
        let first = db.pending_deletes().unwrap().remove(0);
        assert_eq!(
            (
                first.attempt_count,
                first.next_retry_unix,
                first.last_status
            ),
            (1, 1300, Some(403))
        );
        let restarted = Database::new(db.path());
        restarted.init().unwrap();
        restarted.init().unwrap();
        assert_eq!(
            restarted.pending_deletes().unwrap()[0].next_retry_unix,
            1300
        );
        assert!(
            restarted
                .is_pending_delete("new-local-id", "/same-path")
                .unwrap()
        );
        restarted
            .defer_pending_delete_at(
                "old-cloud-id",
                Some(403),
                DeleteFailureKind::Permission,
                0,
                1300,
            )
            .unwrap();
        assert_eq!(
            restarted.pending_deletes().unwrap()[0].next_retry_unix,
            1900
        );
        for _ in 0..30 {
            restarted
                .defer_pending_delete_at(
                    "old-cloud-id",
                    Some(403),
                    DeleteFailureKind::Permission,
                    0,
                    2000,
                )
                .unwrap();
        }
        assert_eq!(
            restarted.pending_deletes().unwrap()[0].next_retry_unix,
            23600
        );
        restarted
            .defer_pending_delete_at(
                "old-cloud-id",
                Some(429),
                DeleteFailureKind::Throttled,
                86400,
                3000,
            )
            .unwrap();
        assert_eq!(
            restarted.pending_deletes().unwrap()[0].next_retry_unix,
            89400
        );
        assert_eq!(restarted.retry_pending_deletes().unwrap(), 1);
        let ready = restarted.pending_deletes().unwrap().remove(0);
        assert!(ready.retry_due(3000));
        assert_eq!(ready.remote_id, "old-cloud-id");
        assert_eq!(ready.path, "/same-path");
        assert!(ready.attempt_count > 0);
        assert_eq!(ready.failure_kind, Some(DeleteFailureKind::Throttled));
        std::fs::remove_dir_all(root).unwrap();
    }
}
