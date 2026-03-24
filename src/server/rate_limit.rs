use serde::Deserialize;
use sha2::{Digest, Sha256};
use worker::D1Type;

use crate::api::ApiError;

use super::AppState;

const WINDOW_MINUTES: usize = 10;
const MAX_REQUESTS_PER_WINDOW: usize = 18;
const DAY_HOURS: usize = 24;
const MAX_REQUESTS_PER_DAY: usize = 25;
const MIN_SECONDS_BETWEEN_REQUESTS: usize = 2;

#[derive(Debug, Deserialize)]
struct CountRow {
    request_count: i64,
}

#[derive(Debug, Deserialize)]
struct SecondsRow {
    seconds_since_last: Option<i64>,
}

pub async fn enforce(state: &AppState, client_ip: &str, route: &str) -> Result<(), ApiError> {
    let Some(db) = state.db() else {
        return Ok(());
    };

    let fingerprint = fingerprint_ip(client_ip, &state.rate_limit_salt());

    let recent_count: usize = db
        .prepare(
            "SELECT COUNT(*) AS request_count
             FROM request_events
             WHERE client_hash = ?1
               AND route = ?2
               AND created_at >= datetime('now', ?3)",
        )
        .bind_refs(&[
            D1Type::Text(fingerprint.as_str()),
            D1Type::Text(route),
            D1Type::Text("-10 minutes"),
        ])
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .first::<CountRow>(None)
        .await
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .map(|row| row.request_count.max(0) as usize)
        .unwrap_or_default();

    if recent_count >= MAX_REQUESTS_PER_WINDOW {
        let _ = record_event(&db, &fingerprint, route, "window_blocked").await;
        return Err(ApiError::rate_limited(format!(
            "Too many runs in the last {WINDOW_MINUTES} minutes. Give it a minute."
        )));
    }

    let daily_count: usize = db
        .prepare(
            "SELECT COUNT(*) AS request_count
             FROM request_events
             WHERE client_hash = ?1
               AND route = ?2
               AND created_at >= datetime('now', ?3)",
        )
        .bind_refs(&[
            D1Type::Text(fingerprint.as_str()),
            D1Type::Text(route),
            D1Type::Text("-24 hours"),
        ])
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .first::<CountRow>(None)
        .await
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .map(|row| row.request_count.max(0) as usize)
        .unwrap_or_default();

    if daily_count >= MAX_REQUESTS_PER_DAY {
        let _ = record_event(&db, &fingerprint, route, "daily_blocked").await;
        return Err(ApiError::rate_limited(format!(
            "Daily cap hit. You get {MAX_REQUESTS_PER_DAY} runs every {DAY_HOURS} hours."
        )));
    }

    let seconds_since_last = db
        .prepare(
            "SELECT CAST((julianday('now') - julianday(MAX(created_at))) * 86400 AS INTEGER)
             AS seconds_since_last
             FROM request_events
             WHERE client_hash = ?1
               AND route = ?2",
        )
        .bind_refs(&[D1Type::Text(fingerprint.as_str()), D1Type::Text(route)])
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .first::<SecondsRow>(None)
        .await
        .map_err(|error| ApiError::internal(d1_error(error)))?
        .and_then(|row| row.seconds_since_last);

    if let Some(seconds) = seconds_since_last {
        if seconds < MIN_SECONDS_BETWEEN_REQUESTS as i64 {
            let _ = record_event(&db, &fingerprint, route, "cooldown_blocked").await;
            return Err(ApiError::rate_limited(
                "Slow down. Rapid-fire regenerate spam is disabled.",
            ));
        }
    }

    record_event(&db, &fingerprint, route, "accepted")
        .await
        .map_err(|error| ApiError::internal(error))?;

    Ok(())
}

pub fn fingerprint_ip(ip: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(ip.trim().as_bytes());
    hex::encode(hasher.finalize())
}

async fn record_event(
    db: &worker::D1Database,
    fingerprint: &str,
    route: &str,
    outcome: &str,
) -> Result<(), String> {
    db.prepare(
        "INSERT INTO request_events (client_hash, route, outcome)
         VALUES (?1, ?2, ?3)",
    )
    .bind_refs(&[
        D1Type::Text(fingerprint),
        D1Type::Text(route),
        D1Type::Text(outcome),
    ])
    .map_err(d1_error)?
    .run()
    .await
    .map_err(d1_error)?;

    Ok(())
}

fn d1_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
