use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{http::error::ApiError, AppState};

#[derive(Deserialize)]
pub struct StripeWebhookPayload {
    pub r#type: String,
    pub data: WebhookData,
}

#[derive(Deserialize)]
pub struct WebhookData {
    pub object: StripeObject,
}

#[derive(Deserialize)]
pub struct StripeObject {
    pub metadata: PaymentMetadata,
}

#[derive(Deserialize)]
pub struct PaymentMetadata {
    pub booking_id: Uuid,
}

pub fn router(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/stripe", post(stripe_webhook))
}

// Rule #49: Idempotent Webhook
// Rule #52: BOOKING CONFIRMATION Short PostgreSQL Transaction
async fn stripe_webhook(
    State(state): State<AppState>,
    Json(payload): Json<StripeWebhookPayload>,
) -> Result<Json<Value>, ApiError> {
    if payload.r#type == "payment_intent.succeeded" {
        let booking_id = payload.data.object.metadata.booking_id;
        tracing::info!("Payment succeeded for booking_id: {}", booking_id);
        
        let mut tx = state.db.begin().await?;

        // 1. Verify booking
        let booking = sqlx::query!(
            r#"
            SELECT b.id, b.user_id, b.showtime_id, b.status::text as "booking_status!",
                   h.id as hold_id, h.status::text as "hold_status!", h.expires_at
            FROM bookings b
            JOIN holds h ON h.user_id = b.user_id AND h.showtime_id = b.showtime_id
            WHERE b.id = $1 FOR UPDATE
            "#,
            booking_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::InvalidRequest("Booking not found".into()))?;

        if booking.booking_status == "confirmed" {
            // Idempotency: Already processed
            return Ok(Json(json!({ "received": true, "status": "already_processed" })));
        }

        if booking.booking_status != "pending_payment" {
            return Err(ApiError::InvalidRequest("Booking is not pending payment".into()));
        }

        if booking.hold_status != "active" {
            return Err(ApiError::InvalidRequest("Underlying hold is not active".into()));
        }

        if booking.expires_at < chrono::Utc::now() {
            return Err(ApiError::HoldExpired);
        }

        // 2. Update Booking to confirmed
        sqlx::query!(
            r#"
            UPDATE bookings SET status = 'confirmed' WHERE id = $1
            "#,
            booking_id
        )
        .execute(&mut *tx)
        .await?;

        // 3. Create Booking Seats by copying from hold_items
        sqlx::query!(
            r#"
            INSERT INTO booking_seats (booking_id, showtime_id, seat_id)
            SELECT $1, showtime_id, seat_id FROM hold_items WHERE hold_id = $2
            "#,
            booking_id,
            booking.hold_id
        )
        .execute(&mut *tx)
        .await?;

        // 4. Complete Hold
        sqlx::query!(
            r#"
            UPDATE holds SET status = 'completed' WHERE id = $1
            "#,
            booking.hold_id
        )
        .execute(&mut *tx)
        .await?;

        // 5. Create Outbox Event (Rule #54)
        sqlx::query!(
            r#"
            INSERT INTO outbox_events (aggregate_type, aggregate_id, event_type, payload)
            VALUES ('booking', $1, 'BookingConfirmed', '{}')
            "#,
            booking_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
    }

    Ok(Json(json!({ "received": true })))
}
