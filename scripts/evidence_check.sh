#!/usr/bin/env bash
# AJ-E801 evidence-store gate: link resolution, secret scan, index freshness.
# Exits non-zero on any finding. Run via `just evidence` (part of CI verify).
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0

# --- 1) every gate doc's evidence links resolve inside the store ----------
for doc in docs/qa/gate-*.md docs/qa/beta-exit.md docs/qa/evidence/*/manifest.md; do
  [ -f "$doc" ] || continue
  while IFS= read -r link; do
    target="${link#./}"
    if [[ "$target" == docs/qa/evidence/* ]] && [ ! -e "$target" ]; then
      echo "BROKEN EVIDENCE LINK: $doc -> $link"; fail=1
    fi
  done < <(grep -oE '\((docs/qa/evidence/[^)#]+)\)' "$doc" | tr -d '()')
done

# --- 2) secret scan over the evidence store -------------------------------
patterns='(sk-[A-Za-z0-9]{16,}|AKIA[0-9A-Z]{16}|-----BEGIN [A-Z ]*PRIVATE KEY-----|ghp_[A-Za-z0-9]{30,}|eyJhbGciOi[A-Za-z0-9._-]{40,})'
while IFS= read -r hit; do
  echo "SECRET PATTERN: $hit"; fail=1
done < <(grep -rInE "$patterns" docs/qa/evidence/ 2>/dev/null || true)

# --- 3) _index.md freshness: every manifest dir appears in the index ------
for d in docs/qa/evidence/*/; do
  id="$(basename "$d")"
  [ "$id" = _template ] && continue
  [ "$id" = _gallery ] && continue
  if ! grep -q "^| $id " docs/qa/evidence/_index.md 2>/dev/null; then
    echo "MISSING FROM _index.md: $id (regenerate: scripts/evidence_index.sh)"; fail=1
  fi
done

[ "$fail" -eq 0 ] && echo "evidence-check: OK"
exit $fail
