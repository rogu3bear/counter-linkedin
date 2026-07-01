# CounterLinkedIn North Star

## Intent

CounterLinkedIn exists today as a memorial. It was a single-screen joke app
(2025–2026) that translated polished professional language into brutally honest
plain English and back. The AI bill came due, the joke did not generate revenue,
and the product was sunsetted. The current job of this repo is to keep that
memorial page alive at the edge — cheaply, statically, and honestly — without
pretending the retired inference product is still running.

## Core Promise

- Serve one static memorial page at `counterlinkedin.com`, fast and forever-cheap.
- Carry no paid inference, no per-request spend, no rate-limit ledger, no human-check gate.
- Stay honest about what the app was and that it is gone. Keep the source open (`View source on GitHub`) and the tone deadpan, not salesy.
- Render through the same Leptos SSR + Cloudflare Workers path the live app used, minus the bindings that cost money.

## Product Shape Today

The repository is a single Rust crate (`counter-linkedin`) that builds two
targets from one source tree:

1. A WASM hydrate bundle (`hydrate` feature) for the browser.
2. A Cloudflare Workers SSR entrypoint (`ssr` feature) wired through `axum` + `worker`.

The only route is `App` in `src/app.rs` — the memorial copy. `src/lib.rs` holds
the `worker::event(fetch)` handler and the `hydrate()` export. `src/main.rs`
exists for the leptos build harness. The Worker is deployed to the custom domain
`counterlinkedin.com` via `wrangler.toml`, built by `scripts/wrangler-build.sh`.

The retired translation product (Workers AI / Llama 3.1 8B, D1 logging and rate
limiting, Turnstile) is gone from the runtime. Its residue still lives in the
tree as history: `migrations/*.sql` and the abuse/spend tables they define,
`scripts/bootstrap.sh` and `scripts/rotate-secrets.sh`. That residue is archive,
not active surface.

## What "Good" Looks Like

- A request to `counterlinkedin.com` returns the memorial page, server-rendered and hydrated, with no binding lookups for AI, D1, or Turnstile.
- The page degrades gracefully: it is readable and complete even if hydration never runs.
- The build is reproducible through `cargo leptos build --release` and `./scripts/wrangler-build.sh`, with the `__wbindgen_start` guard intact so reinitialization does not crash production.
- Cloudflare account state matches a memorial: a static-assets Worker with no paid-inference or secret-backed bindings, confirmed via `cfctl standards audit .`.
- Docs (`README.md`, `AGENTS.md`, `CLAUDE.md`, this file) describe the memorial, never an active translation service.

## Scope Boundaries

- Page content and rendering belong in `src/app.rs` and `style/main.css`.
- The Worker/SSR seam belongs in `src/lib.rs` and the build glue (`build.rs`, `scripts/wrangler-build.sh`).
- Deploy/domain config belongs in `wrangler.toml`.
- Live Cloudflare account reads, mutation planning, and post-change verification go through `cfctl` from `PATH` — not ad hoc Wrangler API calls.
- The retired stack stays retired. Reviving AI/D1/Turnstile is a deliberate, code-first product decision, not a side effect of an unrelated change.

## Decision Filter

Prefer changes that keep the memorial cheap, honest, and boring:

- smaller, more static, fewer runtime dependencies
- no new paid bindings or per-request cost
- copy and styling that stay deadpan and truthful
- build steps that stay reproducible and self-healing

## Anti-Goals

- Quietly reintroducing Workers AI, D1, Turnstile, or any paid/stateful binding.
- Writing docs or copy that imply live translation, inference, or logging.
- Adding telemetry, trackers, or surprise network calls to a memorial page.
- Treating `scripts/bootstrap.sh` or the SQL migrations as proof the old product is back.
- Expanding the single-page memorial into a multi-route app without an explicit product mandate.
