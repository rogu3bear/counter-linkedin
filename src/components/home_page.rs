use std::rc::Rc;

use leptos::{ev::KeyboardEvent, prelude::*, task::spawn_local};

use crate::api::{ErrorEnvelope, TranslationMode, TranslationRequest, TranslationResponse};

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

    let submit = Rc::new({
        let mode = mode;
        let input = input;
        let output = output;
        let warnings = warnings;
        let error = error;
        let copied = copied;
        let in_flight = in_flight;
        let last_request = last_request;

        move |regenerate: bool| {
            if in_flight.get_untracked() {
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
                    turnstile_token: None,
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

                async move {
                    match post_translate(request).await {
                        Ok(response) => {
                            output.set(response.output);
                            warnings.set(response.warnings);
                            error.set(None);

                            let mut canonical = request_for_success;
                            canonical.regenerate = false;
                            last_request.set(Some(canonical));
                        }
                        Err(envelope) => {
                            output.set(String::new());
                            warnings.set(envelope.error.warnings.clone());
                            error.set(Some(envelope.error.message));
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

    view! {
        <main class="translate-page">
            <header class="topbar">
                <div class="brand">
                    <div class="brand-mark" aria-hidden="true">
                        <span class="brand-mark__glyph">"in"</span>
                    </div>
                    <div class="brand-copy">
                        <h1>"CounterLinkedIn"</h1>
                        <p>"LinkedIn backwards. Career backwards."</p>
                    </div>
                </div>
            </header>

            <section class="translator">
                <div class="pane pane--input">
                    <div class="pane-head">
                        <div>
                            <p class="pane-label">"Source"</p>
                            <h2>{move || mode.get().input_label()}</h2>
                            <p class="pane-direction">{move || mode.get().input_hint()}</p>
                        </div>
                        <span class="pane-meta">
                            {move || format!("{} / 4000", input.get().chars().count())}
                        </span>
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

                    <div class="pane-actions">
                        <div class="pane-action-group">
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
                            <button
                                class="primary-action"
                                type="button"
                                disabled=move || in_flight.get()
                                on:click={
                                    let submit = submit.clone();
                                    move |_| submit(false)
                                }
                            >
                                {move || if in_flight.get() { "Generating..." } else { "Generate" }}
                            </button>
                        </div>
                        <p class="shortcut-note">"Ctrl/Cmd + Enter"</p>
                    </div>
                </div>

                <div class="pane pane--output">
                    <div class="pane-head">
                        <div>
                            <p class="pane-label">"Return"</p>
                            <h2>"Choose the version you want back."</h2>
                            <p class="pane-direction">"Same meaning. Different career outcome."</p>
                        </div>
                    </div>

                    <div class="output-toolbar">
                        <div class="output-actions output-actions--modes">
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
                                            class="ghost-action mode-action"
                                            class:mode-action--active=move || mode.get() == item
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
                                        <p>{move || empty_state_copy(mode.get())}</p>
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
        let value = JsFuture::from(clipboard.read_text()).await.map_err(|_| ())?;
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

fn client_error(message: impl Into<String>) -> ErrorEnvelope {
    ErrorEnvelope {
        error: crate::api::ApiError::internal(message.into()),
    }
}

#[cfg(target_arch = "wasm32")]
fn js_client_error(error: wasm_bindgen::JsValue) -> ErrorEnvelope {
    client_error(format!("{error:?}"))
}
