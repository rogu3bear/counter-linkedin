#!/usr/bin/env bash
set -euo pipefail
# Sync the GitHub Actions Cloudflare secrets used by this repo.
#
# Required inputs can be exported directly or loaded from optional env files:
#   CLOUDFLARE_ACCOUNT_ID
#   CLOUDFLARE_API_TOKEN, COUNTER_CLOUDFLARE_API_TOKEN, or CF_DEV_TOKEN
#
# Optional env files, when present:
#   $HOME/dev/.env
#   ./.env
#
# Usage: ./scripts/rotate-secrets.sh [--dry-run]

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
DRY_RUN=false
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=true

for env_file in "$HOME/dev/.env" "$REPO_ROOT/.env"; do
  if [[ -f "$env_file" ]]; then
    set -a
    # shellcheck source=/dev/null
    source "$env_file"
    set +a
  fi
done

TOKEN="${COUNTER_CLOUDFLARE_API_TOKEN:-${CLOUDFLARE_API_TOKEN:-${CF_DEV_TOKEN:-}}}"

[[ -n "$TOKEN" ]] || {
  echo "Error: set CLOUDFLARE_API_TOKEN, COUNTER_CLOUDFLARE_API_TOKEN, or CF_DEV_TOKEN." >&2
  exit 1
}
[[ -n "${CLOUDFLARE_ACCOUNT_ID:-}" ]] || {
  echo "Error: CLOUDFLARE_ACCOUNT_ID not set." >&2
  exit 1
}

GH_REPO="rogu3bear/counter-linkedin"

put_gh() {
  local name="$1" value="$2"
  if [[ "$DRY_RUN" == "true" ]]; then
    echo "  [dry-run] $name (len=${#value})"
    return
  fi
  gh secret set "$name" --repo "$GH_REPO" --body "$value" 2>/dev/null
  echo "  $name"
}

echo "COUNTER secret rotation (gh=$GH_REPO)"
[[ "$DRY_RUN" == "true" ]] && echo "  mode: dry-run"
echo ""

put_gh CLOUDFLARE_ACCOUNT_ID "$CLOUDFLARE_ACCOUNT_ID"
put_gh CLOUDFLARE_API_TOKEN "$TOKEN"

echo ""
echo "Done. GH '$GH_REPO' secrets are in sync."
