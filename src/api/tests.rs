#[cfg(test)]
mod streak_logic_tests {
    use crate::types::compute_streak;

    fn now_ms() -> i64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn ms_ago(hours: i64) -> i64 {
        now_ms() - hours * 60 * 60 * 1000
    }

    // ── No prior checkin ──────────────────────────────────────────────────────

    #[test]
    fn new_user_gets_streak_of_one() {
        let now = now_ms();
        let (streak, just_incremented, action, last_credited) = compute_streak(None, 0, now);
        assert_eq!(streak, 1);
        assert!(just_incremented);
        assert_eq!(action, "incremented");
        assert_eq!(last_credited, now);
    }

    #[test]
    fn new_user_ignores_nonzero_current_value() {
        let now = now_ms();
        let (streak, just_incremented, action, _) = compute_streak(None, 99, now);
        assert_eq!(
            streak, 1,
            "New user must always start at 1 regardless of passed current"
        );
        assert!(just_incremented);
        assert_eq!(action, "incremented");
    }

    // ── Already checked in within 24h ─────────────────────────────────────────

    #[test]
    fn checkin_within_24h_is_idempotent() {
        let now = now_ms();
        let last = ms_ago(12); // 12 hours ago
        let (streak, just_incremented, action, last_credited) = compute_streak(Some(last), 5, now);
        assert_eq!(streak, 5, "Streak must not change within 24h window");
        assert!(!just_incremented);
        assert_eq!(action, "unchanged");
        assert_eq!(
            last_credited, last,
            "last_credited must not update when unchanged"
        );
    }

    #[test]
    fn checkin_just_under_24h_is_unchanged() {
        let now = now_ms();
        let last = ms_ago(23); // 23 hours ago — still within window
        let (streak, _, action, _) = compute_streak(Some(last), 3, now);
        assert_eq!(streak, 3);
        assert_eq!(action, "unchanged");
    }

    // ── Consecutive checkin (24h–48h window) ──────────────────────────────────

    #[test]
    fn checkin_after_24h_increments_streak() {
        let now = now_ms();
        let last = ms_ago(25);
        let (streak, just_incremented, action, last_credited) = compute_streak(Some(last), 4, now);
        assert_eq!(streak, 5, "Checkin in 24-48h window must increment by 1");
        assert!(just_incremented);
        assert_eq!(action, "incremented");
        assert_eq!(
            last_credited, now,
            "last_credited must update to now on increment"
        );
    }

    #[test]
    fn checkin_at_exactly_24h_increments_streak() {
        let now = now_ms();
        let last = now - 24 * 60 * 60 * 1000;
        let (streak, just_incremented, action, _) = compute_streak(Some(last), 1, now);
        assert_eq!(streak, 2);
        assert!(just_incremented);
        assert_eq!(action, "incremented");
    }

    #[test]
    fn checkin_just_under_48h_still_increments() {
        let now = now_ms();
        let last = now - (48 * 60 * 60 * 1000 - 1); // 1ms before 48h boundary
        let (streak, just_incremented, action, _) = compute_streak(Some(last), 9, now);
        assert_eq!(streak, 10);
        assert!(just_incremented);
        assert_eq!(action, "incremented");
    }

    #[test]
    fn first_consecutive_checkin_goes_from_one_to_two() {
        let now = now_ms();
        let (streak, _, _, _) = compute_streak(Some(ms_ago(25)), 1, now);
        assert_eq!(streak, 2);
    }

    #[test]
    fn large_consecutive_streak_increments_correctly() {
        let now = now_ms();
        let (streak, _, _, _) = compute_streak(Some(ms_ago(25)), 99, now);
        assert_eq!(streak, 100);
    }

    // ── Streak broken (48h+) ──────────────────────────────────────────────────

    #[test]
    fn checkin_after_48h_resets_streak_to_one() {
        let now = now_ms();
        let last = ms_ago(49);
        let (streak, just_incremented, action, last_credited) = compute_streak(Some(last), 10, now);
        assert_eq!(streak, 1, "48h+ gap must reset streak to 1");
        assert!(just_incremented, "reset counts as just_incremented");
        assert_eq!(action, "reset");
        assert_eq!(
            last_credited, now,
            "last_credited must update to now on reset"
        );
    }

    #[test]
    fn checkin_at_exactly_48h_resets_streak() {
        let now = now_ms();
        let last = now - 48 * 60 * 60 * 1000;
        let (streak, _, action, _) = compute_streak(Some(last), 7, now);
        assert_eq!(streak, 1);
        assert_eq!(action, "reset");
    }

    #[test]
    fn week_old_checkin_resets_streak_to_one() {
        let now = now_ms();
        let (streak, _, action, _) = compute_streak(Some(ms_ago(7 * 24)), 99, now);
        assert_eq!(streak, 1);
        assert_eq!(action, "reset");
    }

    // ── Zero streak edge case ─────────────────────────────────────────────────

    #[test]
    fn zero_streak_in_window_increments_to_one() {
        let now = now_ms();
        let (streak, _, _, _) = compute_streak(Some(ms_ago(25)), 0, now);
        assert_eq!(streak, 1, "0 + 1 must not underflow");
    }
}

#[cfg(test)]
mod streak_impl_tests {
    use candid::Principal;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::api::handlers::{checkin_impl, delete_streak_impl};
    use crate::api::store::StreakStore;
    use crate::utils::test_helper::db_test_helpers::TestDb;

    fn anon() -> Principal {
        Principal::anonymous()
    }

    fn user_a() -> Principal {
        Principal::from_text("aaaaa-aa").unwrap()
    }

    fn now_ms() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    }

    fn ms_ago(hours: i64) -> i64 {
        now_ms() - hours * 60 * 60 * 1000
    }

    // ── checkin_impl ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn first_checkin_returns_streak_one() {
        let db = TestDb::new().await;
        let res = checkin_impl(&db.pool, anon()).await.unwrap();

        assert_eq!(res.streak_count, 1);
        assert!(res.just_incremented);
        assert_eq!(res.streak_action, "incremented");
        assert!(res.last_credited_at_epoch_ms > 0);
        assert_eq!(
            res.next_increment_eligible_at_epoch_ms,
            res.last_credited_at_epoch_ms + 24 * 60 * 60 * 1000
        );
        assert_eq!(
            res.streak_expires_at_epoch_ms,
            res.last_credited_at_epoch_ms + 48 * 60 * 60 * 1000
        );
    }

    #[tokio::test]
    async fn first_checkin_persists_to_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        checkin_impl(&db.pool, anon()).await.unwrap();

        let row = db.pool.get_streak(&k).await.unwrap().unwrap();
        assert_eq!(row.current_streak, 1);
        assert!(row.last_checkin_epoch_ms > 0);
    }

    #[tokio::test]
    async fn double_checkin_within_24h_is_idempotent() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        // Seed: checked in 12 hours ago with streak 5
        db.pool.set_streak(&k, 5, ms_ago(12)).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.streak_count, 5, "Streak must not change within 24h");
        assert!(!res.just_incremented);
        assert_eq!(res.streak_action, "unchanged");
    }

    #[tokio::test]
    async fn unchanged_checkin_does_not_mutate_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        let seeded_at = ms_ago(12);

        db.pool.set_streak(&k, 5, seeded_at).await.unwrap();
        checkin_impl(&db.pool, anon()).await.unwrap();

        let row = db.pool.get_streak(&k).await.unwrap().unwrap();
        assert_eq!(
            row.last_checkin_epoch_ms, seeded_at,
            "Store must not be mutated on unchanged checkin"
        );
    }

    #[tokio::test]
    async fn checkin_in_window_increments_streak() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 9, ms_ago(25)).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.streak_count, 10);
        assert!(res.just_incremented);
        assert_eq!(res.streak_action, "incremented");
    }

    #[tokio::test]
    async fn checkin_in_window_persists_updated_streak() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 3, ms_ago(25)).await.unwrap();
        checkin_impl(&db.pool, anon()).await.unwrap();

        let row = db.pool.get_streak(&k).await.unwrap().unwrap();
        assert_eq!(row.current_streak, 4, "Updated streak must be written back");
        assert!(
            row.last_checkin_epoch_ms > ms_ago(1),
            "last_checkin_epoch_ms must be updated to approximately now"
        );
    }

    #[tokio::test]
    async fn checkin_after_48h_resets_streak_to_one() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 20, ms_ago(49)).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.streak_count, 1, "Broken streak must reset to 1");
        assert!(res.just_incremented, "reset counts as just_incremented");
        assert_eq!(res.streak_action, "reset");
    }

    #[tokio::test]
    async fn checkin_reset_persists_to_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 20, ms_ago(49)).await.unwrap();
        checkin_impl(&db.pool, anon()).await.unwrap();

        let row = db.pool.get_streak(&k).await.unwrap().unwrap();
        assert_eq!(row.current_streak, 1);
    }

    #[tokio::test]
    async fn response_timestamps_are_consistent() {
        let db = TestDb::new().await;
        let before = now_ms();
        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        let after = now_ms();

        assert!(
            res.server_now_epoch_ms >= before && res.server_now_epoch_ms <= after,
            "server_now_epoch_ms must be within the request window"
        );
        assert_eq!(
            res.next_increment_eligible_at_epoch_ms,
            res.last_credited_at_epoch_ms + 24 * 60 * 60 * 1000
        );
        assert_eq!(
            res.streak_expires_at_epoch_ms,
            res.last_credited_at_epoch_ms + 48 * 60 * 60 * 1000
        );
    }

    // ── delete_streak_impl ────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_streak_removes_record_from_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 5, ms_ago(1)).await.unwrap();
        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let row = db.pool.get_streak(&k).await.unwrap();
        assert!(row.is_none(), "Record must be gone after delete");
    }

    #[tokio::test]
    async fn delete_nonexistent_streak_is_ok() {
        let db = TestDb::new().await;
        let res = delete_streak_impl(&db.pool, anon()).await;
        assert!(res.is_ok(), "Deleting a non-existent key must not error");
    }

    #[tokio::test]
    async fn checkin_after_delete_restarts_streak_at_one() {
        let db = TestDb::new().await;
        let k = anon().to_text();

        db.pool.set_streak(&k, 10, ms_ago(25)).await.unwrap();
        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(
            res.streak_count, 1,
            "After delete, fresh checkin must start at 1"
        );
        assert!(res.just_incremented);
        assert_eq!(res.streak_action, "incremented");
    }

    // ── Key isolation between users ───────────────────────────────────────────

    #[tokio::test]
    async fn user_streaks_are_fully_isolated() {
        let db = TestDb::new().await;

        db.pool
            .set_streak(&user_a().to_text(), 5, ms_ago(25))
            .await
            .unwrap();

        let res_anon = checkin_impl(&db.pool, anon()).await.unwrap();
        let row_a = db
            .pool
            .get_streak(&user_a().to_text())
            .await
            .unwrap()
            .unwrap();

        assert_eq!(res_anon.streak_count, 1, "Fresh user must start at 1");
        assert_eq!(row_a.current_streak, 5, "user_a streak must be unaffected");
    }

    #[tokio::test]
    async fn delete_only_affects_target_user() {
        let db = TestDb::new().await;

        db.pool
            .set_streak(&user_a().to_text(), 7, ms_ago(1))
            .await
            .unwrap();
        db.pool
            .set_streak(&anon().to_text(), 3, ms_ago(1))
            .await
            .unwrap();

        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let row_a = db.pool.get_streak(&user_a().to_text()).await.unwrap();
        let row_anon = db.pool.get_streak(&anon().to_text()).await.unwrap();

        assert!(row_a.is_some(), "user_a must be untouched");
        assert_eq!(row_a.unwrap().current_streak, 7);
        assert!(row_anon.is_none(), "anon must be deleted");
    }
}
