use std::rc::Rc;

use leptos::{ev::KeyboardEvent, html::Div, prelude::*, task::spawn_local};

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
    let turnstile_required = RwSignal::new(false);
    let turnstile_token = RwSignal::new(None::<String>);
    let turnstile_ready = RwSignal::new(false);
    let turnstile_site_key = RwSignal::new(read_turnstile_site_key());
    let turnstile_mount = NodeRef::<Div>::new();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = turnstile_site_key;

    let submit = Rc::new({
        let mode = mode;
        let input = input;
        let output = output;
        let warnings = warnings;
        let error = error;
        let copied = copied;
        let in_flight = in_flight;
        let last_request = last_request;
        let turnstile_required = turnstile_required;
        let turnstile_token = turnstile_token;
        let turnstile_ready = turnstile_ready;

        move |regenerate: bool| {
            if in_flight.get_untracked() {
                return;
            }

            let token = turnstile_token.get_untracked();
            if turnstile_required.get_untracked() && token.as_deref().unwrap_or_default().is_empty() {
                error.set(Some("Click the human check first.".to_string()));
                return;
            }

            let request = if regenerate {
                match last_request.get_untracked() {
                    Some(mut request) => {
                        request.regenerate = true;
                        request.turnstile_token = token;
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
                    turnstile_token: token,
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
                let turnstile_token = turnstile_token;
                let turnstile_ready = turnstile_ready;

                async move {
                    match post_translate(request).await {
                        Ok(response) => {
                            output.set(response.output);
                            warnings.set(response.warnings);
                            error.set(None);

                            let mut canonical = request_for_success;
                            canonical.regenerate = false;
                            canonical.turnstile_token = None;
                            last_request.set(Some(canonical));
                        }
                        Err(envelope) => {
                            output.set(String::new());
                            warnings.set(envelope.error.warnings.clone());
                            error.set(Some(envelope.error.message));
                        }
                    }

                    if turnstile_required.get_untracked() {
                        turnstile_token.set(None);
                        turnstile_ready.set(false);
                        reset_turnstile_widget();
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
        let turnstile_site_key = turnstile_site_key;
        let turnstile_mount = turnstile_mount;
        let turnstile_token = turnstile_token;
        let turnstile_ready = turnstile_ready;
        let error = error;

        move || {
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
                turnstile_required.set(true);
                turnstile_site_key.set(None);
            }
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

                    <div class="turnstile-panel">
                        <div node_ref=turnstile_mount class="turnstile-widget"></div>
                        <Show when=move || turnstile_required.get()>
                            <p class="turnstile-note">
                                "Cloudflare human check required before each run."
                            </p>
                        </Show>
                    </div>

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
                                disabled=move || in_flight.get() || (turnstile_required.get() && !turnstile_ready.get())
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

fn read_turnstile_site_key() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    {
        let window = web_sys::window()?;
        let document = window.document()?;
        let meta = document
            .query_selector("meta[name='turnstile-site-key']")
            .ok()
            .flatten()?;
        meta.get_attribute("content").filter(|value| !value.is_empty())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
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
        error_signal.set(Some("Human check failed to load. Refresh and try again.".to_string()));
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
