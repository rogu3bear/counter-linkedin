use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Meta, MetaTags, Title};

mod asset_manifest {
    include!(concat!(env!("OUT_DIR"), "/asset_manifest.rs"));
}

#[allow(dead_code)]
pub fn shell(options: LeptosOptions, turnstile_site_key: Option<String>) -> impl IntoView {
    let _ = turnstile_site_key;
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <link rel="icon" href="/favicon.svg" type="image/svg+xml"/>
                <AutoReload options=options.clone()/>
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
        <Title text="CounterLinkedIn — In Memoriam"/>
        <Meta
            name="description"
            content="CounterLinkedIn translated corporate language into consequences. It lived fast, burned tokens, and died young."
        />

        <main class="translate-page" style="min-height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center;">
            <div style="text-align: center; max-width: 640px;">
                <div class="brand" style="justify-content: center; margin-bottom: 24px;">
                    <div class="brand-mark" aria-hidden="true">
                        <span class="brand-mark__glyph">"in"</span>
                    </div>
                </div>

                <h1 style="font-size: clamp(2rem, 4vw, 3.2rem); letter-spacing: -0.05em; margin: 0 0 8px;">
                    "CounterLinkedIn"
                </h1>
                <p style="color: var(--ink-soft); font-size: 1.05rem; margin: 0 0 32px; letter-spacing: -0.01em;">
                    "2025 \u{2013} 2026"
                </p>

                <div class="translator" style="text-align: left; padding: 28px; margin-bottom: 28px;">
                    <p style="margin: 0 0 16px; line-height: 1.6; font-size: 1rem;">
                        "It translated corporate language into consequences. It turned job posts into subtext. It made LinkedIn posts sound like what your coworkers were actually thinking."
                    </p>
                    <p style="margin: 0 0 16px; line-height: 1.6; font-size: 1rem;">
                        "The AI bill came due. The joke did not generate revenue. The site has been sunsetted \u{2014} which, in LinkedIn speak, means "
                        <em>"we are excited to announce a strategic pivot toward not paying for inference."</em>
                    </p>
                    <p style="margin: 0; line-height: 1.6; font-size: 1rem; color: var(--ink-soft);">
                        "Translate first. Regret later. \u{2014} the button nobody clicked enough."
                    </p>
                </div>

                <div style="display: flex; flex-direction: column; gap: 12px; align-items: center;">
                    <p style="margin: 0; color: var(--ink-soft); font-size: 0.88rem; line-height: 1.5;">
                        "The source code remains open. The spirit endures."
                    </p>
                    <a
                        href="https://github.com/rogu3bear/counter-linkedin"
                        target="_blank"
                        rel="noopener noreferrer"
                        class="ghost-action"
                        style="display: inline-flex; align-items: center; text-decoration: none; font-size: 0.9rem;"
                    >
                        "View source on GitHub"
                    </a>
                </div>
            </div>
        </main>
    }
}
