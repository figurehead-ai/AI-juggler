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

# A lowercase "provider" in code — `session.provider`, `provider < 3`,
# `fn(provider)` — is an identifier, not words on screen.
CODE_PROVIDER='[.(]providers?\b|\bproviders?[[:space:]]*([<>=!&|?)]|\]|\.[A-Za-z_(]|\()'

jsx_checks() { # $1=file
  local jsx=$1
  check "$jsx" '>[^<>{}]*\bharness(es)?[^<>{}]*<' 'vendor|claude' "user-facing harness in JSX"
  # "Claude (desktop) profiles" names the profile folders themselves (a
  # profile manager's ClaudeCodeProfiles), not accounts.
  check "$jsx" '>[^<>{}]*\b[Pp]rofiles?\b[^<>{}]*<' 'launch|settings|Launch|recipe|desktop|Claude profiles' "profile-as-account in JSX"
  # D-25: on screen it is a "session" (the code keeps Instance).
  check "$jsx" '>[^<>{}]*\b[Ii]nstances?\b[^<>{}]*<' 'NONE^' "Instance on screen in JSX (say session)"
  check "$jsx" '(aria-label|title|placeholder|label|alt)=("|\{`)[^"`$]*\b[Ii]nstances?\b' 'NONE^' "Instance in a visible attribute (say session)"
  # D-25: on screen it is the tool (Claude Code, Codex); the code keeps Provider.
  check "$jsx" '>[^<>{}]*\bProviders?\b[^<>{}]*<' 'NONE^' "Provider on screen in JSX (say tool)"
  check "$jsx" '>[^<>{}]*\bproviders?\b[^<>{}]*<' "$CODE_PROVIDER" "provider on screen in JSX (say tool)"
  check "$jsx" '^\s*[A-Z][^=;{}<>]*\b[Pp]roviders?\b[^=;{}<>]*$' "^[0-9]+:\\s*(//|\\*|/\\*)|AJ-E|D-[0-9]|$CODE_PROVIDER" "Provider in JSX text (say tool)"
  check "$jsx" '(aria-label|title|placeholder|label|alt|reason|hint)=("|\{`)[^"`$]*\b[Pp]roviders?\b' 'NONE^' "Provider in a visible attribute (say tool)"
  case "$jsx" in *.test.tsx) ;; *)
    check "$jsx" '"[^"]*\b(a|the|each|all|other|this|another|every|per) [Pp]roviders?\b[^"]*"' '^[0-9]+:\s*(//|\*|/\*)' "Provider in a user-facing string (say tool)" ;;
  esac
}

# Self-test: the rules flag words on screen and leave code alone.
selftest() {
  local dir hits
  dir="$(mktemp -d)"
  cat > "$dir/bad.tsx" <<'BAD'
<p className="text-xs">No accounts on this provider yet.</p>
<p>Each Provider has its own sign-in.</p>
          Pick the provider you use most
BAD
  cat > "$dir/good.tsx" <<'GOOD'
{list.length > 0 && provider < 3 ? <b>x</b> : null}
          Boolean(session.provider)
<span title={providerDisplayName(row.provider)}>Tool</span>
<b>{row.provider}</b>
GOOD
  hits="$(fail=0; jsx_checks "$dir/bad.tsx"; true)"
  if [ "$(printf '%s\n' "$hits" | grep -c '^TERMINOLOGY:')" -ne 3 ]; then
    echo "terminology-lint selftest: expected 3 hits in bad.tsx, got:"; printf '%s\n' "$hits"; rm -rf "$dir"; exit 2
  fi
  hits="$(fail=0; jsx_checks "$dir/good.tsx"; true)"
  if [ -n "$hits" ]; then
    echo "terminology-lint selftest: code flagged as words:"; printf '%s\n' "$hits"; rm -rf "$dir"; exit 2
  fi
  rm -rf "$dir"
}
selftest

while IFS= read -r jsx; do
  jsx_checks "$jsx"
done < <(find ui/src -name "*.tsx" | grep -v types/generated)

[ "$fail" -eq 0 ] && echo "terminology-lint: OK"
exit $fail
