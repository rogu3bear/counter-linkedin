# AGENTS.md

CounterLinkedIn is now a memorial/static site. Keep doctrine short and do not document the retired inference stack as active runtime.

## Canonical Commands

- Toolchain bootstrap: `./scripts/bootstrap.sh`
- Static build: `cargo leptos build --release`
- Worker bundle build: `./scripts/wrangler-build.sh`
- Local worker preview: `bunx wrangler dev --remote --ip 127.0.0.1 --port 57581`
- Cloudflare source-config standards audit: `cfctl standards audit .`

## Guardrails

- The current product is the memorial page, not the former translation app.
- Treat Workers AI, D1, and Turnstile as retired unless the code explicitly revives them.
- Keep wording grounded in the current static site and checked-in Rust crate.
- `scripts/bootstrap.sh` still carries historical dependency and local-worker setup. Do not mistake that for proof that the retired product surface is active again.
- Use `cfctl` from `PATH` for live Cloudflare/account reads, mutation planning, and post-change verification. Keep direct Wrangler usage limited to the repo's static Worker build/dev loop unless a task explicitly revives deploy work.
