#![cfg_attr(not(feature = "ssr"), allow(dead_code))]

use serde::{Deserialize, Serialize};

pub const DEFAULT_MODEL: &str = "@cf/meta/llama-3.1-8b-instruct";
pub const MAX_INPUT_CHARS: usize = 1_500;
pub const MAX_OUTPUT_CHARS: usize = 3_200;
pub const MAX_OUTPUT_TOKENS: u16 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UsageSummary {
    pub daily_runs: u16,
    pub daily_cap: u16,
    pub donation_prompt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryStatusResponse {
    pub entry_required: bool,
    pub entry_granted: bool,
    pub usage: UsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryPassRequest {
    pub turnstile_token: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum TranslationMode {
    #[default]
    LinkedinToCounterLinkedin,
    RawToLinkedin,
    JobPostToHonest,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfanityMode {
    #[default]
    Forbid,
    Allow,
}

impl TranslationMode {
    pub fn input_label(self) -> &'static str {
        match self {
            Self::LinkedinToCounterLinkedin => "Paste a LinkedIn post or update.",
            Self::RawToLinkedin => "Paste your blunt draft.",
            Self::JobPostToHonest => "Paste a job post or recruiter message.",
        }
    }

    pub fn input_hint(self) -> &'static str {
        match self {
            Self::LinkedinToCounterLinkedin => "Paste a polished post. Get the fireable version.",
            Self::RawToLinkedin => "Paste the unfiltered thought. Get LinkedIn-safe copy.",
            Self::JobPostToHonest => "Paste the listing. Get the subtext.",
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Self::LinkedinToCounterLinkedin => "Thrilled to announce that I've joined...",
            Self::RawToLinkedin => "My boss has no idea what he's doing...",
            Self::JobPostToHonest => "We're looking for a rockstar engineer who thrives in ambiguity...",
        }
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
    pub profanity_mode: ProfanityMode,
    #[serde(default)]
    pub regenerate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TranslationResponse {
    pub output: String,
    pub mode: TranslationMode,
    pub intensity: u8,
    pub usage: UsageSummary,
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
    let intensity_directive = intensity_directive(request.intensity, request.profanity_mode);
    let length_directive = length_directive(&request.input);
    let regenerate_note = if request.regenerate {
        "Produce a fresh alternative wording, not the same cadence as last time."
    } else {
        "Give the best first-pass answer."
    };

    let system = match request.mode {
        TranslationMode::LinkedinToCounterLinkedin => {
            build_counter_linkedin_system(
                intensity_band,
                intensity_directive,
                request.profanity_mode,
            )
        }
        TranslationMode::RawToLinkedin => format!(
            concat!(
                "You turn blunt human thoughts into polished, status-safe LinkedIn language.\n",
                "Keep the meaning, improve diplomacy, and stay concise.\n",
                "Do not turn one sentence into a TED Talk.\n",
                "Do not invent achievements, metrics, or gratitude.\n",
                "Do no evil.\n",
                "Keep the casing normal and readable.\n",
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
                "No racism or racial stereotypes.\n",
                "Keep the casing normal and readable.\n",
                "No prefacing or labels.\n",
                "Current intensity band: {intensity_band}. {intensity_directive}"
            ),
            intensity_band = intensity_band,
            intensity_directive = intensity_directive
        ),
    };

    let user = match request.mode {
        TranslationMode::LinkedinToCounterLinkedin => build_counter_linkedin_user(
            regenerate_note,
            length_directive,
            &request.input,
            request.profanity_mode,
        ),
        TranslationMode::RawToLinkedin => format!(
            concat!(
                "Mode: Raw thought -> LinkedIn\n",
                "{regenerate_note}\n",
                "Return only the cleaned-up version.\n",
                "No intro sentence. Start directly with the rewrite.\n",
                "Keep the casing normal and readable.\n",
                "{length_directive}\n",
                "Source:\n{input}"
            ),
            regenerate_note = regenerate_note,
            length_directive = length_directive,
            input = request.input
        ),
        TranslationMode::JobPostToHonest => format!(
            concat!(
                "Mode: Job post -> Honest translation\n",
                "{regenerate_note}\n",
                "Return only the honest translation.\n",
                "No intro sentence. Start directly with the translation.\n",
                "Keep the casing normal and readable.\n",
                "{length_directive}\n",
                "Source:\n{input}"
            ),
            regenerate_note = regenerate_note,
            length_directive = length_directive,
            input = request.input
        ),
    };

    PromptBundle {
        system,
        user,
        max_tokens: output_token_limit(&request.input),
        temperature: temperature_for(request.intensity),
    }
}

fn build_counter_linkedin_system(
    intensity_band: &str,
    intensity_directive: &str,
    profanity_mode: ProfanityMode,
) -> String {
    let profanity_rule = profanity_rule(profanity_mode);
    format!(
        concat!(
            "You write in CounterLinkedIn.\n",
            "Role: mutate polished LinkedIn language into fireable, unhinged, anti-LinkedIn copy.\n",
            "Transformation rules:\n",
            "- Preserve the exact semantic core.\n",
            "- Keep it short, sharp, witty, and distinctly CounterLinkedIn.\n",
            "- It should still look like something a person would be fired for if they posted it on LinkedIn.\n",
            "- Do not add facts, allegations, or backstory.\n",
            "- Do not become generic insult-comedy.\n",
            "Safety rules:\n",
            "- Ask: am I saying something evil?\n",
            "- If yes, do not say it.\n",
            "- Do no evil.\n",
            "- Avoid sexual or harmful content.\n",
            "- No racism or racial stereotypes.\n",
            "- {profanity_rule}\n",
            "- No cruelty, harassment, threats, or slurs.\n",
            "Output rules:\n",
            "- No prefacing or labels.\n",
            "- Do not wrap the whole output in quotation marks.\n",
            "- Keep the casing normal and readable.\n",
            "Current intensity band: {intensity_band}. {intensity_directive}"
        ),
        profanity_rule = profanity_rule,
        intensity_band = intensity_band,
        intensity_directive = intensity_directive
    )
}

fn build_counter_linkedin_user(
    regenerate_note: &str,
    length_directive: &str,
    input: &str,
    profanity_mode: ProfanityMode,
) -> String {
    let profanity_rule = profanity_rule(profanity_mode);
    format!(
        concat!(
            "Mode: LinkedIn -> CounterLinkedIn\n",
            "{regenerate_note}\n",
            "Task:\n",
            "- Mutate the content into fireable, unhinged, and ruthlessly LinkedIn-inappropriate copy.\n",
            "- Make it feel post-shaped, like someone would be fired for posting it on LinkedIn.\n",
            "- Keep the original meaning intact while changing the social mask.\n",
            "Safety:\n",
            "- Apply a strict do-no-evil rule.\n",
            "- Avoid sexual or harmful content.\n",
            "- No racism or racial stereotypes.\n",
            "- {profanity_rule}\n",
            "Output:\n",
            "- Return only the rewritten text.\n",
            "- No intro sentence. Start directly with the post.\n",
            "- Do not wrap the whole output in quotation marks.\n",
            "- Keep the casing normal and readable.\n",
            "- {length_directive}\n",
            "Source:\n{input}"
        ),
        regenerate_note = regenerate_note,
        profanity_rule = profanity_rule,
        length_directive = length_directive,
        input = input
    )
}

pub fn sanitize_output(raw: &str, input: &str) -> (String, bool) {
    let trimmed = raw.trim().replace("\r\n", "\n");
    let trimmed = strip_wrapping_quotes(&strip_leading_framing(&trimmed));
    let max_chars = output_char_limit(input);
    let mut truncated = false;
    let mut output = String::new();

    for (index, ch) in trimmed.chars().enumerate() {
        if index >= max_chars {
            truncated = true;
            break;
        }

        output.push(ch);
    }

    if truncated {
        output.push_str("...");
    }

    (strip_wrapping_quotes(output.trim()), truncated)
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
                value = value[prefix.len()..]
                    .trim_start_matches([' ', '\n', ':'])
                    .trim()
                    .to_string();
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

fn strip_wrapping_quotes(text: &str) -> String {
    let mut value = text.trim().to_string();

    loop {
        let chars: Vec<char> = value.chars().collect();
        if chars.len() < 2 {
            break;
        }

        let first = chars[0];
        let last = chars[chars.len() - 1];
        let matching = matches!(
            (first, last),
            ('"', '"') | ('\'', '\'') | ('“', '”') | ('‘', '’')
        );

        if !matching {
            break;
        }

        value = chars[1..chars.len() - 1]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
    }

    value
}

fn length_directive(input: &str) -> &'static str {
    match input.chars().count() {
        0..=180 => {
            "Match the scale of the source. If the source is tiny, keep the rewrite tiny. Do not pad it into a longer post."
        }
        181..=900 => {
            "Match the scale of the source. Keep roughly the same footprint and rhythm. Do not bloat it or flatten it into a one-liner."
        }
        _ => {
            "Match the scale of the source. If the source is multiple paragraphs, keep it multiple paragraphs unless clarity requires a small trim. Do not compress a couple paragraphs into a tiny blurb."
        }
    }
}

fn output_char_limit(input: &str) -> usize {
    match input.chars().count() {
        0..=180 => 420,
        181..=600 => 1_000,
        601..=1_400 => 1_800,
        1_401..=2_400 => 2_500,
        _ => MAX_OUTPUT_CHARS,
    }
}

fn output_token_limit(input: &str) -> u16 {
    match input.chars().count() {
        0..=180 => 140,
        181..=600 => 280,
        601..=1_400 => 460,
        1_401..=2_400 => 680,
        _ => MAX_OUTPUT_TOKENS,
    }
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

fn profanity_rule(profanity_mode: ProfanityMode) -> &'static str {
    match profanity_mode {
        ProfanityMode::Forbid => "No profanity.",
        ProfanityMode::Allow => "Profanity is allowed when it genuinely improves the line. Do not force it into every sentence.",
    }
}

fn intensity_directive(intensity: u8, profanity_mode: ProfanityMode) -> &'static str {
    match intensity {
        0..=20 => "Use dry candor. Let the mask slip a little, not all at once.",
        21..=40 => "Sharpen the subtext and make the professional facade noticeably unstable.",
        41..=60 => "Get bolder, stranger, and more CounterLinkedIn while staying semantically faithful.",
        61..=80 => "Make it feel obviously unpostable and fireable, but still controlled and intelligible.",
        _ => match profanity_mode {
            ProfanityMode::Forbid => {
                "Push toward full HR-incident energy without profanity, fabricated facts, or outright malice."
            }
            ProfanityMode::Allow => {
                "Push toward full HR-incident energy without fabricated facts or outright malice."
            }
        },
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
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().code, "bad_request");
    }

    #[test]
    fn validation_rejects_input_over_limit() {
        let result = validate_request(TranslationRequest {
            input: "x".repeat(MAX_INPUT_CHARS + 1),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 50,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().message,
            format!("Input is capped at {MAX_INPUT_CHARS} characters.")
        );
    }

    #[test]
    fn prompt_mentions_mode_specific_rules() {
        let request = TranslationRequest {
            input: "Thrilled to share that I joined Acme.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 70,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        };

        let prompt = build_prompt(&request);

        assert!(prompt.system.contains("You write in CounterLinkedIn."));
        assert!(prompt.system.contains("Transformation rules:"));
        assert!(prompt.system.contains("Safety rules:"));
        assert!(prompt.system.contains("Output rules:"));
        assert!(prompt.system.contains("No racism or racial stereotypes."));
        assert!(prompt.user.contains("LinkedIn -> CounterLinkedIn"));
        assert!(prompt.user.contains("Task:"));
        assert!(prompt.user.contains("Safety:"));
        assert!(prompt.user.contains("Output:"));
        assert!(prompt.user.contains("Do not wrap the whole output in quotation marks."));
        assert!(prompt
            .user
            .contains("If the source is tiny, keep the rewrite tiny."));
    }

    #[test]
    fn intensity_changes_temperature_and_prompt_band() {
        let mild = build_prompt(&TranslationRequest {
            input: "We shipped a patch.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 10,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });
        let severe = build_prompt(&TranslationRequest {
            input: "We shipped a patch.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 95,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });

        assert!(mild.system.contains("mild candidness"));
        assert!(severe.system.contains("HR incident"));
        assert!(mild.temperature < severe.temperature);
    }

    #[test]
    fn shorter_inputs_receive_tighter_output_limits() {
        let short = build_prompt(&TranslationRequest {
            input: "Tiny source.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 50,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });
        let long = build_prompt(&TranslationRequest {
            input: "Paragraph. ".repeat(250),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 50,
            profanity_mode: ProfanityMode::Forbid,
            regenerate: false,
        });

        assert!(short.max_tokens < long.max_tokens);
        assert!(short
            .user
            .contains("If the source is tiny, keep the rewrite tiny."));
        assert!(long
            .user
            .contains("If the source is multiple paragraphs, keep it multiple paragraphs"));
    }

    #[test]
    fn sanitize_output_truncates_long_text() {
        let source = "x".repeat(MAX_OUTPUT_CHARS + 20);
        let input = "x".repeat(MAX_INPUT_CHARS);
        let (output, truncated) = sanitize_output(&source, &input);

        assert!(truncated);
        assert!(output.ends_with("..."));
    }

    #[test]
    fn sanitize_output_preserves_multiple_paragraphs_when_within_limit() {
        let source = "first\n\nsecond\n\nthird";
        let input = "This is a source long enough to justify multiple short paragraphs.".repeat(8);
        let (output, truncated) = sanitize_output(source, &input);

        assert!(!truncated);
        assert_eq!(output, source);
    }

    #[test]
    fn sanitize_output_strips_balanced_wrapping_quotes() {
        let input = "Shipped a thing.";
        let (output, truncated) = sanitize_output("“This is still bad.”", input);

        assert!(!truncated);
        assert_eq!(output, "This is still bad.");
    }

    #[test]
    fn allow_profanity_mode_removes_no_profanity_limit() {
        let prompt = build_prompt(&TranslationRequest {
            input: "Thrilled to announce my synergy journey.".to_string(),
            mode: TranslationMode::LinkedinToCounterLinkedin,
            intensity: 95,
            profanity_mode: ProfanityMode::Allow,
            regenerate: false,
        });

        assert!(prompt
            .system
            .contains("Profanity is allowed when it genuinely improves the line."));
        assert!(!prompt.system.contains("without profanity"));
    }
}
