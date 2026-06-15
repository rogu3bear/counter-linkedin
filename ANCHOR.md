# CounterLinkedIn Anchor

## Purpose

This file captures the truths that should stay stable while the code evolves.
CounterLinkedIn is a sunsetted product; its active surface is a static memorial
page. If a proposal conflicts with these anchors, the burden is on the proposal.

## Product Anchors

- The product today is the memorial page at `counterlinkedin.com`, not the former translation app.
- The page must stay honest: it commemorates a dead joke app and links to open source. It does not market a live service.
- The page must be readable without JavaScript; hydration is enhancement, not a requirement for the content.
- Reviving the translation product is a deliberate, code-first decision — never an implicit consequence of another change.

## Architectural Anchors

- One Rust crate, two targets: `hydrate` (WASM, browser) and `ssr` (Cloudflare Workers via `axum` + `worker`). Features are defined in `Cargo.toml`.
- `src/app.rs` owns the page markup and copy. `src/lib.rs` owns the `worker::event(fetch)` SSR handler and the `hydrate()` export. `src/main.rs` is build-harness scaffolding.
- `build.rs` generates the asset manifest (`asset_manifest.rs`) consumed by `app::shell`; do not hardcode hashed asset paths around it.
- Styling lives in `style/main.css` (Leptos `style-file`); assets in `assets/`. Inline `style=` attributes in `app.rs` are layout-local and acceptable for this single page.
- Deploy truth is `wrangler.toml`: `main = "build/index.js"`, static `[assets]` from `./target/site`, custom domain `counterlinkedin.com`, and an empty `[vars]` — no secret-backed or paid bindings.

## Safety Anchors

- No Workers AI, D1, or Turnstile bindings in the runtime. Their migrations and bootstrap scripts are history, not active surface.
- No per-request paid inference and no surprise network calls from a memorial page.
- Preserve the `__wbindgen_start` guard injected by `scripts/wrangler-build.sh`; removing it can crash production on worker reinitialization.
- When the stats/Access surface or any auth is discussed, remember Cloudflare Access JWTs must be validated against JWKS, never trusted as raw headers (per repo history) — but the live memorial carries none of that, and adding it back is a product decision.
- Keep `getrandom` shims (`getrandom`/`getrandom02`/`getrandom03`) intact; they exist for the WASM target, not as accidental cruft.

## Operational Anchors

- Toolchain bootstrap: `./scripts/bootstrap.sh`.
- Static build: `cargo leptos build --release` (requires the `wasm32-unknown-unknown` target).
- Worker bundle build: `./scripts/wrangler-build.sh` (pins `wasm-bindgen` and `worker-build`, applies the start-guard rewrite).
- Local worker preview: `bunx wrangler dev --remote --ip 127.0.0.1 --port 57581`.
- Live Cloudflare/account reads, mutation planning, and verification: `cfctl` from `PATH`, e.g. `cfctl standards audit /Users/star/dev/counter-linkedin`. Keep direct Wrangler usage limited to the static build/dev loop.

## Decision Questions

Before changing code, ask:

1. Does this keep the memorial cheap, static, and honest about being a sunsetted product?
2. Does it avoid adding paid, stateful, or secret-backed bindings (AI/D1/Turnstile) to the runtime?
3. Does it respect the two-target crate layout (`ssr` vs `hydrate`) and the asset-manifest build seam?
4. Does it keep the build reproducible, including the `__wbindgen_start` guard?
5. Can it be verified through `cargo leptos build --release`, `./scripts/wrangler-build.sh`, local `wrangler dev`, and `cfctl` for account truth?

If the answer to any of those is "no", the change probably needs to be smaller or differently shaped.
