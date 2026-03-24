#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSET_STASH_DIR="$ROOT_DIR/target/wrangler-assets"
SITE_DIR="$ROOT_DIR/target/site"
PKG_DIR="$SITE_DIR/pkg"

cd "$ROOT_DIR"

cargo install -q -f wasm-bindgen-cli --version 0.2.114
cargo leptos build --release

rm -rf "$ASSET_STASH_DIR"
mkdir -p "$ASSET_STASH_DIR"
cp -R "$PKG_DIR" "$ASSET_STASH_DIR/pkg"

cargo install -q "worker-build@^0.7"
worker-build --release --features ssr

# worker-build's recovery path may call __wbindgen_start() even when the
# generated server wasm doesn't export it. Guard any direct receiver call so
# production requests don't crash during worker reinitialization.
perl -0pi -e 's/([[:alpha:]_][[:alnum:]_]*)\.__wbindgen_start\(\)/$1.__wbindgen_start&&$1.__wbindgen_start()/g' "$ROOT_DIR/build/index.js"

rm -rf "$PKG_DIR"
mkdir -p "$SITE_DIR"
cp -R "$ASSET_STASH_DIR/pkg" "$PKG_DIR"
