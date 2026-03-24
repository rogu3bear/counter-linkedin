#![cfg_attr(not(feature = "ssr"), allow(dead_code))]

use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "@cf/meta/llama-3.1-8b-instruct";
pub const MAX_INPUT_CHARS: usize = 4_000;
pub const MAX_OUTPUT_CHARS: usize = 1_280;
pub const MAX_OUTPUT_TOKENS: u16 = 360;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
    #[default]
    LinkedinToCounterLinkedin,
    RawToLinkedin,
    JobPostToHonest,
}

impl TranslationMode {
    pub fn input_label(self) -> &'static str {
        "Paste the text."
    }

    pub fn input_hint(self) -> &'static str {
        "Posts, pitches, job ads. Indeknil can tell."
    }

    pub fn placeholder(self) -> &'static str {
        "Paste the text."
    }

    pub fn output_button_label(self) -> &'static str {
        match self {
            Self::LinkedinToCounterLinkedin => "CounterLinkedIn",
            Self::RawToLinkedin => "LinkedIn",
            Self::JobPostToHonest => "Honest",
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationRequest {
    pub input: String,
    pub mode: TranslationMode,
    pub intensity: u8,
    #[serde(default)]
    pub regenerate: bool,
    #[serde(default)]
    pub turnstile_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationResponse {
    pub output: String,
    pub mode: TranslationMode,
    pub intensity: u8,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEnvelope {
    pub error: ApiError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelPricing {
    pub model_name: String,
    pub input_cost_per_million_usd: f64,
    pub output_cost_per_million_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSnapshot {
    pub summary: MetricsSummary,
    pub daily: Vec<DailyMetricsPoint>,
    pub modes: Vec<ModeMetrics>,
    pub recent: Vec<RecentGeneration>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MetricsSummary {
    pub total_requests: i64,
    pub successful_requests: i64,
    pub failed_requests: i64,
    pub requests_today: i64,
    pub requests_last_24h: i64,
    pub requests_last_7d: i64,
    pub estimated_total_cost_usd: f64,
    pub estimated_cost_today_usd: f64,
    pub estimated_cost_last_24h_usd: f64,
    pub estimated_cost_last_7d_usd: f64,
    pub average_cost_per_success_usd: f64,
    pub average_latency_ms: f64,
    pub total_prompt_tokens: i64,
    pub total_completion_tokens: i64,
    pub pricing: ModelPricing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyMetricsPoint {
    pub day: String,
    pub requests: i64,
    pub successful_requests: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModeMetrics {
    pub mode: String,
    pub requests: i64,
    pub estimated_cost_usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentGeneration {
    pub created_at: String,
    pub status: String,
    pub mode: Option<String>,
    pub intensity: Option<i64>,
    pub regenerate: bool,
    pub input_text: String,
    pub output_text: Option<String>,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub estimated_total_cost_usd: f64,
    pub latency_ms: Option<i64>,
    pub error_code: Option<String>,
}

impl ApiError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "bad_request".to_string(),
            message: message.into(),
            warnings: vec![],
        }
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self {
            code: "rate_limited".to_string(),
            message: message.into(),
            warnings: vec![],
        }
    }

    pub fn upstream_failure(message: impl Into<String>) -> Self {
        Self {
            code: "upstream_failure".to_string(),
            message: message.into(),
            warnings: vec![],
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error".to_string(),
            message: message.into(),
            warnings: vec![],
        }
    }

    pub fn human_check_required(message: impl Into<String>) -> Self {
        Self {
            code: "human_check_required".to_string(),
            message: message.into(),
            warnings: vec![],
        }
    }
}

#[cfg(feature = "ssr")]
impl ApiError {
    pub fn status_code(&self) -> axum::http::StatusCode {
        match self.code.as_str() {
            "bad_request" => axum::http::StatusCode::BAD_REQUEST,
            "rate_limited" => axum::http::StatusCode::TOO_MANY_REQUESTS,
            "human_check_required" => axum::http::StatusCode::FORBIDDEN,
            "upstream_failure" => axum::http::StatusCode::BAD_GATEWAY,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromptBundle {
    pub system: String,
    pub user: String,
    pub max_tokens: u16,
    pub temperature: f32,
}

pub fn validate_request(mut request: TranslationRequest) -> Result<TranslationRequest, ApiError> {
    request.input = request.input.trim().to_string();

    if request.input.is_empty() {
        return Err(ApiError::bad_request("Paste something first."));
    }

    if request.input.chars().count() > MAX_INPUT_CHARS {
        return Err(ApiError::bad_request(format!(
            "Input is capped at {MAX_INPUT_CHARS} characters."
        )));
    }

    if request.intensity > 100 {
        return Err(ApiError::bad_request(
            "Intensity must stay between 0 and 100.",
        ));
    }

    Ok(request)
}

pub fn build_prompt(request: &TranslationRequest) -> PromptBundle {
    let intensity_band = intensity_band(request.intensity);
    let intensity_directive = intensity_directive(request.intensity);
    let regenerate_note = if request.regenerate {
        "Produce a fresh alternative wording, not the same cadence as last time."
    } else {
        "Give the best first-pass answer."
    };

    let system = match request.mode {
        TranslationMode::LinkedinToCounterLinkedin => format!(
            concat!(
                "You write in CounterLinkedIn.\n",
                "Take polished LinkedIn language and mutate it into fireable, unhinged, anti-LinkedIn copy.\n",
                "It should still look like something a person might post on LinkedIn.\n",
                "Preserve the exact semantic core.\n",
                "Keep it short, sharp, witty, and distinctly CounterLinkedIn.\n",
                "Do not add facts, allegations, or backstory.\n",
                "Ask: am I saying something evil?\n",
                "If yes, do not say it.\n",
                "Do no evil.\n",
                "No profanity.\n",
                "No evil.\n",
                "No cruelty, harassment, threats, or slurs.\n",
                "Do not become generic insult-comedy.\n",
                "No prefacing or labels.\n",
                "Current intensity band: {intensity_band}. {intensity_directive}"
            ),
            intensity_band = intensity_band,
            intensity_directive = intensity_directive
        ),
        TranslationMode::RawToLinkedin => format!(
            concat!(
                "You turn blunt human thoughts into polished, status-safe LinkedIn language.\n",
                "Keep the meaning, improve diplomacy, and stay concise.\n",
                "Do not turn one sentence into a TED Talk.\n",
                "Do not invent achievements, metrics, or gratitude.\n",
                "Do no evil.\n",
                "No prefacing or labels.\n",
                "Current intensity band: {intensity_band}. {intensity_directive}"
            ),
            intensity_band = intensity_band,
            intensity_directive = intensity_directive
        ),
        TranslationMode::JobPostToHonest => format!(
            concat!(
                "You translate job posts and recruiter outreach into CounterLinkedIn honesty.\n",
                "Surface subtext, vague workload expectations, and likely realities.\n",
                "Stay grounded in what the text supports.\n",
                "Be funny, sharp, concise, and readable.\n",
                "Do no evil.\n",
                "No prefacing or labels.\n",
                "Current intensity band: {intensity_band}. {intensity_directive}"
            ),
            intensity_band = intensity_band,
            intensity_directive = intensity_directive
        ),
    };

    let user = match request.mode {
        TranslationMode::LinkedinToCounterLinkedin => format!(
            concat!(
                "Mode: LinkedIn -> CounterLinkedIn\n",
                "{regenerate_note}\n",
                "Take this and make it fireable, unhinged, and totally against LinkedIn principles.\n",
                "Make it feel post-shaped, like someone actually posted it on LinkedIn.\n",
                "Then apply a strict do-no-evil rule.\n",
                "No profanity.\n",
                "Return only the rewritten text.\n",
                "No intro sentence. Start directly with the post.\n",
                "Aim for 1-2 short paragraphs.\n",
                "Usually 2-6 sentences total.\n",
                "Source:\n{input}"
            ),
            regenerate_note = regenerate_note,
            input = request.input
        ),
        TranslationMode::RawToLinkedin => format!(
            concat!(
                "Mode: Raw thought -> LinkedIn\n",
                "{regenerate_note}\n",
                "Return only the cleaned-up version.\n",
                "No intro sentence. Start directly with the rewrite.\n",
                "Aim for 1-2 short paragraphs.\n",
                "Usually 2-5 crisp sentences total.\n",
                "Source:\n{input}"
            ),
            regenerate_note = regenerate_note,
            input = request.input
        ),
        TranslationMode::JobPostToHonest => format!(
            concat!(
                "Mode: Job post -> Honest translation\n",
                "{regenerate_note}\n",
                "Return only the honest translation.\n",
                "No intro sentence. Start directly with the translation.\n",
                "Aim for 1-2 short paragraphs.\n",
                "Usually 3-7 compact sentences total.\n",
                "Source:\n{input}"
            ),
            regenerate_note = regenerate_note,
            input = request.input
        ),
    };

    PromptBundle {
        system,
        user,
        max_tokens: MAX_OUTPUT_TOKENS,
        temperature: temperature_for(request.intensity),
    }
}

pub fn sanitize_output(raw: &str) -> (String, bool) {
    let trimmed = raw
        .trim()
        .trim_matches('\"')
        .trim_matches('\'')
        .replace("\r\n", "\n");
    let trimmed = strip_leading_framing(&trimmed);
    let (trimmed, paragraph_truncated) = limit_paragraphs(&trimmed, 2);

    let mut truncated = paragraph_truncated;
    let mut output = String::new();

    for (index, ch) in trimmed.chars().enumerate() {
        if index >= MAX_OUTPUT_CHARS {
            truncated = true;
            break;
        }

        output.push(ch);
    }

    if truncated {
        output.push_str("...");
    }

    (output.trim().to_string(), truncated)
}

fn strip_leading_framing(text: &str) -> String {
    let mut value = text.trim().to_string();

    const PREFIXES: &[&str] = &[
        "here is your ",
        "here's your ",
        "here is the ",
        "here's the ",
        "certainly, ",
        "absolutely, ",
        "counterlinkedin version:",
        "counterlinkedin rewrite:",
        "rewritten version:",
        "honest translation:",
        "translation:",
    ];

    loop {
        let lower = value.to_lowercase();
        let mut changed = false;

        for prefix in PREFIXES {
            if lower.starts_with(prefix) {
                value = value[prefix.len()..].trim_start_matches([' ', '\n', ':']).trim().to_string();
                changed = true;
                break;
            }
        }

        if !changed {
            break;
        }
    }

    value
}

fn limit_paragraphs(text: &str, max_paragraphs: usize) -> (String, bool) {
    let paragraphs = text
        .split("\n\n")
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    if paragraphs.len() <= max_paragraphs {
        return (paragraphs.join("\n\n"), false);
    }

    (paragraphs[..max_paragraphs].join("\n\n"), true)
}

pub fn intensity_band(intensity: u8) -> &'static str {
    match intensity {
        0..=20 => "mild candidness",
        21..=40 => "pointed",
        41..=60 => "risky honesty",
        61..=80 => "definitely not posting this",
        _ => "HR incident",
    }
}

fn intensity_directive(intensity: u8) -> &'static str {
    match intensity {
        0..=20 => "Use dry candor. Let the mask slip a little, not all at once.",
        21..=40 => "Sharpen the subtext and make the professional facade noticeably unstable.",
        41..=60 => "Get bolder, stranger, and more CounterLinkedIn while staying semantically faithful.",
        61..=80 => "Make it feel obviously unpostable and fireable, but still controlled and intelligible.",
        _ => "Push toward full HR-incident energy without profanity, fabricated facts, or outright malice.",
    }
}

fn temperature_for(intensity: u8) -> f32 {
    match intensity {
        0..=20 => 0.35,
        21..=40 => 0.55,
        41..=60 => 0.7,
        61..=80 => 0.82,
        _ => 0.92,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_empty_input() {
        let result = validate_request(TranslationRequest {
            input: "   ".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 50,
            regenerate: false,
            turnstile_token: None,
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "bad_request");
    }

    #[test]
    fn prompt_mentions_mode_specific_rules() {
        let request = TranslationRequest {
            input: "Thrilled to share that I joined Acme.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 70,
            regenerate: false,
            turnstile_token: None,
        };

        let prompt = build_prompt(&request);

        assert!(prompt.system.contains("You write in CounterLinkedIn."));
        assert!(prompt.user.contains("LinkedIn -> CounterLinkedIn"));
    }

    #[test]
    fn sanitize_output_truncates_long_text() {
        let source = "x".repeat(MAX_OUTPUT_CHARS + 20);
        let (output, truncated) = sanitize_output(&source);

        assert!(truncated);
        assert!(output.ends_with("..."));
    }

    #[test]
    fn sanitize_output_keeps_only_two_paragraphs() {
        let source = "first\n\nsecond\n\nthird";
        let (output, truncated) = sanitize_output(source);

        assert!(truncated);
        assert_eq!(output, "first\n\nsecond...");
    }
}
