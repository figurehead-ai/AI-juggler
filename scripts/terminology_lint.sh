#!/usr/bin/env bash
# AJ-E108 terminology lint: user-facing legacy nouns must not creep back.
set -uo pipefail
cd "$(git rev-parse --show-toplevel)"
fail=0
check() { # $1=file $2=pattern $3=allow-regex $4=reason
  while IFS= read -r hit; do
    [ -z "${hit:-}" ] && continue
    echo "TERMINOLOGY: $1:$hit — $4"
    fail=1
  done < <(grep -nE "$2" "$1" 2>/dev/null | grep -viE "$3" || true)
}

ALLOW_HARNESS='HarnessId|HarnessAdapter|harness binaries|harness\.state|vendor|unmanaged|glossary|Legacy term|schema v5|code name|adapter-internal|isolated profile directory|test harness'
ALLOW_PROFILE='ProfileInfo|ProfileId|profile_create|profile_delete|[Ll]aunch settings|LaunchRecipe|profiles table|glossary|Legacy term|isolated profile directory|profile: &Profile|launch spec for a profile|profile roots|as-account-glossary-row'
for f in docs/SPEC.md docs/PROPOSAL.md docs/plan/EPICS.md; do
  check "$f" '\bharness(es)?\b' "$ALLOW_HARNESS" "user-facing harness (use Provider)"
  check "$f" '\bprofiles?\b' "$ALLOW_PROFILE" "profile-as-account (use Account/Launch settings)"
done

while IFS= read -r jsx; do
  check "$jsx" '>[^<>{}]*\bharness(es)?[^<>{}]*<' 'vendor|claude' "user-facing harness in JSX"
  check "$jsx" '>[^<>{}]*\b[Pp]rofiles?\b[^<>{}]*<' 'launch|settings|Launch|recipe' "profile-as-account in JSX"
done < <(find ui/src -name "*.tsx" | grep -v types/generated)

[ "$fail" -eq 0 ] && echo "terminology-lint: OK"
exit $fail
