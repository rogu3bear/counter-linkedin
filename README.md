# CounterLinkedIn

CounterLinkedIn is a single-screen Leptos app that flips polished professional language into brutally honest plain English, and back again when needed.

The MVP keeps the `rogu3bear/leptos-cloudflare` full-stack shape intact:

- single Rust crate
- Leptos SSR + hydration
- Cloudflare Workers runtime
- Workers AI for generation
- D1-backed request throttling, generation logging, and spend estimation

## Modes

- `linkedin_to_counter_linkedin`: polished update in, career-risk honesty out
- `raw_to_linkedin`: blunt draft in, status-safe rewrite out
- `job_post_to_honest`: recruiter/job copy in, grounded subtext out

## Required tools

```bash
rustup toolchain install stable
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos --locked
```

This repo uses `bunx wrangler`, so you do not need a global Wrangler install.

## Local setup

Run the template checks first:

```bash
./scripts/check-deps.sh
./scripts/bootstrap.sh
```

Create a D1 database for the rate-limit and analytics tables:

```bash
bunx wrangler d1 create counter-linkedin-db
```

Copy the returned `database_id` into `wrangler.toml` for both `database_id` and `preview_database_id`.

Apply the migration locally:

```bash
bunx wrangler d1 migrations apply counter-linkedin-db --local
```

For production, apply it remotely too:

```bash
bunx wrangler d1 migrations apply counter-linkedin-db --remote
```

## Required bindings and env vars

`wrangler.toml` expects these bindings:

- `AI`: Workers AI binding
- `DB`: D1 database binding

These vars are used by the Worker:

- `WORKERS_AI_MODEL`: Workers AI model name. Default is `@cf/meta/llama-3.1-8b-instruct`
- `AI_INPUT_COST_PER_MILLION_USD`: per-million input token price used for spend estimation
- `AI_OUTPUT_COST_PER_MILLION_USD`: per-million output token price used for spend estimation

Optional secret:

- `RATE_LIMIT_SALT`: recommended secret used to hash client IPs before logging
- `TURNSTILE_SECRET`: not enforced yet, but the server hook is already wired so Turnstile can be added without restructuring the API
- `ADMIN_USERNAME`: required to protect the metrics dashboard and admin API
- `ADMIN_PASSWORD`: required to protect the metrics dashboard and admin API

## Local development

Build the Leptos assets:

```bash
cargo leptos build --release
```

Then run Wrangler locally:

```bash
bunx wrangler dev --remote --ip 127.0.0.1 --port 57581
```

Use `--remote` because Workers AI runs in Cloudflare, not in the purely local Worker simulator.

## Deploy

```bash
bunx wrangler deploy
```

Wrangler runs the configured build command:

1. `cargo leptos build --release`
2. `worker-build --release --features ssr`

## How Workers AI is wired

The translation endpoint lives at `/api/translate`.

The internal metrics endpoint lives at `/api/admin/metrics` and is intended to be viewed through `stats.indeknil.com`.

Server flow:

1. Validate `mode`, `intensity`, and input length
2. Apply optional D1-backed rate limiting in [`src/server/rate_limit.rs`](/Users/star/dev/counter-linkedin/src/server/rate_limit.rs)
3. Build a mode-specific prompt in [`src/api.rs`](/Users/star/dev/counter-linkedin/src/api.rs)
4. Call the configured Workers AI model through the `AI` binding in [`src/server/translate.rs`](/Users/star/dev/counter-linkedin/src/server/translate.rs)
5. Return a stable JSON payload:

```json
{
  "output": "…",
  "mode": "linkedin_to_counter_linkedin",
  "intensity": 70,
  "warnings": []
}
```

Error responses return:

```json
{
  "error": {
    "code": "bad_request",
    "message": "Paste something first.",
    "warnings": []
  }
}
```

## Swapping models

Change `WORKERS_AI_MODEL` in [`wrangler.toml`](/Users/star/dev/counter-linkedin/wrangler.toml) or override it per environment with Wrangler vars/secrets.

Keep the model:

- fast enough for an interactive button press
- compatible with prompt-style text generation
- cheap enough to tolerate iterative use

If you switch to a model with a different response schema, update the `AiOutput` struct in [`src/server/translate.rs`](/Users/star/dev/counter-linkedin/src/server/translate.rs).

If you switch to a model with different token pricing, update `AI_INPUT_COST_PER_MILLION_USD` and `AI_OUTPUT_COST_PER_MILLION_USD` in [`wrangler.toml`](/Users/star/dev/counter-linkedin/wrangler.toml) or override them per environment.

## Rate limiting and abuse hooks

Current controls:

- input capped at 4,000 characters
- output trimmed to a compact maximum
- client disables duplicate submissions while a request is in flight
- D1-backed per-IP cooldown and rolling-window throttle
- regenerate goes through the same throttle path as generate

The main hook points for future abuse controls are:

- request validation in [`src/api.rs`](/Users/star/dev/counter-linkedin/src/api.rs)
- rate limiting in [`src/server/rate_limit.rs`](/Users/star/dev/counter-linkedin/src/server/rate_limit.rs)
- Turnstile enforcement path in [`src/server/state.rs`](/Users/star/dev/counter-linkedin/src/server/state.rs) and [`src/server/translate.rs`](/Users/star/dev/counter-linkedin/src/server/translate.rs)

## Non-obvious choices

- The app uses a direct JSON endpoint instead of Leptos server functions for generation so the Worker can inspect headers for IP-based throttling cleanly.
- D1 now stores both the throttle ledger and a full generation ledger: input text, output text, mode, token usage, latency, and estimated cost.
- Cost numbers are estimates computed from Workers AI token usage and the configured per-million prices. They are useful for ops, but they are not a substitute for Cloudflare invoice truth.
- `stats.indeknil.com` is served by the same Worker. The root path is rewritten to `/metrics`, and admin auth is enforced before the dashboard or API load.
- Prompt construction stays in shared Rust code so the UI and server agree on mode names, limits, and output expectations.

## Intentionally out of scope

- accounts
- public gallery
- auth
- payments
- browser extension
- saved history UI
- full Turnstile enforcement
- moderation pipeline beyond the current validation and rate limiting
