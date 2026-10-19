#!/usr/bin/env bash
# AJ-E810: lists research documents whose newest "Research date:" (or
# "Accessed") is more than 90 days old. Exit 1 when any is stale, so a
# gate review or release checklist can require a re-verification first.
set -euo pipefail
cd "$(dirname "$0")/.."
today=$(date -u +%s)
stale=0
for doc in docs/research/*.md; do
  latest=$(grep -Eo '(Research date|Accessed|research date)[^0-9]*[0-9]{4}-[0-9]{2}-[0-9]{2}' "$doc" \
    | grep -Eo '[0-9]{4}-[0-9]{2}-[0-9]{2}' | sort | tail -1 || true)
  [ -z "$latest" ] && continue
  when=$(date -u -j -f "%Y-%m-%d" "$latest" +%s 2>/dev/null || date -u -d "$latest" +%s)
  age=$(( (today - when) / 86400 ))
  if [ "$age" -gt 90 ]; then
    echo "STALE ($age days): $doc — last $latest"
    stale=1
  fi
done
[ "$stale" -eq 0 ] && echo "research-staleness: nothing older than 90 days"
exit "$stale"
