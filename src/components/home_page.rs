use std::rc::Rc;

use leptos::{ev::KeyboardEvent, html::Div, prelude::*, task::spawn_local};

use crate::api::{
    EntryStatusResponse, ErrorEnvelope, TranslationMode, TranslationRequest, TranslationResponse,
    UsageSummary,
};

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TurnstileScriptState {
    Idle,
    Loading,
    Ready,
    Failed,
}

#[component]
pub fn HomePage() -> impl IntoView {
    let mode = RwSignal::new(TranslationMode::LinkedinToCounterLinkedin);
    let input = RwSignal::new(String::new());
    let output = RwSignal::new(String::new());
    let warnings = RwSignal::new(Vec::<String>::new());
    let error = RwSignal::new(None::<String>);
    let copied = RwSignal::new(false);
    let in_flight = RwSignal::new(false);
    let last_request = RwSignal::new(None::<TranslationRequest>);
    let usage = RwSignal::new(UsageSummary::default());
    let donation_dismissed = RwSignal::new(false);
    let entry_required = RwSignal::new(false);
    let entry_granted = RwSignal::new(false);
    let entry_loading = RwSignal::new(true);
    let entry_pass_in_flight = RwSignal::new(false);
    let turnstile_token = RwSignal::new(None::<String>);
    let turnstile_ready = RwSignal::new(false);
    let turnstile_site_key = RwSignal::new(read_turnstile_site_key());
    let turnstile_mount = NodeRef::<Div>::new();
    let turnstile_rendered = RwSignal::new(false);
    let turnstile_script_state = RwSignal::new(TurnstileScriptState::Idle);
    let status_booted = RwSignal::new(false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (
        turnstile_site_key,
        entry_pass_in_flight,
        turnstile_rendered,
        turnstile_script_state,
        status_booted,
    );

    let submit = Rc::new({
        let mode = mode;
        let input = input;
        let output = output;
        let warnings = warnings;
        let error = error;
        let copied = copied;
        let in_flight = in_flight;
        let last_request = last_request;
        let usage = usage;
        let entry_required = entry_required;
        let entry_granted = entry_granted;
        let turnstile_token = turnstile_token;
        let turnstile_ready = turnstile_ready;

        move |regenerate: bool| {
            if in_flight.get_untracked() {
                return;
            }

            if entry_required.get_untracked() && !entry_granted.get_untracked() {
                error.set(Some("Finish the entry check first.".to_string()));
                return;
            }

            let request = if regenerate {
                match last_request.get_untracked() {
                    Some(mut request) => {
                        request.regenerate = true;
                        request
                    }
                    None => return,
                }
            } else {
                TranslationRequest {
                    input: input.get_untracked(),
                    mode: mode.get_untracked(),
                    intensity: 70,
                    regenerate: false,
                }
            };

            in_flight.set(true);
            copied.set(false);
            error.set(None);
            warnings.set(Vec::new());

            let request_for_success = request.clone();
            spawn_local({
                let output = output;
                let warnings = warnings;
                let error = error;
                let in_flight = in_flight;
                let last_request = last_request;
                let usage = usage;
                let entry_granted = entry_granted;
                let turnstile_token = turnstile_token;
                let turnstile_ready = turnstile_ready;

                async move {
                    match post_translate(request).await {
                        Ok(response) => {
                            output.set(response.output);
                            warnings.set(response.warnings);
                            usage.set(response.usage);
                            error.set(None);

                            let mut canonical = request_for_success;
                            canonical.regenerate = false;
                            last_request.set(Some(canonical));
                        }
                        Err(envelope) => {
                            output.set(String::new());
                            warnings.set(envelope.error.warnings.clone());
                            error.set(Some(envelope.error.message));

                            if envelope.error.code == "human_check_required" {
                                entry_granted.set(false);
                                turnstile_token.set(None);
                                turnstile_ready.set(false);
                                reset_turnstile_widget();
                            }
                        }
                    }

                    in_flight.set(false);
                }
            });
        }
    });

    let copy_output = Rc::new({
        let output = output;
        let copied = copied;

        move || {
            let current_output = output.get_untracked();
            if current_output.trim().is_empty() {
                return;
            }

            spawn_local({
                let copied = copied;
                async move {
                    if copy_text(current_output).await.is_ok() {
                        copied.set(true);
                    }
                }
            });
        }
    });

    let paste_input = Rc::new({
        let input = input;
        let copied = copied;
        let error = error;

        move || {
            spawn_local({
                let input = input;
                let copied = copied;
                let error = error;

                async move {
                    match read_clipboard_text().await {
                        Ok(text) => {
                            input.set(normalize_line_endings(text));
                            copied.set(false);
                            error.set(None);
                        }
                        Err(()) => {
                            error.set(Some(
                                "Clipboard paste failed. Your browser blocked it.".to_string(),
                            ));
                        }
                    }
                }
            });
        }
    });

    #[cfg(target_arch = "wasm32")]
    Effect::new({
        let status_booted = status_booted;
        let entry_required = entry_required;
        let entry_granted = entry_granted;
        let entry_loading = entry_loading;
        let usage = usage;
        let error = error;

        move || {
            if status_booted.get() {
                return;
            }

            status_booted.set(true);

            spawn_local(async move {
                match fetch_entry_status().await {
                    Ok(status) => {
                        entry_required.set(status.entry_required);
                        entry_granted.set(status.entry_granted);
                        usage.set(status.usage);
                        error.set(None);
                    }
                    Err(message) => {
                        error.set(Some(message));
                    }
                }

                entry_loading.set(false);
            });
        }
    });

    #[cfg(target_arch = "wasm32")]
    Effect::new({
        let turnstile_site_key = turnstile_site_key;
        let turnstile_script_state = turnstile_script_state;
        let entry_required = entry_required;
        let entry_granted = entry_granted;
        let error = error;

        move || {
            if !entry_required.get() || entry_granted.get() {
                return;
            }

            if !matches!(
                turnstile_script_state.get(),
                TurnstileScriptState::Idle | TurnstileScriptState::Failed
            ) {
                return;
            }

            let Some(site_key) = turnstile_site_key.get() else {
                error.set(Some("Human check is unavailable right now.".to_string()));
                turnstile_script_state.set(TurnstileScriptState::Failed);
                return;
            };

            turnstile_script_state.set(TurnstileScriptState::Loading);

            if ensure_turnstile_script(site_key, turnstile_script_state, error) {
                return;
            }

            turnstile_script_state.set(TurnstileScriptState::Failed);
            error.set(Some(
                "Human check failed to load. Retry to request the widget again.".to_string(),
            ));
        }
    });

    #[cfg(target_arch = "wasm32")]
    Effect::new({
        let turnstile_site_key = turnstile_site_key;
        let turnstile_mount = turnstile_mount;
        let turnstile_token = turnstile_token;
        let turnstile_ready = turnstile_ready;
        let entry_required = entry_required;
        let entry_granted = entry_granted;
        let error = error;
        let turnstile_rendered = turnstile_rendered;
        let turnstile_script_state = turnstile_script_state;

        move || {
            if !entry_required.get()
                || entry_granted.get()
                || turnstile_rendered.get()
                || turnstile_script_state.get() != TurnstileScriptState::Ready
            {
                return;
            }

            let Some(site_key) = turnstile_site_key.get() else {
                return;
            };
            let Some(container) = turnstile_mount.get() else {
                return;
            };

            if render_turnstile_widget(
                container.into(),
                site_key,
                turnstile_token,
                turnstile_ready,
                error,
            ) {
                turnstile_rendered.set(true);
            }
        }
    });

    #[cfg(target_arch = "wasm32")]
    Effect::new({
        let turnstile_token = turnstile_token;
        let turnstile_ready = turnstile_ready;
        let entry_required = entry_required;
        let entry_granted = entry_granted;
        let entry_loading = entry_loading;
        let entry_pass_in_flight = entry_pass_in_flight;
        let turnstile_rendered = turnstile_rendered;
        let usage = usage;
        let error = error;

        move || {
            if !entry_required.get() || entry_granted.get() || entry_pass_in_flight.get() {
                return;
            }

            let Some(token) = turnstile_token.get() else {
                return;
            };

            if token.trim().is_empty() {
                return;
            }

            entry_pass_in_flight.set(true);
            entry_loading.set(true);

            spawn_local(async move {
                match post_entry_pass(token).await {
                    Ok(status) => {
                        entry_granted.set(status.entry_granted);
                        usage.set(status.usage);
                        turnstile_token.set(None);
                        turnstile_ready.set(false);
                        error.set(None);
                    }
                    Err(message) => {
                        turnstile_token.set(None);
                        turnstile_ready.set(false);
                        turnstile_rendered.set(false);
                        reset_turnstile_widget();
                        error.set(Some(message));
                    }
                }

                entry_pass_in_flight.set(false);
                entry_loading.set(false);
            });
        }
    });

    view! {
        <main class="translate-page">
            <header class="translate-header">
                <div class="brand">
                    <div class="brand-mark" aria-hidden="true">
                        <span class="brand-mark__glyph">"in"</span>
                    </div>
                    <div class="brand-copy">
                        <h1>"CounterLinkedIn"</h1>
                        <p>"Translate corporate language into consequences."</p>
                    </div>
                </div>
                <div class="header-status">
                    <Show when=move || { usage.get().daily_runs > 0 }>
                        <p class="usage-pill">
                            {move || format!("{} / {} today", usage.get().daily_runs, usage.get().daily_cap)}
                        </p>
                    </Show>
                    <p class="header-note">"Utility first. Joke intact."</p>
                </div>
            </header>

            <div
                class="entry-gate"
                class:entry-gate--visible=move || entry_required.get() && !entry_granted.get()
            >
                <div class="entry-gate__scrim"></div>
                <div class="entry-gate__card">
                    <p class="entry-gate__eyebrow">"Human check"</p>
                    <h2>"One gate at the door."</h2>
                    <p class="entry-gate__copy">
                        "Pass it once when you enter. The pre-generate interruption is gone."
                    </p>
                    <div node_ref=turnstile_mount class="entry-gate__widget"></div>
                    <Show
                        when=move || entry_loading.get()
                        fallback=move || {
                            if turnstile_script_state.get() == TurnstileScriptState::Failed {
                                view! {
                                    <div class="entry-gate__actions">
                                        <p class="entry-gate__note">
                                            "Human check failed to load. Retry to request it again."
                                        </p>
                                        <button
                                            class="ghost-action ghost-action--quiet"
                                            type="button"
                                            on:click=move |_| {
                                                turnstile_rendered.set(false);
                                                turnstile_ready.set(false);
                                                turnstile_token.set(None);
                                                turnstile_script_state.set(TurnstileScriptState::Idle);
                                                error.set(None);
                                            }
                                        >
                                            "Retry"
                                        </button>
                                    </div>
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <Show when=move || error.get().is_some()>
                                        <p class="entry-gate__note">{move || error.get().unwrap_or_default()}</p>
                                    </Show>
                                }
                                    .into_any()
                            }
                        }
                    >
                        <p class="entry-gate__note">"Checking access..."</p>
                    </Show>
                </div>
            </div>

            <section class="translator translator--google">
                <div class="translator-bar">
                    <div class="mode-tabs">
                        <For
                            each=move || {
                                [
                                    TranslationMode::LinkedinToCounterLinkedin,
                                    TranslationMode::RawToLinkedin,
                                    TranslationMode::JobPostToHonest,
                                ]
                                .into_iter()
                            }
                            key=|item| *item as u8
                            children=move |item| {
                                view! {
                                    <button
                                        class="mode-tab"
                                        class:mode-tab--active=move || mode.get() == item
                                        type="button"
                                        disabled=move || in_flight.get()
                                        on:click=move |_| {
                                            mode.set(item);
                                            copied.set(false);
                                            error.set(None);
                                        }
                                    >
                                        {item.output_button_label()}
                                    </button>
                                }
                            }
                        />
                    </div>
                    <p class="translator-note">
                        {move || if entry_required.get() && !entry_granted.get() {
                            "Pass the human check once to unlock the tool."
                        } else {
                            "Ctrl/Cmd + Enter"
                        }}
                    </p>
                </div>

                <Show when=move || { usage.get().donation_prompt && !donation_dismissed.get() }>
                    <div class="donation-banner" role="status" aria-live="polite">
                        <div class="donation-banner__copy">
                            <p class="donation-banner__eyebrow">"Approaching 10 runs"</p>
                            <p class="donation-banner__text">
                                "Placeholder donation prompt. Add the real ask later."
                            </p>
                        </div>
                        <button
                            class="ghost-action ghost-action--quiet"
                            type="button"
                            on:click=move |_| donation_dismissed.set(true)
                        >
                            "Dismiss"
                        </button>
                    </div>
                </Show>

                <div class="translate-columns">
                    <section class="editor-pane editor-pane--source">
                        <div class="editor-head">
                            <div>
                                <p class="editor-label">"Source"</p>
                                <h2>{move || mode.get().input_label()}</h2>
                            </div>
                            <button
                                class="ghost-action"
                                type="button"
                                disabled=move || in_flight.get()
                                on:click={
                                    let paste_input = paste_input.clone();
                                    move |_| paste_input()
                                }
                            >
                                "Paste"
                            </button>
                        </div>

                        <textarea
                            id="input-text"
                            class="translate-textarea"
                            rows="12"
                            placeholder=move || mode.get().placeholder()
                            prop:value=move || input.get()
                            on:input=move |ev| {
                                input.set(event_target_value(&ev));
                                copied.set(false);
                                error.set(None);
                            }
                            on:keydown={
                                let submit = submit.clone();
                                move |ev: KeyboardEvent| {
                                    if (ev.ctrl_key() || ev.meta_key()) && ev.key() == "Enter" {
                                        ev.prevent_default();
                                        submit(false);
                                    }
                                }
                            }
                        />

                        <div class="editor-foot">
                            <p class="pane-direction">{move || mode.get().input_hint()}</p>
                            <span class="pane-meta">
                                {move || format!("{} / 4000", input.get().chars().count())}
                            </span>
                        </div>
                    </section>

                    <section class="editor-pane editor-pane--output">
                        <div class="editor-head">
                            <div>
                                <p class="editor-label">"Result"</p>
                                <h2>{move || mode.get().output_button_label()}</h2>
                            </div>
                            <div class="output-actions output-actions--utility">
                                <button
                                    class="ghost-action"
                                    type="button"
                                    disabled=move || in_flight.get() || output.get().trim().is_empty()
                                    on:click={
                                        let copy_output = copy_output.clone();
                                        move |_| copy_output()
                                    }
                                >
                                    {move || if copied.get() { "Copied" } else { "Copy" }}
                                </button>
                                <button
                                    class="ghost-action"
                                    type="button"
                                    disabled=move || in_flight.get() || last_request.get().is_none()
                                    on:click={
                                        let submit = submit.clone();
                                        move |_| submit(true)
                                    }
                                >
                                    "Regenerate"
                                </button>
                            </div>
                        </div>

                        <Show when=move || error.get().is_some()>
                            <div class="feedback feedback--error" role="alert">
                                {move || error.get().unwrap_or_default()}
                            </div>
                        </Show>

                        <Show
                            when=move || in_flight.get()
                            fallback=move || {
                                if output.get().trim().is_empty() {
                                    view! {
                                        <div class="output-empty">
                                            <p class="output-empty__label">"Result"</p>
                                            <p class="output-empty__copy">{move || empty_state_copy(mode.get())}</p>
                                        </div>
                                    }
                                        .into_any()
                                } else {
                                    view! {
                                        <div class="output-copy" aria-live="polite">
                                            {move || output.get()}
                                        </div>
                                    }
                                        .into_any()
                                }
                            }
                        >
                            <LoadingPane/>
                        </Show>

                        <Show when=move || !warnings.get().is_empty()>
                            <ul class="warning-list">
                                <For
                                    each=move || warnings.get()
                                    key=|warning| warning.clone()
                                    children=move |warning| view! { <li>{warning}</li> }
                                />
                            </ul>
                        </Show>

                        <div class="editor-foot editor-foot--action">
                            <button
                                class="primary-action translate-action"
                                type="button"
                                disabled=move || in_flight.get() || entry_loading.get() || (entry_required.get() && !entry_granted.get())
                                on:click={
                                    let submit = submit.clone();
                                    move |_| submit(false)
                                }
                            >
                                {move || if in_flight.get() { "Generating..." } else { "Translate" }}
                            </button>
                            <p class="shortcut-note">
                                {move || if usage.get().donation_prompt {
                                    "Donation banner is active near the 10-run mark."
                                } else {
                                    "Translate first. Regret later."
                                }}
                            </p>
                        </div>
                    </section>
                </div>
            </section>
        </main>
    }
}

#[component]
fn LoadingPane() -> impl IntoView {
    view! {
        <div class="loading-pane" aria-live="polite" aria-busy="true">
            <div class="loading-line"></div>
            <div class="loading-line loading-line--short"></div>
            <div class="loading-line"></div>
        </div>
    }
}

fn empty_state_copy(mode: TranslationMode) -> &'static str {
    match mode {
        TranslationMode::LinkedinToCounterLinkedin => "The fireable version lands here.",
        TranslationMode::RawToLinkedin => "The employable version lands here.",
        TranslationMode::JobPostToHonest => "The subtext lands here.",
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn fetch_entry_status() -> Result<EntryStatusResponse, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};

        let options = RequestInit::new();
        options.set_method("GET");
        options.set_mode(RequestMode::SameOrigin);

        let request = Request::new_with_str_and_init("/api/entry/status", &options)
            .map_err(|error| format!("{error:?}"))?;

        let window = web_sys::window().ok_or_else(|| "Missing browser window".to_string())?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|error| format!("{error:?}"))?
            .dyn_into::<Response>()
            .map_err(|error| format!("{error:?}"))?;

        let json = JsFuture::from(response.json().map_err(|error| format!("{error:?}"))?)
            .await
            .map_err(|error| format!("{error:?}"))?;

        if response.ok() {
            serde_wasm_bindgen::from_value(json).map_err(|error| error.to_string())
        } else {
            let envelope = serde_wasm_bindgen::from_value::<ErrorEnvelope>(json)
                .map_err(|error| error.to_string())?;
            Err(envelope.error.message)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Err("Entry status is only available in the browser.".to_string())
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
async fn post_entry_pass(_token: String) -> Result<EntryStatusResponse, String> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};

        let payload = serde_json::to_string(&crate::api::EntryPassRequest {
            turnstile_token: _token,
        })
        .map_err(|error| error.to_string())?;

        let options = RequestInit::new();
        options.set_method("POST");
        options.set_mode(RequestMode::SameOrigin);
        options.set_body(&wasm_bindgen::JsValue::from_str(&payload));

        let request = Request::new_with_str_and_init("/api/entry/pass", &options)
            .map_err(|error| format!("{error:?}"))?;
        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(|error| format!("{error:?}"))?;

        let window = web_sys::window().ok_or_else(|| "Missing browser window".to_string())?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|error| format!("{error:?}"))?
            .dyn_into::<Response>()
            .map_err(|error| format!("{error:?}"))?;

        let json = JsFuture::from(response.json().map_err(|error| format!("{error:?}"))?)
            .await
            .map_err(|error| format!("{error:?}"))?;

        if response.ok() {
            serde_wasm_bindgen::from_value(json).map_err(|error| error.to_string())
        } else {
            let envelope = serde_wasm_bindgen::from_value::<ErrorEnvelope>(json)
                .map_err(|error| error.to_string())?;
            Err(envelope.error.message)
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Err("Entry pass is only available in the browser.".to_string())
    }
}

async fn post_translate(request: TranslationRequest) -> Result<TranslationResponse, ErrorEnvelope> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Request, RequestInit, RequestMode, Response};

        let payload =
            serde_json::to_string(&request).map_err(|error| client_error(error.to_string()))?;
        let options = RequestInit::new();
        options.set_method("POST");
        options.set_mode(RequestMode::SameOrigin);
        options.set_body(&wasm_bindgen::JsValue::from_str(&payload));

        let request =
            Request::new_with_str_and_init("/api/translate", &options).map_err(js_client_error)?;
        request
            .headers()
            .set("Content-Type", "application/json")
            .map_err(js_client_error)?;

        let window = web_sys::window().ok_or_else(|| client_error("Missing browser window"))?;
        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(js_client_error)?
            .dyn_into::<Response>()
            .map_err(js_client_error)?;

        let json = JsFuture::from(response.json().map_err(js_client_error)?)
            .await
            .map_err(js_client_error)?;

        if response.ok() {
            serde_wasm_bindgen::from_value(json).map_err(|error| client_error(error.to_string()))
        } else {
            Err(serde_wasm_bindgen::from_value(json).unwrap_or_else(|_| {
                client_error("The server returned an unreadable error response.")
            }))
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = request;
        Err(client_error(
            "Client-side fetch is only available in the browser.",
        ))
    }
}

async fn copy_text(text: String) -> Result<(), ()> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::JsFuture;

        let Some(window) = web_sys::window() else {
            return Err(());
        };

        let clipboard = window.navigator().clipboard();
        JsFuture::from(clipboard.write_text(&text))
            .await
            .map_err(|_| ())?;
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = text;
        Err(())
    }
}

async fn read_clipboard_text() -> Result<String, ()> {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen_futures::JsFuture;

        let Some(window) = web_sys::window() else {
            return Err(());
        };

        let clipboard = window.navigator().clipboard();
        let value = JsFuture::from(clipboard.read_text())
            .await
            .map_err(|_| ())?;
        value.as_string().ok_or(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        Err(())
    }
}

fn normalize_line_endings(text: String) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn read_turnstile_site_key() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let document = window.document()?;
        let meta = document
            .query_selector("meta[name='turnstile-site-key']")
            .ok()
            .flatten()?;
        meta.get_attribute("content")
            .filter(|value| !value.is_empty())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

#[cfg(target_arch = "wasm32")]
fn ensure_turnstile_script(
    site_key: String,
    script_state: RwSignal<TurnstileScriptState>,
    error_signal: RwSignal<Option<String>>,
) -> bool {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};

    const TURNSTILE_SCRIPT_ID: &str = "counterlinkedin-turnstile-script";
    const TURNSTILE_SCRIPT_SRC: &str =
        "https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit";

    let Some(window) = web_sys::window() else {
        return false;
    };
    let Some(document) = window.document() else {
        return false;
    };
    let Some(head) = document.head() else {
        return false;
    };

    let _ = site_key;

    if js_sys::Reflect::get(&window, &JsValue::from_str("turnstile"))
        .map(|value| !value.is_undefined() && !value.is_null())
        .unwrap_or(false)
    {
        script_state.set(TurnstileScriptState::Ready);
        error_signal.set(None);
        return true;
    }

    if let Some(existing) = document.get_element_by_id(TURNSTILE_SCRIPT_ID) {
        if existing.get_attribute("data-ready").as_deref() == Some("true") {
            script_state.set(TurnstileScriptState::Ready);
            error_signal.set(None);
        }
        return true;
    }

    let Ok(element) = document.create_element("script") else {
        return false;
    };
    let Ok(script) = element.dyn_into::<web_sys::HtmlScriptElement>() else {
        return false;
    };

    script.set_id(TURNSTILE_SCRIPT_ID);
    script.set_src(TURNSTILE_SCRIPT_SRC);
    script.set_defer(true);

    let onload_script = script.clone();
    let onload = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        onload_script.set_attribute("data-ready", "true").ok();
        script_state.set(TurnstileScriptState::Ready);
        error_signal.set(None);
    }));
    script.set_onload(Some(onload.as_ref().unchecked_ref()));
    onload.forget();

    let onerror_script = script.clone();
    let onerror = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        onerror_script.remove();
        script_state.set(TurnstileScriptState::Failed);
        error_signal.set(Some(
            "Human check failed to load. Retry to request it again.".to_string(),
        ));
    }));
    script.set_onerror(Some(onerror.as_ref().unchecked_ref()));
    onerror.forget();

    head.append_child(&script).is_ok()
}

fn client_error(message: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        error: crate::api::ApiError::internal(message.into()),
    }
}

#[cfg(target_arch = "wasm32")]
fn js_client_error(error: wasm_bindgen::JsValue) -> ErrorEnvelope {
    client_error(format!("{error:?}"))
}

#[cfg(target_arch = "wasm32")]
fn render_turnstile_widget(
    container: web_sys::HtmlElement,
    site_key: String,
    token_signal: RwSignal<Option<String>>,
    ready_signal: RwSignal<bool>,
    error_signal: RwSignal<Option<String>>,
) -> bool {
    use wasm_bindgen::{closure::Closure, JsCast, JsValue};

    let Ok(window) = web_sys::window().ok_or(()) else {
        return false;
    };
    let turnstile = match js_sys::Reflect::get(&window, &JsValue::from_str("turnstile")) {
        Ok(value) if !value.is_undefined() && !value.is_null() => value,
        _ => return false,
    };

    let Ok(render_value) = js_sys::Reflect::get(&turnstile, &JsValue::from_str("render")) else {
        return false;
    };
    let Ok(render) = render_value.dyn_into::<js_sys::Function>() else {
        return false;
    };

    let options = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &options,
        &JsValue::from_str("sitekey"),
        &JsValue::from_str(&site_key),
    );
    let _ = js_sys::Reflect::set(
        &options,
        &JsValue::from_str("theme"),
        &JsValue::from_str("light"),
    );

    let success = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |token: JsValue| {
        token_signal.set(token.as_string());
        ready_signal.set(true);
        error_signal.set(None);
    }));
    let _ = js_sys::Reflect::set(
        &options,
        &JsValue::from_str("callback"),
        success.as_ref().unchecked_ref(),
    );
    success.forget();

    let expired = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        ready_signal.set(false);
        token_signal.set(None);
    }));
    let _ = js_sys::Reflect::set(
        &options,
        &JsValue::from_str("expired-callback"),
        expired.as_ref().unchecked_ref(),
    );
    expired.forget();

    let errored = Closure::<dyn FnMut()>::wrap(Box::new(move || {
        ready_signal.set(false);
        token_signal.set(None);
        error_signal.set(Some(
            "Human check failed to load. Refresh and try again.".to_string(),
        ));
    }));
    let _ = js_sys::Reflect::set(
        &options,
        &JsValue::from_str("error-callback"),
        errored.as_ref().unchecked_ref(),
    );
    errored.forget();

    let _ = render.call2(&turnstile, container.as_ref(), &options);
    true
}

#[cfg(target_arch = "wasm32")]
fn reset_turnstile_widget() {
    use wasm_bindgen::{JsCast, JsValue};

    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(turnstile) = js_sys::Reflect::get(&window, &JsValue::from_str("turnstile")) else {
        return;
    };
    if turnstile.is_undefined() || turnstile.is_null() {
        return;
    }
    let Ok(reset_value) = js_sys::Reflect::get(&turnstile, &JsValue::from_str("reset")) else {
        return;
    };
    let Ok(reset) = reset_value.dyn_into::<js_sys::Function>() else {
        return;
    };
    let _ = reset.call0(&turnstile);
}

#[cfg(not(target_arch = "wasm32"))]
fn reset_turnstile_widget() {}
