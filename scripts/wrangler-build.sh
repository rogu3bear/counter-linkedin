#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ASSET_STASH_DIR="$ROOT_DIR/target/wrangler-assets"
SITE_DIR="$ROOT_DIR/target/site"
PKG_DIR="$SITE_DIR/pkg"
REQUIRED_WASM_BINDGEN_VERSION="0.2.114"
REQUIRED_WORKER_BUILD_MAJOR="0.7"

cd "$ROOT_DIR"

ensure_cargo_binary() {
  local command_name="$1"
  local expected_version="$2"
  shift 2

  if command -v "$command_name" >/dev/null 2>&1; then
    local current_version
    current_version="$("$command_name" --version 2>/dev/null || true)"
    if [[ "$current_version" == *"$expected_version"* ]]; then
      return
    fi
  fi

  cargo install -q "$@"
}

ensure_cargo_binary wasm-bindgen "$REQUIRED_WASM_BINDGEN_VERSION" -f wasm-bindgen-cli --version "$REQUIRED_WASM_BINDGEN_VERSION"
cargo leptos build --release

rm -rf "$ASSET_STASH_DIR"
mkdir -p "$ASSET_STASH_DIR"
cp -R "$PKG_DIR" "$ASSET_STASH_DIR/pkg"

ensure_cargo_binary worker-build "$REQUIRED_WORKER_BUILD_MAJOR" "worker-build@^${REQUIRED_WORKER_BUILD_MAJOR}"
worker-build --release --features ssr

# worker-build's recovery path may call __wbindgen_start() even when the
# generated server wasm doesn't export it. Guard any direct receiver call so
# production requests don't crash during worker reinitialization.
perl -0pi -e 's/([[:alpha:]_][[:alnum:]_]*)\.__wbindgen_start\(\)/$1.__wbindgen_start&&$1.__wbindgen_start()/g' "$ROOT_DIR/build/index.js"

rm -rf "$PKG_DIR"
mkdir -p "$SITE_DIR"
cp -R "$ASSET_STASH_DIR/pkg" "$PKG_DIR"
