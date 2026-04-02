#[cfg(test)]
mod streak_logic_tests {
    use crate::api::handlers::compute_streak;
    use chrono::{Duration, Utc};
    use chrono_tz::Asia::Kolkata;

    fn today() -> String {
        Utc::now().with_timezone(&Kolkata).date_naive().to_string()
    }

    fn days_ago(n: i64) -> String {
        (Utc::now().with_timezone(&Kolkata).date_naive() - Duration::days(n)).to_string()
    }

    // ── No prior checkin ──────────────────────────────────────────────────────

    #[test]
    fn new_user_gets_streak_of_one() {
        let (streak, date) = compute_streak(None, 0);
        assert_eq!(streak, "1");
        assert_eq!(date, today());
    }

    #[test]
    fn new_user_ignores_nonzero_current_value() {
        // current is irrelevant when there's no prior date — should still reset to 1
        let (streak, date) = compute_streak(None, 99);
        assert_eq!(streak, "1");
        assert_eq!(date, today());
    }

    // ── Already checked in today ──────────────────────────────────────────────

    #[test]
    fn same_day_checkin_is_idempotent() {
        let (streak, date) = compute_streak(Some(&today()), 5);
        assert_eq!(
            streak, "5",
            "Streak must not change on a same-day re-checkin"
        );
        assert_eq!(date, today());
    }

    #[test]
    fn same_day_checkin_with_streak_one_stays_at_one() {
        let (streak, _) = compute_streak(Some(&today()), 1);
        assert_eq!(streak, "1");
    }

    // ── Consecutive day ───────────────────────────────────────────────────────

    #[test]
    fn yesterday_checkin_increments_streak() {
        let (streak, date) = compute_streak(Some(&days_ago(1)), 4);
        assert_eq!(
            streak, "5",
            "Consecutive day must increment streak by exactly 1"
        );
        assert_eq!(date, today());
    }

    #[test]
    fn first_consecutive_checkin_goes_from_one_to_two() {
        let (streak, _) = compute_streak(Some(&days_ago(1)), 1);
        assert_eq!(streak, "2");
    }

    #[test]
    fn large_consecutive_streak_increments_correctly() {
        let (streak, _) = compute_streak(Some(&days_ago(1)), 99);
        assert_eq!(streak, "100");
    }

    // ── Streak broken ─────────────────────────────────────────────────────────

    #[test]
    fn two_day_gap_resets_streak_to_one() {
        let (streak, date) = compute_streak(Some(&days_ago(2)), 10);
        assert_eq!(streak, "1", "A 2-day gap must reset the streak to 1");
        assert_eq!(date, today());
    }

    #[test]
    fn thirty_day_gap_resets_streak_to_one() {
        let (streak, _) = compute_streak(Some(&days_ago(30)), 99);
        assert_eq!(streak, "1");
    }

    #[test]
    fn exactly_two_day_gap_is_broken_not_consecutive() {
        let (streak, _) = compute_streak(Some(&days_ago(2)), 7);
        assert_eq!(streak, "1");
    }

    // ── Malformed / corrupted date ────────────────────────────────────────────

    #[test]
    fn malformed_date_does_not_panic_and_resets_streak() {
        // A corrupted date is treated as None -> streak resets to 1.
        let (streak, date) = compute_streak(Some("not-a-date"), 5);
        assert_eq!(streak, "1", "Corrupted date must reset streak safely to 1");
        assert_eq!(date, today());
    }

    #[test]
    fn wrong_date_format_resets_streak() {
        // ISO with time component -- not the expected "%Y-%m-%d"
        let (streak, _) = compute_streak(Some("2025-01-15T10:00:00Z"), 3);
        assert_eq!(streak, "1");
    }

    // ── Zero current streak edge case ─────────────────────────────────────────

    #[test]
    fn zero_streak_yesterday_increments_to_one() {
        // Must not underflow (u64): 0 + 1 = 1
        let (streak, _) = compute_streak(Some(&days_ago(1)), 0);
        assert_eq!(streak, "1");
    }
}

#[cfg(test)]
mod streak_impl_tests {
    use candid::Principal;
    use chrono::{Duration, Utc};
    use chrono_tz::Asia::Kolkata;

    use crate::api::handlers::{checkin_impl, delete_streak_impl, get_streak_impl};
    use crate::api::store::StreakStore;
    use crate::utils::test_helper::db_test_helpers::TestDb;

    fn anon() -> Principal {
        Principal::anonymous()
    }

    fn user_a() -> Principal {
        Principal::from_text("aaaaa-aa").unwrap()
    }

    fn today() -> String {
        Utc::now().with_timezone(&Kolkata).date_naive().to_string()
    }

    fn days_ago(n: i64) -> String {
        (Utc::now().with_timezone(&Kolkata).date_naive() - Duration::days(n)).to_string()
    }

    // ── get_streak_impl ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn get_streak_new_user_returns_all_none() {
        let db = TestDb::new().await;
        let res = get_streak_impl(&db.pool, anon()).await.unwrap();
        assert!(res.current_streak.is_none());
        assert!(res.last_checkin_date.is_none());
    }

    #[tokio::test]
    async fn get_streak_returns_stored_values_verbatim() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "7", "2025-01-01").await.unwrap();

        let res = get_streak_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.current_streak.as_deref(), Some("7"));
        assert_eq!(res.last_checkin_date.as_deref(), Some("2025-01-01"));
    }

    #[tokio::test]
    async fn get_streak_does_not_mutate_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "3", "2025-01-01").await.unwrap();

        get_streak_impl(&db.pool, anon()).await.unwrap();
        let raw = db.pool.get_streak(&k).await;
        assert_eq!(
            raw.unwrap().unwrap().current_streak,
            Some("3".to_string()),
            "get_streak must be a pure read -- store must be unchanged"
        );
    }

    // ── checkin_impl ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn first_checkin_returns_streak_one_and_todays_date() {
        let db = TestDb::new().await;
        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.current_streak.as_deref(), Some("1"));
        assert_eq!(res.last_checkin_date.as_deref(), Some(today().as_str()));
    }

    #[tokio::test]
    async fn checkin_persists_updated_streak_to_store() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        let yesterday = days_ago(1);

        db.pool.set_streak(&k, "3", &yesterday).await.unwrap();
        let res = checkin_impl(&db.pool, anon()).await.unwrap();

        // Returned value must already be correct
        assert_eq!(res.current_streak.as_deref(), Some("4"));
        assert_eq!(res.last_checkin_date.as_deref(), Some(today().as_str()));

        let streak_data = db.pool.get_streak(&k).await.unwrap().unwrap();

        assert_eq!(
            streak_data.current_streak.as_deref(),
            Some("4"),
            "BUG: current_streak not written back to store after checkin"
        );
        assert_eq!(
            streak_data.last_checkin_date.as_deref(),
            Some(today().as_str()),
            "BUG: last_checkin_date not written back to store after checkin"
        );
    }

    #[tokio::test]
    async fn double_checkin_same_day_is_idempotent() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "5", today().as_str()).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(
            res.current_streak.as_deref(),
            Some("5"),
            "Same-day double checkin must not increment streak"
        );
    }

    #[tokio::test]
    async fn checkin_consecutive_day_increments_streak() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "9", &days_ago(1)).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(res.current_streak.as_deref(), Some("10"));
    }

    #[tokio::test]
    async fn checkin_after_gap_resets_streak_to_one() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "20", &days_ago(2)).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(
            res.current_streak.as_deref(),
            Some("1"),
            "Broken streak must reset to 1 regardless of prior value"
        );
    }

    // ── delete_streak_impl ────────────────────────────────────────────────────

    #[tokio::test]
    async fn delete_streak_removes_key_from_store() {
        let db = TestDb::new().await;
        db.pool
            .set_streak(&anon().to_text(), "5", "2025-01-01")
            .await
            .unwrap();

        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let res = get_streak_impl(&db.pool, anon()).await.unwrap();
        assert!(res.current_streak.is_none());
        assert!(res.last_checkin_date.is_none());
    }

    #[tokio::test]
    async fn delete_nonexistent_streak_is_ok() {
        let db = TestDb::new().await;
        let res = delete_streak_impl(&db.pool, anon()).await;
        assert!(res.is_ok(), "Deleting a non-existent key must not error");
    }

    #[tokio::test]
    async fn get_after_delete_returns_all_none() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "3", "2025-01-01").await.unwrap();
        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let res = get_streak_impl(&db.pool, anon()).await.unwrap();
        assert!(res.current_streak.is_none());
        assert!(res.last_checkin_date.is_none());
    }

    #[tokio::test]
    async fn checkin_after_delete_restarts_streak_at_one() {
        let db = TestDb::new().await;
        let k = anon().to_text();
        db.pool.set_streak(&k, "10", &days_ago(1)).await.unwrap();
        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let res = checkin_impl(&db.pool, anon()).await.unwrap();
        assert_eq!(
            res.current_streak.as_deref(),
            Some("1"),
            "After delete, a fresh checkin must start at 1"
        );
    }

    // ── Key isolation between users ───────────────────────────────────────────

    #[tokio::test]
    async fn user_streaks_are_fully_isolated() {
        let db = TestDb::new().await;
        let k_a = user_a().to_text();

        db.pool.set_streak(&k_a, "5", &days_ago(1)).await.unwrap();
        let res_anon = checkin_impl(&db.pool, anon()).await.unwrap();
        let res_a = get_streak_impl(&db.pool, user_a()).await.unwrap();

        assert_eq!(
            res_anon.current_streak.as_deref(),
            Some("1"),
            "Fresh user must start at 1"
        );
        assert_eq!(
            res_a.current_streak.as_deref(),
            Some("5"),
            "user_a streak must be unaffected by anon checkin"
        );
    }

    #[tokio::test]
    async fn delete_only_affects_target_user() {
        let db = TestDb::new().await;
        db.pool
            .set_streak(&user_a().to_text(), "7", "2025-01-01")
            .await
            .unwrap();
        db.pool
            .set_streak(&anon().to_text(), "3", "2025-01-01")
            .await
            .unwrap();

        delete_streak_impl(&db.pool, anon()).await.unwrap();

        let res_a = get_streak_impl(&db.pool, user_a()).await.unwrap();
        let res_anon = get_streak_impl(&db.pool, anon()).await.unwrap();

        assert_eq!(
            res_a.current_streak.as_deref(),
            Some("7"),
            "user_a must be untouched"
        );
        assert!(res_anon.current_streak.is_none(), "anon must be deleted");
    }
}
