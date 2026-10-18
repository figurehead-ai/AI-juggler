#!/usr/bin/env bash
# Regenerate docs/qa/evidence/_index.md from the manifests (AJ-E801 Req.5).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
out=docs/qa/evidence/_index.md
{
  echo "# Evidence index (generated — do not edit; run scripts/evidence_index.sh)"
  echo
  echo "| Task | Verdicts | Last date | Manifest |"
  echo "|---|---|---|---|"
  for d in docs/qa/evidence/*/; do
    id="$(basename "$d")"
    [ "$id" = _template ] && continue
    [ "$id" = _gallery ] && continue
    m="$d/manifest.md"
    if [ -f "$m" ]; then
      verdicts=$( { grep -oE '\| (PASS|FAIL|UNPROVEN) ' "$m" || true; } | sort | uniq -c | tr -d '|' | awk '{printf "%s%s ", $2, $1}' | sed 's/ *$//')
      [ -z "$verdicts" ] && verdicts="(no claims yet)"
      last=$( { grep -oE '[0-9]{4}-[0-9]{2}-[0-9]{2}' "$m" || true; } | tail -1)
      echo "| $id | $verdicts | ${last:-—} | [manifest](${d#docs/qa/evidence/}manifest.md) |"
    else
      echo "| $id | (directory without manifest — add one) | — | — |"
    fi
  done
} > "$out"
echo "generated $out"
