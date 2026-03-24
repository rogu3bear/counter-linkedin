use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use worker::D1Type;

use crate::api::{
    DailyMetricsPoint, MetricsSnapshot, MetricsSummary, ModeMetrics, ModelPricing,
    RecentGeneration, TranslationMode,
};

use super::{rate_limit, AppState};

#[derive(Debug, Clone)]
pub struct GenerationLog {
    pub client_ip: String,
    pub host: String,
    pub route: String,
    pub mode: Option<TranslationMode>,
    pub intensity: Option<u8>,
    pub regenerate: bool,
    pub input_text: String,
    pub output_text: Option<String>,
    pub model_name: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_input_cost_usd: f64,
    pub estimated_output_cost_usd: f64,
    pub estimated_total_cost_usd: f64,
    pub latency_ms: Option<i64>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SummaryRow {
    total_requests: i64,
    successful_requests: i64,
    failed_requests: i64,
    requests_today: i64,
    requests_last_24h: i64,
    requests_last_7d: i64,
    estimated_total_cost_usd: Option<f64>,
    estimated_cost_today_usd: Option<f64>,
    estimated_cost_last_24h_usd: Option<f64>,
    estimated_cost_last_7d_usd: Option<f64>,
    average_cost_per_success_usd: Option<f64>,
    average_latency_ms: Option<f64>,
    total_prompt_tokens: Option<i64>,
    total_completion_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct DailyRow {
    day: String,
    requests: i64,
    successful_requests: i64,
    estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ModeRow {
    mode: Option<String>,
    requests: i64,
    estimated_cost_usd: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct RecentRow {
    created_at: String,
    status: String,
    mode: Option<String>,
    intensity: Option<i64>,
    regenerate: i64,
    input_text: String,
    output_text: Option<String>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    total_tokens: Option<i64>,
    estimated_total_cost_usd: Option<f64>,
    latency_ms: Option<i64>,
    error_code: Option<String>,
}

pub fn requires_admin_auth(host: &str, path: &str) -> bool {
    path.starts_with("/api/admin") && !host.eq_ignore_ascii_case("stats.counterlinkedin.com")
}

pub fn rewrite_path_for_host(host: &str, path: &str) -> Option<&'static str> {
    if host.eq_ignore_ascii_case("stats.counterlinkedin.com") && path == "/" {
        Some("/metrics")
    } else {
        None
    }
}

pub fn authorize(headers: &HeaderMap, state: &AppState) -> Result<(), Response> {
    let username = state.admin_username().ok_or_else(admin_not_configured)?;
    let password = state.admin_password().ok_or_else(admin_not_configured)?;
    let Some(value) = headers.get(header::AUTHORIZATION).and_then(|value| value.to_str().ok()) else {
        return Err(auth_challenge());
    };
    let Some(encoded) = value.strip_prefix("Basic ") else {
        return Err(auth_challenge());
    };
    let Ok(decoded) = STANDARD.decode(encoded) else {
        return Err(auth_challenge());
    };
    let Ok(decoded) = String::from_utf8(decoded) else {
        return Err(auth_challenge());
    };
    let Some((candidate_user, candidate_password)) = decoded.split_once(':') else {
        return Err(auth_challenge());
    };

    if candidate_user == username && candidate_password == password {
        Ok(())
    } else {
        Err(auth_challenge())
    }
}

pub async fn metrics(State(state): State<AppState>) -> Response {
    send_wrapper::SendWrapper::new(async move {
        let snapshot = match metrics_inner(&state).await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error })),
                )
                    .into_response();
            }
        };

        (StatusCode::OK, Json(snapshot)).into_response()
    })
    .await
}

pub async fn log_generation(state: &AppState, event: GenerationLog) -> Result<(), String> {
    let Some(db) = state.db() else {
        return Ok(());
    };

    let warnings_json = serde_json::to_string(&event.warnings).map_err(|error| error.to_string())?;
    let fingerprint = rate_limit::fingerprint_ip(&event.client_ip, &state.rate_limit_salt());
    let mode = event.mode.map(mode_string);
    let input_chars = event.input_text.chars().count() as i64;
    let output_chars = event.output_text.as_ref().map(|text| text.chars().count() as i64);
    let intensity = event.intensity.map(i64::from);
    let regenerate = if event.regenerate { 1_i64 } else { 0_i64 };

    db.prepare(
        "INSERT INTO generation_events (
            client_hash, host, route, mode, intensity, regenerate, model_name,
            input_text, input_chars, output_text, output_chars, prompt_tokens,
            completion_tokens, total_tokens, estimated_input_cost_usd,
            estimated_output_cost_usd, estimated_total_cost_usd, latency_ms,
            status, error_code, error_message, warnings_json
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
    )
    .bind_refs(&[
        D1Type::Text(fingerprint.as_str()),
        D1Type::Text(event.host.as_str()),
        D1Type::Text(event.route.as_str()),
        option_text(mode.as_deref()),
        option_int(intensity),
        int_value(regenerate),
        option_text(event.model_name.as_deref()),
        D1Type::Text(event.input_text.as_str()),
        int_value(input_chars),
        option_text(event.output_text.as_deref()),
        option_int(output_chars),
        int_value(event.prompt_tokens),
        int_value(event.completion_tokens),
        int_value(event.total_tokens),
        D1Type::Real(event.estimated_input_cost_usd),
        D1Type::Real(event.estimated_output_cost_usd),
        D1Type::Real(event.estimated_total_cost_usd),
        option_int(event.latency_ms),
        D1Type::Text(event.status.as_str()),
        option_text(event.error_code.as_deref()),
        option_text(event.error_message.as_deref()),
        D1Type::Text(warnings_json.as_str()),
    ])
    .map_err(d1_error)?
    .run()
    .await
    .map_err(d1_error)?;

    Ok(())
}

pub fn estimate_costs(
    state: &AppState,
    prompt_tokens: i64,
    completion_tokens: i64,
) -> (f64, f64, f64) {
    let input_rate = state.input_cost_per_million_usd();
    let output_rate = state.output_cost_per_million_usd();
    let input_cost = (prompt_tokens.max(0) as f64 / 1_000_000.0) * input_rate;
    let output_cost = (completion_tokens.max(0) as f64 / 1_000_000.0) * output_rate;
    let total_cost = input_cost + output_cost;
    (input_cost, output_cost, total_cost)
}

pub fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn mode_string(mode: TranslationMode) -> String {
    serde_json::to_string(&mode)
        .unwrap_or_else(|_| "\"linkedin_to_counter_linkedin\"".to_string())
        .trim_matches('"')
        .to_string()
}

fn auth_challenge() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(header::WWW_AUTHENTICATE, "Basic realm=\"CounterLinkedIn Metrics\"")],
        "Authentication required.",
    )
        .into_response()
}

fn admin_not_configured() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "Admin credentials are not configured for metrics.",
    )
        .into_response()
}

async fn metrics_inner(state: &AppState) -> Result<MetricsSnapshot, String> {
    let Some(db) = state.db() else {
        return Err("D1 binding is unavailable.".to_string());
    };

    let summary_row = db
        .prepare(
            "SELECT
               COUNT(*) AS total_requests,
               COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) AS successful_requests,
               COALESCE(SUM(CASE WHEN status != 'success' THEN 1 ELSE 0 END), 0) AS failed_requests,
               COALESCE(SUM(CASE WHEN date(created_at) = date('now') THEN 1 ELSE 0 END), 0) AS requests_today,
               COALESCE(SUM(CASE WHEN created_at >= datetime('now', '-24 hours') THEN 1 ELSE 0 END), 0) AS requests_last_24h,
               COALESCE(SUM(CASE WHEN created_at >= datetime('now', '-7 days') THEN 1 ELSE 0 END), 0) AS requests_last_7d,
               COALESCE(SUM(estimated_total_cost_usd), 0) AS estimated_total_cost_usd,
               COALESCE(SUM(CASE WHEN date(created_at) = date('now') THEN estimated_total_cost_usd ELSE 0 END), 0) AS estimated_cost_today_usd,
               COALESCE(SUM(CASE WHEN created_at >= datetime('now', '-24 hours') THEN estimated_total_cost_usd ELSE 0 END), 0) AS estimated_cost_last_24h_usd,
               COALESCE(SUM(CASE WHEN created_at >= datetime('now', '-7 days') THEN estimated_total_cost_usd ELSE 0 END), 0) AS estimated_cost_last_7d_usd,
               COALESCE(AVG(CASE WHEN status = 'success' THEN estimated_total_cost_usd END), 0) AS average_cost_per_success_usd,
               COALESCE(AVG(CASE WHEN status = 'success' THEN latency_ms END), 0) AS average_latency_ms,
               COALESCE(SUM(prompt_tokens), 0) AS total_prompt_tokens,
               COALESCE(SUM(completion_tokens), 0) AS total_completion_tokens
             FROM generation_events",
        )
        .first::<SummaryRow>(None)
        .await
        .map_err(d1_error)?
        .unwrap_or(SummaryRow {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            requests_today: 0,
            requests_last_24h: 0,
            requests_last_7d: 0,
            estimated_total_cost_usd: Some(0.0),
            estimated_cost_today_usd: Some(0.0),
            estimated_cost_last_24h_usd: Some(0.0),
            estimated_cost_last_7d_usd: Some(0.0),
            average_cost_per_success_usd: Some(0.0),
            average_latency_ms: Some(0.0),
            total_prompt_tokens: Some(0),
            total_completion_tokens: Some(0),
        });

    let daily_rows = db
        .prepare(
            "SELECT
               date(created_at) AS day,
               COUNT(*) AS requests,
               COALESCE(SUM(CASE WHEN status = 'success' THEN 1 ELSE 0 END), 0) AS successful_requests,
               COALESCE(SUM(estimated_total_cost_usd), 0) AS estimated_cost_usd
             FROM generation_events
             WHERE created_at >= datetime('now', '-13 days')
             GROUP BY date(created_at)
             ORDER BY day DESC",
        )
        .all()
        .await
        .map_err(d1_error)?
        .results::<DailyRow>()
        .map_err(d1_error)?;

    let mode_rows = db
        .prepare(
            "SELECT
               mode,
               COUNT(*) AS requests,
               COALESCE(SUM(estimated_total_cost_usd), 0) AS estimated_cost_usd
             FROM generation_events
             WHERE mode IS NOT NULL
             GROUP BY mode
             ORDER BY requests DESC",
        )
        .all()
        .await
        .map_err(d1_error)?
        .results::<ModeRow>()
        .map_err(d1_error)?;

    let recent_rows = db
        .prepare(
            "SELECT
               created_at,
               status,
               mode,
               intensity,
               regenerate,
               input_text,
               output_text,
               prompt_tokens,
               completion_tokens,
               total_tokens,
               estimated_total_cost_usd,
               latency_ms,
               error_code
             FROM generation_events
             ORDER BY created_at DESC
             LIMIT 40",
        )
        .all()
        .await
        .map_err(d1_error)?
        .results::<RecentRow>()
        .map_err(d1_error)?;

    let pricing = ModelPricing {
        model_name: state.model_name(),
        input_cost_per_million_usd: state.input_cost_per_million_usd(),
        output_cost_per_million_usd: state.output_cost_per_million_usd(),
    };

    Ok(MetricsSnapshot {
        summary: MetricsSummary {
            total_requests: summary_row.total_requests,
            successful_requests: summary_row.successful_requests,
            failed_requests: summary_row.failed_requests,
            requests_today: summary_row.requests_today,
            requests_last_24h: summary_row.requests_last_24h,
            requests_last_7d: summary_row.requests_last_7d,
            estimated_total_cost_usd: summary_row.estimated_total_cost_usd.unwrap_or_default(),
            estimated_cost_today_usd: summary_row.estimated_cost_today_usd.unwrap_or_default(),
            estimated_cost_last_24h_usd: summary_row.estimated_cost_last_24h_usd.unwrap_or_default(),
            estimated_cost_last_7d_usd: summary_row.estimated_cost_last_7d_usd.unwrap_or_default(),
            average_cost_per_success_usd: summary_row.average_cost_per_success_usd.unwrap_or_default(),
            average_latency_ms: summary_row.average_latency_ms.unwrap_or_default(),
            total_prompt_tokens: summary_row.total_prompt_tokens.unwrap_or_default(),
            total_completion_tokens: summary_row.total_completion_tokens.unwrap_or_default(),
            pricing,
        },
        daily: daily_rows
            .into_iter()
            .map(|row| DailyMetricsPoint {
                day: row.day,
                requests: row.requests,
                successful_requests: row.successful_requests,
                estimated_cost_usd: row.estimated_cost_usd.unwrap_or_default(),
            })
            .collect(),
        modes: mode_rows
            .into_iter()
            .map(|row| ModeMetrics {
                mode: row.mode.unwrap_or_else(|| "unknown".to_string()),
                requests: row.requests,
                estimated_cost_usd: row.estimated_cost_usd.unwrap_or_default(),
            })
            .collect(),
        recent: recent_rows
            .into_iter()
            .map(|row| RecentGeneration {
                created_at: row.created_at,
                status: row.status,
                mode: row.mode,
                intensity: row.intensity,
                regenerate: row.regenerate > 0,
                input_text: row.input_text,
                output_text: row.output_text,
                prompt_tokens: row.prompt_tokens.unwrap_or_default(),
                completion_tokens: row.completion_tokens.unwrap_or_default(),
                total_tokens: row.total_tokens.unwrap_or_default(),
                estimated_total_cost_usd: row.estimated_total_cost_usd.unwrap_or_default(),
                latency_ms: row.latency_ms,
                error_code: row.error_code,
            })
            .collect(),
    })
}

fn option_text(value: Option<&str>) -> D1Type<'_> {
    match value {
        Some(value) => D1Type::Text(value),
        None => D1Type::Null,
    }
}

fn option_int(value: Option<i64>) -> D1Type<'static> {
    match value {
        Some(value) => int_value(value),
        None => D1Type::Null,
    }
}

fn int_value(value: i64) -> D1Type<'static> {
    let clamped = value.clamp(i32::MIN as i64, i32::MAX as i64) as i32;
    D1Type::Integer(clamped)
}

fn d1_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
