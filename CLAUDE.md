# CLAUDE.md

CounterLinkedIn is a sunsetted Leptos + Cloudflare Workers project whose active surface is the memorial page.

## Core Commands

```bash
./scripts/bootstrap.sh
cargo leptos build --release
./scripts/wrangler-build.sh
bunx wrangler dev --remote --ip 127.0.0.1 --port 57581
```

## Working Notes

- Build and runtime guidance should reflect the memorial/static mode.
- Do not write docs that imply active paid inference, D1 logging, or live translation flows unless the code brings them back.
- Treat the bootstrap script as environment setup and historical tooling support, not as evidence that the retired product stack is live.
