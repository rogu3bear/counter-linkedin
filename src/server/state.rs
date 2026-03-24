use std::sync::Arc;

use axum::extract::FromRef;
use leptos::prelude::LeptosOptions;

use crate::api::DEFAULT_MODEL;

#[derive(Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub env: Arc<worker::Env>,
}

impl AppState {
    pub fn new(leptos_options: LeptosOptions, env: worker::Env) -> Self {
        Self {
            leptos_options,
            env: Arc::new(env),
        }
    }

    pub fn ai(&self) -> worker::Result<worker::Ai> {
        self.env.ai("AI")
    }

    pub fn db(&self) -> Option<worker::D1Database> {
        self.env.d1("DB").ok()
    }

    pub fn model_name(&self) -> String {
        self.env
            .var("WORKERS_AI_MODEL")
            .ok()
            .map(|value| value.to_string())
            .unwrap_or_else(|| DEFAULT_MODEL.to_string())
    }

    pub fn rate_limit_salt(&self) -> String {
        if let Ok(value) = self.env.secret("RATE_LIMIT_SALT") {
            return value.to_string();
        }

        self.env
            .var("RATE_LIMIT_SALT")
            .ok()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "counter-linkedin-local".to_string())
    }

    pub fn turnstile_secret_present(&self) -> bool {
        self.env.secret("TURNSTILE_SECRET").is_ok() || self.env.var("TURNSTILE_SECRET").is_ok()
    }

    pub fn turnstile_secret(&self) -> Option<String> {
        self.env
            .secret("TURNSTILE_SECRET")
            .ok()
            .or_else(|| self.env.var("TURNSTILE_SECRET").ok())
            .map(|value| value.to_string())
    }

    pub fn turnstile_site_key(&self) -> Option<String> {
        self.env
            .var("TURNSTILE_SITE_KEY")
            .ok()
            .map(|value| value.to_string())
    }

    pub fn input_cost_per_million_usd(&self) -> f64 {
        self.env
            .var("AI_INPUT_COST_PER_MILLION_USD")
            .ok()
            .and_then(|value| value.to_string().parse::<f64>().ok())
            .unwrap_or_else(|| default_model_pricing(&self.model_name()).0)
    }

    pub fn output_cost_per_million_usd(&self) -> f64 {
        self.env
            .var("AI_OUTPUT_COST_PER_MILLION_USD")
            .ok()
            .and_then(|value| value.to_string().parse::<f64>().ok())
            .unwrap_or_else(|| default_model_pricing(&self.model_name()).1)
    }

    pub fn admin_username(&self) -> Option<String> {
        self.env
            .secret("ADMIN_USERNAME")
            .ok()
            .or_else(|| self.env.var("ADMIN_USERNAME").ok())
            .map(|value| value.to_string())
    }

    pub fn admin_password(&self) -> Option<String> {
        self.env
            .secret("ADMIN_PASSWORD")
            .ok()
            .or_else(|| self.env.var("ADMIN_PASSWORD").ok())
            .map(|value| value.to_string())
    }
}

impl FromRef<AppState> for LeptosOptions {
    fn from_ref(input: &AppState) -> Self {
        input.leptos_options.clone()
    }
}

fn default_model_pricing(model: &str) -> (f64, f64) {
    match model {
        "@cf/meta/llama-3.1-8b-instruct" => (0.28, 0.83),
        _ => (0.0, 0.0),
    }
}
