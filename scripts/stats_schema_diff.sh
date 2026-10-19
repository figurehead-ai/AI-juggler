#!/usr/bin/env bash
# AJ-E1101: the usage-statistics schema must match the copy the server
# enforces, byte for byte. That copy lives in the private website repo,
# which public CI can't read, so this runs locally before each release
# (docs/RELEASING.md). Usage: FH_SITE=<website clone> scripts/stats_schema_diff.sh
set -euo pipefail
site="${FH_SITE:?set FH_SITE to the website repo clone}"
server="$site/workers/stats/schema.v1.json"
[ -f "$server" ] || { echo "stats-schema: $server not found (pull the website repo)" >&2; exit 1; }
if cmp -s stats/schema.v1.json "$server"; then
  echo "stats-schema: identical ($(shasum -a 256 stats/schema.v1.json | cut -c1-16))"
else
  echo "stats-schema: DIFFERENT — the server would reject what this build sends:" >&2
  diff -u "$server" stats/schema.v1.json >&2 || true
  exit 1
fi
