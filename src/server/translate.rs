use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::{
    build_prompt, sanitize_output, validate_request, ApiError, ErrorEnvelope, TranslationRequest,
    TranslationResponse,
};

use super::{analytics, entry, rate_limit, AppState};

#[derive(Debug, Serialize)]
struct AiInput<'a> {
    prompt: &'a str,
    max_tokens: u16,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct AiOutput {
    response: String,
    #[serde(default)]
    usage: Option<AiUsage>,
}

#[derive(Debug, Deserialize)]
struct AiUsage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

pub async fn translate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TranslationRequest>,
) -> Response {
    send_wrapper::SendWrapper::new(async move { translate_inner(state, headers, payload).await })
        .await
}

async fn translate_inner(
    state: AppState,
    headers: HeaderMap,
    payload: TranslationRequest,
) -> Response {
    let client_ip = entry::client_ip(&headers);
    let host = host_name(&headers);
    let raw_input = payload.input.clone();
    let raw_mode = payload.mode;
    let raw_intensity = payload.intensity;
    let raw_regenerate = payload.regenerate;

    let request = match validate_request(payload) {
        Ok(request) => request,
        Err(error) => {
            let _ = analytics::log_generation(
                &state,
                analytics::GenerationLog {
                    client_ip,
                    host,
                    route: "/api/translate".to_string(),
                    mode: Some(raw_mode),
                    intensity: Some(raw_intensity),
                    regenerate: raw_regenerate,
                    input_text: raw_input,
                    output_text: None,
                    model_name: None,
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    estimated_input_cost_usd: 0.0,
                    estimated_output_cost_usd: 0.0,
                    estimated_total_cost_usd: 0.0,
                    latency_ms: None,
                    status: "bad_request".to_string(),
                    error_code: Some(error.code.clone()),
                    error_message: Some(error.message.clone()),
                    warnings: error.warnings.clone(),
                },
            )
            .await;
            return error_response(error);
        }
    };

    if let Err(error) = rate_limit::enforce(&state, &client_ip, "/api/translate").await {
        let _ = analytics::log_generation(
            &state,
            analytics::GenerationLog {
                client_ip,
                host,
                route: "/api/translate".to_string(),
                mode: Some(request.mode),
                intensity: Some(request.intensity),
                regenerate: request.regenerate,
                input_text: request.input.clone(),
                output_text: None,
                model_name: Some(state.model_name()),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                estimated_input_cost_usd: 0.0,
                estimated_output_cost_usd: 0.0,
                estimated_total_cost_usd: 0.0,
                latency_ms: None,
                status: "rate_limited".to_string(),
                error_code: Some(error.code.clone()),
                error_message: Some(error.message.clone()),
                warnings: error.warnings.clone(),
            },
        )
        .await;
        return error_response(error);
    }

    if !entry::has_entry_pass(&headers, &state, &client_ip) {
        return error_response(ApiError::human_check_required(
            "Finish the entry check first.",
        ));
    }

    let prompt = build_prompt(&request);
    let prompt_text = format!("SYSTEM\n{}\n\nUSER\n{}", prompt.system, prompt.user);
    let started_at = analytics::now_ms();
    let model_name = state.model_name();

    let ai = match state.ai() {
        Ok(ai) => ai,
        Err(error) => {
            let api_error =
                ApiError::internal(format!("Workers AI binding is unavailable: {error}"));
            let _ = analytics::log_generation(
                &state,
                analytics::GenerationLog {
                    client_ip,
                    host,
                    route: "/api/translate".to_string(),
                    mode: Some(request.mode),
                    intensity: Some(request.intensity),
                    regenerate: request.regenerate,
                    input_text: request.input.clone(),
                    output_text: None,
                    model_name: Some(model_name),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    estimated_input_cost_usd: 0.0,
                    estimated_output_cost_usd: 0.0,
                    estimated_total_cost_usd: 0.0,
                    latency_ms: Some((analytics::now_ms() - started_at).round() as i64),
                    status: "internal_error".to_string(),
                    error_code: Some(api_error.code.clone()),
                    error_message: Some(api_error.message.clone()),
                    warnings: api_error.warnings.clone(),
                },
            )
            .await;
            return error_response(api_error);
        }
    };

    let ai_result = ai
        .run::<_, AiOutput>(
            model_name.clone(),
            AiInput {
                prompt: &prompt_text,
                max_tokens: prompt.max_tokens,
                temperature: prompt.temperature,
            },
        )
        .await;

    let ai_output = match ai_result {
        Ok(output) => output,
        Err(error) => {
            let api_error =
                ApiError::upstream_failure(format!("Workers AI request failed: {error}"));
            let _ = analytics::log_generation(
                &state,
                analytics::GenerationLog {
                    client_ip,
                    host,
                    route: "/api/translate".to_string(),
                    mode: Some(request.mode),
                    intensity: Some(request.intensity),
                    regenerate: request.regenerate,
                    input_text: request.input.clone(),
                    output_text: None,
                    model_name: Some(model_name),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    estimated_input_cost_usd: 0.0,
                    estimated_output_cost_usd: 0.0,
                    estimated_total_cost_usd: 0.0,
                    latency_ms: Some((analytics::now_ms() - started_at).round() as i64),
                    status: "upstream_failure".to_string(),
                    error_code: Some(api_error.code.clone()),
                    error_message: Some(api_error.message.clone()),
                    warnings: api_error.warnings.clone(),
                },
            )
            .await;
            return error_response(api_error);
        }
    };

    let (output, was_truncated) = sanitize_output(&ai_output.response, &request.input);
    if output.is_empty() {
        let api_error = ApiError::upstream_failure("Workers AI returned an empty response.");
        let _ = analytics::log_generation(
            &state,
            analytics::GenerationLog {
                client_ip,
                host,
                route: "/api/translate".to_string(),
                mode: Some(request.mode),
                intensity: Some(request.intensity),
                regenerate: request.regenerate,
                input_text: request.input.clone(),
                output_text: None,
                model_name: Some(model_name),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
                estimated_input_cost_usd: 0.0,
                estimated_output_cost_usd: 0.0,
                estimated_total_cost_usd: 0.0,
                latency_ms: Some((analytics::now_ms() - started_at).round() as i64),
                status: "upstream_failure".to_string(),
                error_code: Some(api_error.code.clone()),
                error_message: Some(api_error.message.clone()),
                warnings: api_error.warnings.clone(),
            },
        )
        .await;
        return error_response(api_error);
    }

    let usage = ai_output.usage.unwrap_or(AiUsage {
        prompt_tokens: 0,
        completion_tokens: 0,
        total_tokens: 0,
    });
    let (estimated_input_cost_usd, estimated_output_cost_usd, estimated_total_cost_usd) =
        analytics::estimate_costs(&state, usage.prompt_tokens, usage.completion_tokens);

    let mut warnings = Vec::new();
    if was_truncated {
        warnings.push("Output was trimmed to stay proportional to the source.".to_string());
    }
    let _ = analytics::log_generation(
        &state,
        analytics::GenerationLog {
            client_ip,
            host,
            route: "/api/translate".to_string(),
            mode: Some(request.mode),
            intensity: Some(request.intensity),
            regenerate: request.regenerate,
            input_text: request.input.clone(),
            output_text: Some(output.clone()),
            model_name: Some(model_name),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            estimated_input_cost_usd,
            estimated_output_cost_usd,
            estimated_total_cost_usd,
            latency_ms: Some((analytics::now_ms() - started_at).round() as i64),
            status: "success".to_string(),
            error_code: None,
            error_message: None,
            warnings: warnings.clone(),
        },
    )
    .await;

    (
        StatusCode::OK,
        Json(TranslationResponse {
            output,
            mode: request.mode,
            intensity: request.intensity,
            usage: rate_limit::usage_summary(&state, &client_ip, "/api/translate")
                .await
                .unwrap_or_default(),
            warnings,
        }),
    )
        .into_response()
}

fn error_response(error: ApiError) -> Response {
    (error.status_code(), Json(ErrorEnvelope { error })).into_response()
}

fn host_name(headers: &HeaderMap) -> String {
    headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "local-dev".to_string())
}
