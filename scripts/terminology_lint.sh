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
# The same, with a case-sensitive allow-regex (code is lowercase; the
# words on screen may not be).
check_cs() { # $1=file $2=pattern $3=allow-regex $4=reason
  while IFS= read -r hit; do
    [ -z "${hit:-}" ] && continue
    echo "TERMINOLOGY: $1:$hit — $4"
    fail=1
  done < <(grep -nE "$2" "$1" 2>/dev/null | grep -vE "$3" || true)
}

ALLOW_HARNESS='HarnessId|HarnessAdapter|harness binaries|harness\.state|vendor|unmanaged|glossary|Legacy term|schema v5|code name|adapter-internal|isolated profile directory|test harness'
ALLOW_PROFILE='ProfileInfo|ProfileId|profile_create|profile_delete|[Ll]aunch settings|LaunchRecipe|profiles table|glossary|Legacy term|isolated profile directory|profile: &Profile|launch spec for a profile|profile roots|as-account-glossary-row'
for f in docs/SPEC.md docs/PROPOSAL.md docs/plan/EPICS.md; do
  check "$f" '\bharness(es)?\b' "$ALLOW_HARNESS" "user-facing harness (use Provider)"
  check "$f" '\bprofiles?\b' "$ALLOW_PROFILE" "profile-as-account (use Account/Launch settings)"
done

# A lowercase "provider" in code — `session.provider`, `provider.id`,
# `fn(provider)`, `fn(a, provider)`, `{provider}`, `[provider]`,
# `provider={…}`, `provider < 3`, `provider === "codex" ? …` — is an
# identifier, not words on screen. Only those shapes: "Which provider?"
# and "this provider!" are words.
CODE_PROVIDER='\.providers?\b|\bproviders?\.[A-Za-z_]|\(providers?[,)]|,[[:space:]]*providers?[[:space:]]*[,)]|\{providers?\}|\[providers?\]|\bproviders?=[{"]|\bproviders?[[:space:]]+(<|>|<=|>=|===|!==|==|!=|&&|\|\||\?\?|\?)[[:space:]]'

# D-25: no task, epic or decision codes on screen ("lands with AJ-E606+",
# "E3+", "(E2)", "lands with E2", "audit-logged (D-08)", "D-7").
# Comments may cite them; what the user reads may not.
TASKCODE='(\bAJ-E[0-9]+|\bE[0-9]+\+|\(E[0-9]+\)|\bE[0-9]+\b|\bD-[0-9]{1,2}\b)'
COMMENT_LINE='^[0-9]+:\s*(//|\*|/\*|\{/\*)'

# String literals ("…", '…', `…`) that carry a code. A tokenizer, not a
# grep, over the whole file: it skips // and /* */ comments (so
# `f("a"); // AJ-E1: "b"` is clean) and regex literals (so a `"` inside
# /…/ opens no string), reads each literal whole (a template literal may
# span lines), leaves apostrophes in words alone, and ignores attributes
# nobody reads (data-testid, key, id, className…).
string_codes() { # $1=file
  while IFS= read -r hit; do
    [ -z "${hit:-}" ] && continue
    echo "TERMINOLOGY: $1:$hit — task code in a user-facing string (say it in plain words)"
    fail=1
  done < <(perl -0777 -ne '
    my $src = $_;
    my @lines = split /\n/, $src, -1;
    my %seen;
    while ($src =~ m{
        (/\*.*?\*/ | //[^\n]*)                                    # 1: comment
      | ((?:(?<=[(,=:\[!&|?{};\n])|(?<=return)|\A)[ \t]*
          /(?![/*>])(?:[^/\\\n\[]|\\.|\[(?:[^\]\\\n]|\\.)*\])+/[a-z]*)  # 2: regex literal
      | (?:\b([\w-]+)=)?                                          # 3: attribute
        ( "(?:[^"\\\n]|\\.)*"
        | (?<![A-Za-z])\x27(?:[^\x27\\\n]|\\.)*\x27(?![A-Za-z])
        | `(?:[^`\\]|\\.)*` )                                     # 4: string
      }gsx) {
      next if defined $1 || defined $2;
      my ($attr, $lit, $at) = ($3 // "", $4, $-[4]);
      next if $attr =~ /^(data-[\w-]+|key|id|className|htmlFor|role|type|name|href|src|ref|testId)$/;
      next unless $lit =~ /\bAJ-E\d+|\bE\d+\+|\(E\d+\)|\bE\d+\b|\bD-\d{1,2}\b/;
      # The line the code itself is on (a template literal may start earlier).
      my $off = $at + $-[0] if $lit =~ /\bAJ-E\d+|\bE\d+\+|\(E\d+\)|\bE\d+\b|\bD-\d{1,2}\b/;
      my $ln = 1 + (substr($src, 0, $off) =~ tr/\n//);
      next if $seen{$ln}++;
      print "$ln:$lines[$ln-1]\n";
    }
  ' "$1" 2>/dev/null || true)
}

jsx_checks() { # $1=file
  local jsx=$1
  check "$jsx" '>[^<>{}]*\bharness(es)?[^<>{}]*<' 'vendor|claude' "user-facing harness in JSX"
  # "Claude (desktop) profiles" names the profile folders themselves (a
  # profile manager's ClaudeCodeProfiles), not accounts.
  check "$jsx" '>[^<>{}]*\b[Pp]rofiles?\b[^<>{}]*<' 'launch|settings|Launch|recipe|desktop|Claude profiles' "profile-as-account in JSX"
  # D-25: on screen it is a "session" (the code keeps Instance).
  check "$jsx" '>[^<>{}]*\b[Ii]nstances?\b[^<>{}]*<' 'NONE^' "Instance on screen in JSX (say session)"
  check "$jsx" '(aria-label|title|placeholder|label|alt)=("|\{`)[^"`$]*\b[Ii]nstances?\b' 'NONE^' "Instance in a visible attribute (say session)"
  # A label that starts its line: "instances: <span>2</span>".
  check "$jsx" '^\s*[^=;{}<>"`]*\b[Ii]nstances?\b[^=;{}<>"`]*<[a-z]' "$COMMENT_LINE" "Instance on screen in JSX (say session)"
  # D-25: on screen it is the tool (Claude Code, Codex); the code keeps Provider.
  check "$jsx" '>[^<>{}]*\bProviders?\b[^<>{}]*<' 'NONE^' "Provider on screen in JSX (say tool)"
  check_cs "$jsx" '>[^<>{}]*\bproviders?\b[^<>{}]*<' "$CODE_PROVIDER" "provider on screen in JSX (say tool)"
  check "$jsx" '^\s*[A-Z][^=;{}<>]*\bProviders?\b[^=;{}<>]*$' '^[0-9]+:\s*(//|\*|/\*)|AJ-E|D-[0-9]' "Provider in JSX text (say tool)"
  check_cs "$jsx" '^\s*[A-Z][^=;{}<>]*\bproviders?\b[^=;{}<>]*$' "^[0-9]+:\\s*(//|\\*|/\\*)|AJ-E|D-[0-9]|$CODE_PROVIDER" "provider in JSX text (say tool)"
  check "$jsx" '(aria-label|title|placeholder|label|alt|reason|hint)=("|\{`)[^"`$]*\b[Pp]roviders?\b' 'NONE^' "Provider in a visible attribute (say tool)"
  check "$jsx" ">[^<>{}]*${TASKCODE}[^<>{}]*<" "$COMMENT_LINE" "task code on screen in JSX (say it in plain words)"
  check "$jsx" "(aria-label|title|placeholder|label|alt|reason|hint)=(\"|\\{\`)[^\"\`\$]*${TASKCODE}" 'NONE^' "task code in a visible attribute (say it in plain words)"
  # JSX text that wraps: a bare line of words between tags.
  check "$jsx" "^\\s*[A-Za-z(][^=;{}<>\"\`]*${TASKCODE}[^=;{}<>]*\$" "$COMMENT_LINE" "task code in JSX text (say it in plain words)"
  case "$jsx" in *.test.tsx) ;; *)
    check "$jsx" '"[^"]*\b(a|the|each|all|other|this|another|every|per) [Pp]roviders?\b[^"]*"' '^[0-9]+:\s*(//|\*|/\*)' "Provider in a user-facing string (say tool)"
    string_codes "$jsx" ;;
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
                instances: <span className="tabular-nums">{n}</span>
          and every kill is audit-logged (D-08). Crash tidies wait 5 minutes
          Which Provider?
          Pick a Provider (Claude Code or Codex)
<p>Which provider?</p>
<p>No accounts for this provider!</p>
<p className="text-muted">Usage lands with E2</p>
const soon = 'arrives in E4';
<p>See D-7 for why</p>
const why = "kept apart by D-7";
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
  if (instances.length <3) return null;
 * AJ-E413 Req.3 (D-06): a doc comment may cite both.
<Card>{provider}</Card>
<Row onClick={() => choose(provider)}>Tool</Row>
          Install(account, provider)
<Row tone={provider === "codex" ? "accent" : "neutral"}>Tool</Row>
  foo("bar"); // AJ-E101: "baz"
<div data-testid="AJ-E1025-panel" />
<p>Can't stop it</p>; // AJ-E1: it's fine
  const kind = `E2E tests`; const keys = "Ctrl+D";
GOOD
  cat > "$dir/bad.ts" <<'BAD'
export const reason = "arrives with AJ-E606";
export const msg = `first line
  arrives with E2`;
const quoted = /"(.*)"/; const later = "lands with E2";
const half = text.replace(/"/g, ""); const why = "see D-7";
BAD
  cat > "$dir/good.ts" <<'GOOD'
export const label = "Not measured yet"; // AJ-E1025: "plain words"
const ratio = a / b; const ok = "fine";
/**
 * AJ-E413 "a doc comment may quote"
 */
const t = `fine ${x} words`;
GOOD
  hits="$(fail=0; jsx_checks "$dir/bad.tsx"; true)"
  # Every line of bad.tsx must be flagged (by one rule or more).
  if [ "$(printf '%s\n' "$hits" | grep -oE 'bad\.tsx:[0-9]+:' | sort -u | wc -l | tr -d ' ')" -ne 16 ]; then
    echo "terminology-lint selftest: expected all 16 lines of bad.tsx flagged, got:"; printf '%s\n' "$hits"; rm -rf "$dir"; exit 2
  fi
  hits="$(fail=0; string_codes "$dir/bad.ts"; string_codes "$dir/good.ts"; true)"
  # bad.ts: the code on line 3 sits in a template literal opened on line 2;
  # lines 4 and 5 follow a regex literal holding a quote.
  if [ "$(printf '%s\n' "$hits" | grep -oE 'bad\.ts:[0-9]+:' | sort -u | tr -d '\n')" != "bad.ts:1:bad.ts:3:bad.ts:4:bad.ts:5:" ] || printf '%s\n' "$hits" | grep -q 'good\.ts'; then
    echo "terminology-lint selftest: .ts strings — expected bad.ts flagged and good.ts clean, got:"; printf '%s\n' "$hits"; rm -rf "$dir"; exit 2
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
# Plain .ts carries user-facing strings too (reasons, labels, errors).
while IFS= read -r ts; do
  string_codes "$ts"
done < <(find ui/src -name "*.ts" ! -name "*.test.ts" ! -name "*.d.ts" | grep -v types/generated)

[ "$fail" -eq 0 ] && echo "terminology-lint: OK"
exit $fail
