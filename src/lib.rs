mod api;
mod app;
mod components;
#[cfg(feature = "ssr")]
mod server;

#[cfg(feature = "ssr")]
#[worker::event(fetch)]
async fn fetch(
    mut req: worker::HttpRequest,
    env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    use axum::routing::{get, post};
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_service::Service;

    let conf =
        get_configuration(None).map_err(|error| worker::Error::RustError(error.to_string()))?;
    let mut leptos_options = conf.leptos_options;
    leptos_options.output_name = "counter-linkedin".into();
    let routes = generate_route_list(app::App);
    let state = server::AppState::new(leptos_options.clone(), env);

    let headers = req.headers().clone();
    let host = headers
        .get("host")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let path = req.uri().path().to_string();

    if server::analytics::requires_admin_auth(&host, &path) {
        if let Err(response) = server::analytics::authorize(&headers) {
            return Ok(response);
        }
    }

    if let Some(rewritten) = server::analytics::rewrite_path_for_host(&host, &path) {
        let query = req
            .uri()
            .query()
            .map(|value| format!("?{value}"))
            .unwrap_or_default();
        let rewritten_uri: axum::http::Uri = format!("{rewritten}{query}").parse().map_err(
            |error: axum::http::uri::InvalidUri| worker::Error::RustError(error.to_string()),
        )?;
        *req.uri_mut() = rewritten_uri;
    }

    let mut router = Router::new()
        .route("/api/entry/status", get(server::entry::status))
        .route("/api/entry/pass", post(server::entry::pass))
        .route("/api/translate", post(server::translate::translate))
        .route("/api/admin/metrics", get(server::analytics::metrics))
        .leptos_routes_with_context(&state, routes, || {}, {
            let leptos_options = leptos_options.clone();
            let turnstile_site_key = state.turnstile_site_key();
            move || app::shell(leptos_options.clone(), turnstile_site_key.clone())
        })
        .with_state(state);

    Ok(router.call(req).await?)
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
