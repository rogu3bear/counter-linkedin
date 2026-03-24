use axum::{
    extract::State,
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    Json,
};
use sha2::{Digest, Sha256};
use wasm_bindgen::JsValue;
use worker::{Fetch, Headers, Method, Request, RequestInit};

use crate::api::{ApiError, EntryPassRequest, EntryStatusResponse};

use super::{rate_limit, AppState};

const ENTRY_COOKIE_NAME: &str = "counterlinkedin_entry";
const ENTRY_COOKIE_MAX_AGE_SECS: u64 = 60 * 60 * 24 * 7;

pub async fn status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    send_wrapper::SendWrapper::new(async move {
        let client_ip = client_ip(&headers);

        match build_status(&state, &headers, &client_ip).await {
            Ok(status) => Json(status).into_response(),
            Err(error) => error_response(error),
        }
    })
    .await
}

pub async fn pass(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<EntryPassRequest>,
) -> Response {
    send_wrapper::SendWrapper::new(async move {
        let client_ip = client_ip(&headers);

        if state.turnstile_secret_present() {
            if payload.turnstile_token.trim().is_empty() {
                return error_response(ApiError::human_check_required(
                    "Finish the entry check first.",
                ));
            }

            if let Err(error) =
                verify_turnstile(&state, payload.turnstile_token.trim(), &client_ip).await
            {
                return error_response(error);
            }
        }

        match build_status(&state, &headers, &client_ip).await {
            Ok(mut status) => {
                status.entry_granted = true;
                (
                    [(header::SET_COOKIE, entry_cookie(&state, &client_ip))],
                    Json(status),
                )
                    .into_response()
            }
            Err(error) => error_response(error),
        }
    })
    .await
}

pub fn has_entry_pass(headers: &HeaderMap, state: &AppState, client_ip: &str) -> bool {
    if !state.turnstile_secret_present() {
        return true;
    }

    let Some(cookie_header) = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };

    let Some(raw_cookie) = cookie_header
        .split(';')
        .map(str::trim)
        .find_map(|part| part.strip_prefix(&format!("{ENTRY_COOKIE_NAME}=")))
    else {
        return false;
    };

    validate_entry_cookie(raw_cookie, client_ip, &state.rate_limit_salt())
}

pub fn client_ip(headers: &HeaderMap) -> String {
    for key in ["cf-connecting-ip", "x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers.get(key).and_then(|value| value.to_str().ok()) {
            if let Some(first) = value.split(',').next() {
                let trimmed = first.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    "local-dev".to_string()
}

async fn build_status(
    state: &AppState,
    headers: &HeaderMap,
    client_ip: &str,
) -> Result<EntryStatusResponse, ApiError> {
    Ok(EntryStatusResponse {
        entry_required: state.turnstile_secret_present(),
        entry_granted: has_entry_pass(headers, state, client_ip),
        usage: rate_limit::usage_summary(state, client_ip, "/api/translate").await?,
    })
}

fn entry_cookie(state: &AppState, client_ip: &str) -> String {
    let expires_at = now_unix_secs() + ENTRY_COOKIE_MAX_AGE_SECS;
    let signature = entry_signature(client_ip, expires_at, &state.rate_limit_salt());

    format!(
        "{ENTRY_COOKIE_NAME}={expires_at}.{signature}; Max-Age={ENTRY_COOKIE_MAX_AGE_SECS}; Path=/; HttpOnly; Secure; SameSite=Lax"
    )
}

fn validate_entry_cookie(value: &str, client_ip: &str, salt: &str) -> bool {
    let Some((expires_at, signature)) = value.split_once('.') else {
        return false;
    };

    let Ok(expires_at) = expires_at.parse::<u64>() else {
        return false;
    };

    expires_at > now_unix_secs() && signature == entry_signature(client_ip, expires_at, salt)
}

fn entry_signature(client_ip: &str, expires_at: u64, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"counterlinkedin-entry:");
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(client_ip.trim().as_bytes());
    hasher.update(b":");
    hasher.update(expires_at.to_string().as_bytes());
    hex::encode(hasher.finalize())
}

fn now_unix_secs() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0).floor() as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default()
    }
}

async fn verify_turnstile(state: &AppState, token: &str, client_ip: &str) -> Result<(), ApiError> {
    let secret = state
        .turnstile_secret()
        .ok_or_else(|| ApiError::internal("Turnstile secret is not configured."))?;

    let params = web_sys::UrlSearchParams::new().map_err(|error| {
        ApiError::internal(format!("Failed to build Turnstile request: {error:?}"))
    })?;
    params.append("secret", &secret);
    params.append("response", token);
    params.append("remoteip", client_ip);

    let headers = Headers::new();
    headers
        .set("Content-Type", "application/x-www-form-urlencoded")
        .map_err(|error| ApiError::internal(format!("Turnstile headers failed: {error}")))?;

    let body = params.to_string().as_string().unwrap_or_default();
    let mut init = RequestInit::new();
    init.with_method(Method::Post);
    init.with_headers(headers);
    init.with_body(Some(JsValue::from_str(&body)));

    let request = Request::new_with_init(
        "https://challenges.cloudflare.com/turnstile/v0/siteverify",
        &init,
    )
    .map_err(|error| ApiError::internal(format!("Turnstile request failed: {error}")))?;

    #[derive(Debug, serde::Deserialize)]
    struct TurnstileVerification {
        success: bool,
        #[serde(default, rename = "error-codes")]
        error_codes: Vec<String>,
    }

    let mut response = Fetch::Request(request)
        .send()
        .await
        .map_err(|error| ApiError::internal(format!("Turnstile fetch failed: {error}")))?;

    let verification = response
        .json::<TurnstileVerification>()
        .await
        .map_err(|error| {
            ApiError::internal(format!("Turnstile response decode failed: {error}"))
        })?;

    if verification.success {
        Ok(())
    } else {
        Err(ApiError::human_check_required(format!(
            "Human check failed: {}",
            verification.error_codes.join(", ")
        )))
    }
}

fn error_response(error: ApiError) -> Response {
    (
        error.status_code(),
        Json(crate::api::ErrorEnvelope { error }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::{entry_signature, validate_entry_cookie};

    #[test]
    fn entry_cookie_validation_accepts_matching_signature() {
        let expires_at = u64::MAX / 2;
        let salt = "salt";
        let client_ip = "127.0.0.1";
        let signature = entry_signature(client_ip, expires_at, salt);
        let cookie_value = format!("{expires_at}.{signature}");

        assert!(validate_entry_cookie(&cookie_value, client_ip, salt));
    }

    #[test]
    fn entry_cookie_validation_rejects_wrong_ip() {
        let expires_at = u64::MAX / 2;
        let salt = "salt";
        let signature = entry_signature("127.0.0.1", expires_at, salt);
        let cookie_value = format!("{expires_at}.{signature}");

        assert!(!validate_entry_cookie(&cookie_value, "127.0.0.2", salt));
    }
}
