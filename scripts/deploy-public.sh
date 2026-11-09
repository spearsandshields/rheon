#!/usr/bin/env bash
#
# deploy-public.sh — build the playground and publish it to the PUBLIC dsntk-wasm-build
# repo, served by GitHub Pages at https://spearsandshields.github.io/dsntk-wasm-build/.
# (The build repository predates the rheon name; renaming it changes the public URL, so
# it is a separate decision. Set DSNTK_BUILD_REPO / edit PAGES_URL when that happens.)
#
# Same shape as parasma's deploy: the public repo holds ONLY the built static site, as a
# single parentless commit that is force-pushed (no source, no history). Pages serves the
# `main` branch root; `.nojekyll` keeps Jekyll away from the hashed assets.
#
# The default is a DRY RUN (build + sanity gate, no push). Re-run with --push to publish.
#
# Usage:
#   scripts/deploy-public.sh           # dry run
#   scripts/deploy-public.sh --push    # build + gate + single-commit force-push
#
# Env:
#   DSNTK_BUILD_REPO   the dsntk-wasm-build checkout (default ~/dev/dsntk-wasm-build)
#
set -euo pipefail

PUSH=0
[ "${1:-}" = "--push" ] && PUSH=1

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="$ROOT/apps/playground/dist"
BUILD="${DSNTK_BUILD_REPO:-$HOME/dev/dsntk-wasm-build}"
PAGES_URL="https://spearsandshields.github.io/dsntk-wasm-build/"
SRC_REV="$(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo uncommitted)"
WASM_SHA="$(shasum -a256 "$ROOT/dist/feel.wasm" | cut -c1-12)"

say()  { printf '\n\033[1m== %s ==\033[0m\n' "$*"; }
fail() { printf '\033[31m  ERROR: %s\033[0m\n' "$*"; exit 1; }

# ── sanity gate (fail-closed) ───────────────────────────────────────────────
# 1. the site is complete: index.html, one feel.wasm asset, the worker chunk, the notices
# 2. nothing machine-specific leaked into the deploy (absolute home paths, source maps are fine)
gate() {
  local dir="$1" label="$2"
  [ -f "$dir/index.html" ] || fail "$label: index.html missing"
  [ "$(find "$dir" -name 'feel*.wasm' | wc -l | tr -d ' ')" = 1 ] || fail "$label: expected exactly one feel.wasm asset"
  find "$dir" -name 'worker*.js' | grep -q . || fail "$label: worker chunk missing"
  [ -f "$dir/third-party-notices.txt" ] || fail "$label: third-party-notices.txt missing"
  if grep -rIl --exclude='*.map' --exclude-dir=.git "/Users/" "$dir" >/dev/null 2>&1; then
    fail "$label: a deployed file contains a local /Users/ path"
  fi
  # vite base './' → every asset reference in index.html must be relative
  if grep -Eq '(src|href)="/' "$dir/index.html"; then
    fail "$label: index.html has root-absolute asset URLs (vite base must stay './')"
  fi
  echo "  gate clean ($label): $(find "$dir" -type f -not -path '*/.git/*' | wc -l | tr -d ' ') files, $(du -sh "$dir" | cut -f1)"
}

say "Build runtime + playground (source $SRC_REV, feel.wasm $WASM_SHA)"
cd "$ROOT"
[ -f dist/feel.wasm ] || fail "dist/feel.wasm missing: run npm run build:wasm first"
npm run build:runtime >/dev/null
rm -rf "$DIST"
npm run build:playground >/dev/null

say "Gate: built dist"
gate "$DIST" dist

if [ "$PUSH" = 0 ]; then
  say "DRY RUN OK — re-run with --push to publish to $PAGES_URL"
  exit 0
fi

[ -d "$BUILD/.git" ] || fail "$BUILD is not a git repo (clone spearsandshields/dsntk-wasm-build there, or set DSNTK_BUILD_REPO)"

say "Stage into $BUILD"
cd "$BUILD"
git checkout -f main >/dev/null 2>&1 || git checkout --orphan main >/dev/null 2>&1
find . -mindepth 1 -maxdepth 1 -not -name '.git' -exec rm -rf {} +
cp -r "$DIST"/. .
touch .nojekyll
# Build artifact only: source maps embed the TypeScript sources, so they stay out of the public repo.
find . -name '*.map' -not -path './.git/*' -delete
find . -name '.DS_Store' -not -path './.git/*' -delete

say "Gate: staged public tree"
gate "$BUILD" staged

say "Single-commit force-push to dsntk-wasm-build"
git add -A
TREE="$(git write-tree)"
COMMIT="$(git commit-tree "$TREE" -m "dsntk FEEL playground — live demo (source $SRC_REV, feel.wasm $WASM_SHA)")"
git reset --hard "$COMMIT" >/dev/null
git push origin main --force
echo "  DEPLOYED — live shortly at $PAGES_URL"
