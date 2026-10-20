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

# D-25: no task, epic or decision codes on screen ("lands with AJ-E606+",
# "E3+", "(E2)", "audit-logged (D-08)"). Comments may cite them; what the
# user reads may not.
TASKCODE='(\bAJ-E[0-9]+|\bE[0-9]+\+|\(E[0-9]+\)|\bD-[0-9]{2}\b)'
COMMENT_LINE='^[0-9]+:\s*(//|\*|/\*|\{/\*)'

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
  check "$jsx" ">[^<>{}]*${TASKCODE}[^<>{}]*<" "$COMMENT_LINE" "task code on screen in JSX (say it in plain words)"
  check "$jsx" "(aria-label|title|placeholder|label|alt|reason|hint)=(\"|\\{\`)[^\"\`\$]*${TASKCODE}" 'NONE^' "task code in a visible attribute (say it in plain words)"
  # JSX text that wraps: a bare line of words between tags.
  check "$jsx" "^\\s*[A-Za-z(][^=;{}<>\"\`]*${TASKCODE}[^=;{}<>]*\$" "$COMMENT_LINE" "task code in JSX text (say it in plain words)"
  case "$jsx" in *.test.tsx) ;; *)
    check "$jsx" '"[^"]*\b(a|the|each|all|other|this|another|every|per) [Pp]roviders?\b[^"]*"' '^[0-9]+:\s*(//|\*|/\*)' "Provider in a user-facing string (say tool)"
    check "$jsx" "\"[^\"]*${TASKCODE}[^\"]*\"|\`[^\`]*${TASKCODE}[^\`]*\`" "$COMMENT_LINE" "task code in a user-facing string (say it in plain words)" ;;
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
<p className="text-[10px] text-muted">lands with AJ-E606+ (two-clock usage engine)</p>
<ActionRow label="Rename" reason="account settings land with E3+" />
const why = "handovers render from Switch Graph edges (E2)";
          and every kill is audit-logged (D-08). Crash tidies wait 5 minutes
BAD
  cat > "$dir/good.tsx" <<'GOOD'
{list.length > 0 && provider < 3 ? <b>x</b> : null}
          Boolean(session.provider)
<span title={providerDisplayName(row.provider)}>Tool</span>
<b>{row.provider}</b>
        {/* AJ-E402 Req.4: the card variant — a comment may cite a task. */}
// AJ-E1005: the morning question first.
  const tone = ready ? "border-hairline" : "border-hairline/50"; // AJ-E804: no opacity
        {/* D-25: plain words on screen; a comment may cite a decision. */}
 * AJ-E413 Req.3 (D-06): a doc comment may cite both.
GOOD
  hits="$(fail=0; jsx_checks "$dir/bad.tsx"; true)"
  # Every line of bad.tsx must be flagged (by one rule or more).
  if [ "$(printf '%s\n' "$hits" | grep -oE 'bad\.tsx:[0-9]+:' | sort -u | wc -l | tr -d ' ')" -ne 7 ]; then
    echo "terminology-lint selftest: expected all 7 lines of bad.tsx flagged, got:"; printf '%s\n' "$hits"; rm -rf "$dir"; exit 2
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
