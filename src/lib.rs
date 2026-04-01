mod app;

#[cfg(feature = "ssr")]
#[worker::event(fetch)]
async fn fetch(
    req: worker::HttpRequest,
    _env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<axum::http::Response<axum::body::Body>> {
    use axum::Router;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use tower_service::Service;

    let conf =
        get_configuration(None).map_err(|error| worker::Error::RustError(error.to_string()))?;
    let mut leptos_options = conf.leptos_options;
    leptos_options.output_name = "counter-linkedin".into();
    let routes = generate_route_list(app::App);

    let mut router = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || app::shell(leptos_options.clone(), None)
        })
        .with_state(leptos_options);

    Ok(router.call(req).await?)
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(app::App);
}
