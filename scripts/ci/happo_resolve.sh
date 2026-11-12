#!/usr/bin/env bash
# 0.4.3: Happo's visual review of a release, resolved from this Mac after
# every diff was looked at by eye (as our PNG baselines are). Happo stays
# advisory (never a required check). Never run in CI: its key lives only in
# the maintainer's Keychain (item -a happo -s aij-happo, `apiKey:apiSecret`)
# and never reaches GitHub, a log or a command line.
#
#   scripts/ci/happo_resolve.sh <base-sha> <head-sha>
#       fetches the comparison and saves its diff images, with the JSON,
#       to $HAPPO_OUT (default: $TMPDIR/happo-<head>); look at every one
#   scripts/ci/happo_resolve.sh <base-sha> <head-sha> --resolve success|failure
#       resolves it: success when every diff is intended, failure when one
#       is a real regression
#
# With no Keychain item it says so and does nothing (exit 0).
set -euo pipefail

usage() { sed -n '2,16p' "$0" >&2; exit 2; }
base="${1:-}"; head="${2:-}"
[ -n "$base" ] && [ -n "$head" ] || usage
[[ "$base" =~ ^[0-9a-f]{7,40}$ && "$head" =~ ^[0-9a-f]{7,40}$ ]] || { echo "happo-resolve: base and head are commit shas" >&2; exit 2; }
resolution=""
if [ "${3:-}" = "--resolve" ]; then
  resolution="${4:-}"
  [ "$resolution" = "success" ] || [ "$resolution" = "failure" ] || usage
elif [ -n "${3:-}" ]; then
  usage
fi

api="${HAPPO_API:-https://happo.io/api}"
compare="$api/reports/$base/compare/$head"

key="$(security find-generic-password -a happo -s aij-happo -w 2>/dev/null || true)"
if [ -z "$key" ]; then
  echo "happo-resolve: no Happo key in the Keychain (-a happo -s aij-happo); nothing done"
  exit 0
fi
# curl reads the credentials from its config on stdin: never in argv (ps),
# never echoed. `printf` is a shell builtin.
happo() {
  printf 'user = "%s"\n' "$key" | curl --silent --show-error --fail-with-body --config - "$@"
}
trap 'unset key' EXIT

if [ -z "$resolution" ]; then
  out="${HAPPO_OUT:-${TMPDIR:-/tmp}/happo-$head}"
  mkdir -p "$out"
  happo "$compare/results" -o "$out/results.json"
  # Every image URL in the results, saved under its own name for review.
  python3 - "$out/results.json" > "$out/urls.txt" <<'PY'
import json, sys
seen = set()
def walk(value, path=""):
    if isinstance(value, dict):
        for k, v in value.items():
            walk(v, f"{path}/{k}")
    elif isinstance(value, list):
        for i, v in enumerate(value):
            walk(v, f"{path}/{i}")
    elif isinstance(value, str) and value.startswith("https://") and (".png" in value or "image" in path.lower() or path.endswith("/url")):
        if value not in seen:
            seen.add(value)
            print(value)
print(f"{len(seen)} image(s)", file=sys.stderr)
PY
  n=0
  while IFS= read -r url; do
    n=$((n + 1))
    name="$(printf '%04d-%s' "$n" "$(basename "${url%%\?*}")")"
    happo "$url" -o "$out/$name" || echo "happo-resolve: couldn't fetch image $n" >&2
  done < "$out/urls.txt"
  echo "happo-resolve: $n image(s) and results.json in $out; look at every one, then run again with --resolve success|failure"
  exit 0
fi

happo -X POST -H 'Content-Type: application/json' -d "{\"resolution\":\"$resolution\"}" "$compare/resolve" > /dev/null
echo "happo-resolve: $base..$head resolved as $resolution"
