#!/usr/bin/env bash
# 0.4.6 (AJ-E3011's done-gate walks): the UI bundled for a persona walk on a
# lab bridge, so the walk measures the app's speed, not the dev server's
# first-open compiles (2–4 s there, 0–80 ms in the bundled Dev window).
#
#   scripts/qa/walk_bundle.sh build --out /tmp/aij-walk-046/ui-dist [--src <checkout>]
#   scripts/qa/walk_bundle.sh serve --out /tmp/aij-walk-046/ui-dist [--src <checkout>] [--port 5173]
#   scripts/qa/walk_bundle.sh --selftest
#
# A development-mode build keeps the test bridge's client (transport.ts loads
# it only when import.meta.env.DEV). The built page carries the shipping
# Content-Security-Policy (default-src 'self'), which blocks the page from
# reaching the bridge on 127.0.0.1; only THIS throwaway copy gets
# `connect-src 'self' http://127.0.0.1:*`. The shipping CSP can't be relaxed
# by this script:
# - the output must be under /tmp/aij-walk-* (anything else is refused);
# - the repo's ui/index.html and tauri.conf.json CSPs (and ui/dist, if
#   any) are fingerprinted before the build and must be unchanged after it.
#
# Load the page with a cache-busting parameter, or the browser keeps the
# old index.html: http://localhost:5173/?devBridge=<port>&fresh=<n>#token=…
set -euo pipefail

here="$(cd "$(dirname "$0")/../.." && pwd)"
cmd="${1:-}"
[ $# -gt 0 ] && shift
src="$here"
out=""
port=5173
while [ $# -gt 0 ]; do
  case "$1" in
    --out) out="$2"; shift 2 ;;
    --src) src="$2"; shift 2 ;;
    --port) port="$2"; shift 2 ;;
    *) echo "walk-bundle: unknown argument $1" >&2; exit 2 ;;
  esac
done

# The output folder: a new or existing folder under /tmp/aij-walk-*, never a
# link, never anywhere else.
check_out() {
  local path="$1"
  [ -n "$path" ] || { echo "walk-bundle: --out is required" >&2; return 1; }
  case "$path" in
    /tmp/aij-walk-*/*|/private/tmp/aij-walk-*/*) ;;
    *) echo "walk-bundle: refused: the output must be under /tmp/aij-walk-* (got $path)" >&2; return 1 ;;
  esac
  case "$path" in
    *..*) echo "walk-bundle: refused: no '..' in the output path" >&2; return 1 ;;
  esac
  if [ -L "$path" ]; then
    echo "walk-bundle: refused: the output is a link" >&2; return 1
  fi
  local parent
  parent="$(dirname "$path")"
  mkdir -p "$parent"
  case "$(cd "$parent" && pwd -P)" in
    /private/tmp/aij-walk-*|/tmp/aij-walk-*) ;;
    *) echo "walk-bundle: refused: the output's folder resolves outside /tmp/aij-walk-*" >&2; return 1 ;;
  esac
}

# The shipping CSPs (and the repo's own build, if any), as one fingerprint.
fingerprint() {
  {
    grep -o 'Content-Security-Policy[^>]*' "$src/ui/index.html" || true
    grep -o '"csp": *"[^"]*"' "$src/crates/juggler-app/tauri.conf.json" || true
    if [ -d "$src/ui/dist" ]; then
      find "$src/ui/dist" -type f -print0 | sort -z | xargs -0 shasum
    fi
  } | shasum | cut -d' ' -f1
}

# Widen connect-src in the copy's index.html, once, or fail.
widen() {
  local page="$1/index.html"
  local n
  n="$(grep -c 'http-equiv="Content-Security-Policy"' "$page" || true)"
  [ "$n" = "1" ] || { echo "walk-bundle: expected one CSP meta in $page, found $n" >&2; return 1; }
  if grep -q "connect-src" "$page"; then
    echo "walk-bundle: the copy's CSP already names connect-src: left as it is" >&2
    return 0
  fi
  python3 -I - "$page" <<'EOF'
import re, sys
path = sys.argv[1]
text = open(path, encoding="utf-8").read()
pattern = re.compile(r'(http-equiv="Content-Security-Policy" content=")([^"]*)(")')
new, count = pattern.subn(lambda m: m.group(1) + m.group(2) + "; connect-src 'self' http://127.0.0.1:*" + m.group(3), text)
if count != 1:
    sys.exit(f"walk-bundle: the CSP meta wasn't changed exactly once ({count})")
open(path, "w", encoding="utf-8").write(new)
EOF
}

case "$cmd" in
  build)
    check_out "$out"
    before="$(fingerprint)"
    (cd "$src/ui" && NODE_ENV=development pnpm exec vite build --mode development --outDir "$out" --emptyOutDir > /dev/null)
    widen "$out"
    after="$(fingerprint)"
    if [ "$before" != "$after" ]; then
      echo "walk-bundle: the repo's shipping CSP or its own build changed during the build: stopping" >&2
      exit 1
    fi
    grep -q "devBridge" "$out"/assets/*.js || { echo "walk-bundle: the bundle has no test-bridge client" >&2; exit 1; }
    echo "walk-bundle: built $out (the shipping CSPs unchanged); serve it with: $0 serve --out $out"
    ;;
  serve)
    check_out "$out"
    [ -f "$out/index.html" ] || { echo "walk-bundle: nothing built at $out" >&2; exit 1; }
    exec pnpm --dir "$src/ui" exec vite preview --outDir "$out" --port "$port" --strictPort
    ;;
  --selftest)
    ok=0
    for bad in "/tmp/elsewhere/ui-dist" "/Users/x/ui-dist" "/tmp/aij-walk-1/../x" "ui/dist" ""; do
      if (check_out "$bad") 2> /dev/null; then echo "selftest: $bad was accepted" >&2; ok=1; fi
    done
    tmp="$(mktemp -d /tmp/aij-walk-selftest.XXXXXX)"
    trap 'rm -rf "$tmp"' EXIT
    mkdir -p "$tmp/dist"
    printf '<meta http-equiv="Content-Security-Policy" content="default-src '"'"'self'"'"'" />\n' > "$tmp/dist/index.html"
    widen "$tmp/dist"
    grep -q "connect-src 'self' http://127.0.0.1:\*" "$tmp/dist/index.html" || { echo "selftest: connect-src not added" >&2; ok=1; }
    if (check_out "$tmp/dist") 2> /dev/null; then :; else echo "selftest: $tmp/dist was refused" >&2; ok=1; fi
    [ "$ok" = 0 ] && echo "walk-bundle selftest: ok"
    exit "$ok"
    ;;
  *)
    echo "usage: $0 build|serve --out /tmp/aij-walk-*/… [--src <checkout>] [--port 5173] | --selftest" >&2
    exit 2
    ;;
esac
