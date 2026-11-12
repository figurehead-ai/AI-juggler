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
  # Each diff's before and after (a screenshot, or an accessibility
  # tree), named by component, variant and target, for review. Happo gives
  # them as paths on its own host.
  python3 - "$out/results.json" "${api%/api}" > "$out/urls.txt" <<'PY'
import json, re, sys
results, host = json.load(open(sys.argv[1])), sys.argv[2]
for n, pair in enumerate(results.get("diffs", []), 1):
    for side, snap in zip(("before", "after"), pair):
        url = snap.get("url") or ""
        if url.startswith("/"):
            url = host + url
        name = re.sub(r"[^A-Za-z0-9._-]+", "-", f"{n:02d}-{snap.get('component')}-{snap.get('variant')}-{snap.get('target')}-{side}")
        ext = ".json" if url.split("?")[0].endswith(".json") else ".png"
        print(f"{name}{ext}\t{url}")
print(f"{len(results.get('diffs', []))} diff(s), {len(results.get('unchanged', []))} unchanged", file=sys.stderr)
PY
  n=0
  while IFS=$'\t' read -r name url; do
    n=$((n + 1))
    happo "$url" -o "$out/$name" || echo "happo-resolve: couldn't fetch $name" >&2
  done < "$out/urls.txt"
  echo "happo-resolve: $n file(s) and results.json in $out; look at every one, then run again with --resolve success|failure"
  exit 0
fi

happo -X POST -H 'Content-Type: application/json' -d "{\"resolution\":\"$resolution\"}" "$compare/resolve" > /dev/null
echo "happo-resolve: $base..$head resolved as $resolution"
