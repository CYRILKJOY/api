use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tokio::task::JoinSet;
use uuid::Uuid;

#[tokio::test]
async fn test_1000_users_same_seat() {
    dotenvy::dotenv().ok();
    
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:54322/postgres".to_string());

    let pool = PgPoolOptions::new()
        .max_connections(100)
        .connect(&db_url)
        .await
        .expect("Failed to connect to db");

    // Fetch a showtime and one available seat
    let showtime = sqlx::query!(
        r#"
        SELECT id FROM showtimes LIMIT 1
        "#
    ).fetch_one(&pool).await.expect("No showtimes found");

    // Ensure there is at least one seat available
    let seat = sqlx::query!(
        r#"
        SELECT seat_id FROM showtime_seats 
        WHERE showtime_id = $1 AND status = 'available' LIMIT 1
        "#,
        showtime.id
    ).fetch_one(&pool).await.expect("No available seats found for test");

    let pool = Arc::new(pool);
    let mut set = JoinSet::new();

    // 1000 users attempting to hold the same exact seat simultaneously
    for _ in 0..1000 {
        let pool = pool.clone();
        let showtime_id = showtime.id;
        let seat_id = seat.seat_id;
        let user_id = Uuid::new_v4();

        set.spawn(async move {
            let record = sqlx::query!(
                r#"
                SELECT create_hold($1, $2, $3, 5) as result
                "#,
                user_id,
                showtime_id,
                &vec![seat_id]
            )
            .fetch_one(&*pool)
            .await;

            match record {
                Ok(r) => {
                    let result_json = r.result.expect("No result");
                    result_json.get("success").and_then(|v| v.as_bool()).unwrap_or(false)
                }
                Err(_) => false, // Pool timeout or other error
            }
        });
    }

    let mut success_count = 0;
    while let Some(res) = set.join_next().await {
        let success = res.expect("Task panicked");
        if success {
            success_count += 1;
        }
    }

    // EXACTLY one valid seat owner expected
    assert_eq!(success_count, 1, "Expected exactly 1 success, but got {}", success_count);
}
