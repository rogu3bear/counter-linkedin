use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};

use crate::components::{home_page::HomePage, metrics_page::MetricsPage};

mod asset_manifest {
    include!(concat!(env!("OUT_DIR"), "/asset_manifest.rs"));
}

#[allow(dead_code)]
pub fn shell(options: LeptosOptions, turnstile_site_key: Option<String>) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <AutoReload options=options.clone()/>
                {turnstile_site_key.clone().map(|site_key| view! {
                    <>
                        <meta name="turnstile-site-key" content=site_key.clone()/>
                    </>
                })}
                <link rel="modulepreload" href=format!("/pkg/{}", asset_manifest::JS_FILE)/>
                <script type="module">
                    {format!(
                        "import init, {{ hydrate }} from '/pkg/{}'; init({{ module_or_path: '/pkg/{}' }}).then(() => hydrate());",
                        asset_manifest::JS_FILE,
                        asset_manifest::WASM_FILE
                    )}
                </script>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <link id="leptos" rel="stylesheet" href=format!("/pkg/{}", asset_manifest::CSS_FILE)/>
        <Title text="CounterLinkedIn"/>
        <Meta
            name="description"
            content="Translate professional polish into terminable honesty."
        />

        <Router>
            <Routes fallback=|| view! { <p class="route-miss">"Page not found."</p> }.into_view()>
                <Route path=StaticSegment("") view=HomePage/>
                <Route path=StaticSegment("metrics") view=MetricsPage/>
            </Routes>
        </Router>
    }
}
