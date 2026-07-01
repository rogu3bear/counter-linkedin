# CounterLinkedIn

**2025 – 2026**

CounterLinkedIn was a single-screen web app that translated polished professional language into brutally honest plain English — and back again when needed.

It ran on Leptos + Cloudflare Workers + Workers AI. The AI bill came due. The joke did not generate revenue. The site has been sunsetted — which, in LinkedIn speak, means *we are excited to announce a strategic pivot toward not paying for inference.*

The memorial page lives at [counterlinkedin.com](https://counterlinkedin.com).

## What it did

Three translation modes:

- **LinkedIn → CounterLinkedIn** — polished update in, career-risk honesty out
- **Raw → LinkedIn** — blunt draft in, status-safe rewrite out
- **Job Post → Honest** — recruiter copy in, grounded subtext out

## Stack

- **Rust** — single crate, Leptos 0.8 SSR + hydration
- **Cloudflare Workers** — edge runtime
- **Workers AI** — Llama 3.1 8B for generation (now disabled)
- **D1** — rate limiting, generation logging, spend estimation (now removed)
- **Turnstile** — human check gate (now removed)

## Running locally

The memorial page is a static Leptos app. To build it:

```bash
rustup target add wasm32-unknown-unknown
cargo leptos build --release
bunx wrangler dev
```

For live Cloudflare/account checks, use `cfctl` from `PATH` rather than ad hoc
API calls:

```bash
cfctl standards audit .
```

## License

[MIT](LICENSE)
