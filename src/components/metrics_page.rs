use leptos::prelude::*;
use leptos_meta::{Meta, Title};

use crate::api::{MetricsSnapshot, ModeMetrics, RecentGeneration};

#[component]
pub fn MetricsPage() -> impl IntoView {
    let metrics = LocalResource::new(fetch_metrics);

    view! {
        <main class="metrics-page">
            <Title text="Metrics — CounterLinkedIn"/>
            <Meta
                name="description"
                content="Usage, spend, and generation analytics for CounterLinkedIn."
            />
            <header class="metrics-header">
                <div class="brand">
                    <div class="brand-mark" aria-hidden="true">
                        <span class="brand-mark__glyph">"in"</span>
                    </div>
                    <div class="brand-copy">
                        <h1>"CounterLinkedIn Metrics"</h1>
                        <p>"Usage, spend, and every generation that hit the edge."</p>
                    </div>
                </div>
            </header>

            <Suspense fallback=move || view! { <MetricsLoading/> }>
                {move || {
                    metrics
                        .get()
                        .map(|result| match result {
                            Ok(snapshot) => view! { <MetricsDashboard snapshot/> }.into_any(),
                            Err(message) => view! {
                                <section class="metrics-error" role="alert">
                                    <h2>"Metrics unavailable."</h2>
                                    <p>{message}</p>
                                </section>
                            }
                                .into_any(),
                        })
                }}
            </Suspense>
        </main>
    }
}

#[component]
fn MetricsDashboard(snapshot: MetricsSnapshot) -> impl IntoView {
    let summary = snapshot.summary.clone();
    let modes = snapshot.modes.clone();
    let recent = snapshot.recent.clone();

    view! {
        <section class="metrics-grid">
            <MetricCard label="Total requests".to_string() value=summary.total_requests.to_string() detail=format!("{} successes", summary.successful_requests)/>
            <MetricCard label="Estimated spend".to_string() value=format_usd(summary.estimated_total_cost_usd) detail=format!("{} in the last 7 days", format_usd(summary.estimated_cost_last_7d_usd))/>
            <MetricCard label="24h spend".to_string() value=format_usd(summary.estimated_cost_last_24h_usd) detail=format!("{} requests in the last 24h", summary.requests_last_24h)/>
            <MetricCard label="Average request cost".to_string() value=format_usd(summary.average_cost_per_success_usd) detail=format!("{} today", format_usd(summary.estimated_cost_today_usd))/>
            <MetricCard label="Average latency".to_string() value=format!("{} ms", summary.average_latency_ms.round() as i64) detail=format!("{} requests today", summary.requests_today)/>
            <MetricCard label="Prompt tokens".to_string() value=summary.total_prompt_tokens.to_string() detail=format!("{} completion tokens", summary.total_completion_tokens)/>
            <MetricCard label="Current model pricing".to_string() value=format!("{} / {} per 1M", format_usd(summary.pricing.input_cost_per_million_usd), format_usd(summary.pricing.output_cost_per_million_usd)) detail=summary.pricing.model_name/>
        </section>

        <section class="metrics-section">
            <div class="metrics-section__head">
                <h2>"Mode mix"</h2>
                <p>"Which kind of mutation is costing money."</p>
            </div>
            <div class="mode-list">
                <For
                    each=move || modes.clone().into_iter()
                    key=|item| item.mode.clone()
                    children=move |item: ModeMetrics| {
                        let requests = item.requests.max(0) as f64;
                        let width = if summary.total_requests > 0 {
                            (requests / summary.total_requests as f64) * 100.0
                        } else {
                            0.0
                        };

                        view! {
                            <div class="mode-row">
                                <div class="mode-row__meta">
                                    <strong>{human_mode(&item.mode)}</strong>
                                    <span>{format!("{} requests", item.requests)}</span>
                                </div>
                                <div class="mode-bar">
                                    <div class="mode-bar__fill" style=format!("width: {:.2}%;", width.clamp(0.0, 100.0))></div>
                                </div>
                                <div class="mode-row__cost">{format_usd(item.estimated_cost_usd)}</div>
                            </div>
                        }
                    }
                />
            </div>
        </section>

        <section class="metrics-section">
            <div class="metrics-section__head">
                <h2>"Daily run rate"</h2>
                <p>"Requests and estimated spend by day."</p>
            </div>
            <div class="daily-table">
                <div class="daily-table__head">
                    <span>"Day"</span>
                    <span>"Requests"</span>
                    <span>"Success"</span>
                    <span>"Estimated spend"</span>
                </div>
                <For
                    each=move || snapshot.daily.clone().into_iter()
                    key=|item| item.day.clone()
                    children=move |item| view! {
                        <div class="daily-table__row">
                            <span>{item.day}</span>
                            <span>{item.requests}</span>
                            <span>{item.successful_requests}</span>
                            <span>{format_usd(item.estimated_cost_usd)}</span>
                        </div>
                    }
                />
            </div>
        </section>

        <section class="metrics-section">
            <div class="metrics-section__head">
                <h2>"Recent generations"</h2>
                <p>"Inputs, outputs, status, spend, and latency."</p>
            </div>
            <div class="generation-list">
                <For
                    each=move || recent.clone().into_iter()
                    key=|item| format!("{}-{}", item.created_at, item.input_text)
                    children=move |item: RecentGeneration| {
                        view! {
                            <article class="generation-card">
                                <div class="generation-card__head">
                                    <div>
                                        <strong>{item.mode.as_deref().map(human_mode).unwrap_or("Unknown")}</strong>
                                        <p>{item.created_at.clone()}</p>
                                    </div>
                                    <div class="generation-card__stats">
                                        <span class=format!("status-pill status-pill--{}", item.status)>{item.status.clone()}</span>
                                        <span>{format_usd(item.estimated_total_cost_usd)}</span>
                                        <span>{format!("{} tok", item.total_tokens)}</span>
                                        <span>{item.latency_ms.map(|value| format!("{value} ms")).unwrap_or_else(|| "n/a".to_string())}</span>
                                    </div>
                                </div>
                                <div class="generation-card__body">
                                    <div>
                                        <h3>"Input"</h3>
                                        <p>{item.input_text.clone()}</p>
                                    </div>
                                    <div>
                                        <h3>"Output"</h3>
                                        <p>{item.output_text.unwrap_or_else(|| "No output stored.".to_string())}</p>
                                    </div>
                                </div>
                            </article>
                        }
                    }
                />
            </div>
        </section>
    }
}

#[component]
fn MetricCard(label: String, value: String, detail: String) -> impl IntoView {
    view! {
        <article class="metric-card">
            <p class="metric-card__label">{label}</p>
            <strong class="metric-card__value">{value}</strong>
            <p class="metric-card__detail">{detail}</p>
        </article>
    }
}

#[component]
fn MetricsLoading() -> impl IntoView {
    view! {
        <section class="metrics-grid">
            <For
                each=move || 0..6
                key=|item| *item
                children=move |_| view! {
                    <article class="metric-card metric-card--loading">
                        <div class="loading-line"></div>
                        <div class="loading-line loading-line--short"></div>
                    </article>
                }
            />
        </section>
    }
}

fn human_mode(mode: &str) -> &'static str {
    match mode {
        "linkedin_to_counter_linkedin" => "CounterLinkedIn",
        "raw_to_linkedin" => "LinkedIn",
        "job_post_to_honest" => "Honest",
        _ => "Unknown",
    }
}

fn format_usd(amount: f64) -> String {
    format!("${amount:.4}")
}

async fn fetch_metrics() -> Result<MetricsSnapshot, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};

        let options = RequestInit::new();
        options.set_method("GET");
        options.set_mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init("/api/admin/metrics", &options)
            .map_err(|error| format!("{error:?}"))?;

        let window = web_sys::window().ok_or_else(|| "Missing browser window".to_string())?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|error| format!("{error:?}"))?
            .dyn_into::<Response>()
            .map_err(|error| format!("{error:?}"))?;

        if !response.ok() {
            return Err(format!("Metrics endpoint returned {}.", response.status()));
        }

        let json = JsFuture::from(response.json().map_err(|error| format!("{error:?}"))?)
            .await
            .map_err(|error| format!("{error:?}"))?;

        serde_wasm_bindgen::from_value(json).map_err(|error| error.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Err("Metrics fetch is only available in the browser.".to_string())
    }
}
