#!/bin/bash
# 0.4.7 (the soak's persona runs): a lab's stand-in for Claude Code and
# Codex CLI. A lab never runs the real tools: `lab_scenario.sh setup` puts
# this file in the lab as `claude` and `codex` (`$HOME/.aij-lab-bin/`), and
# `open` points AI Juggler at them (its AI_JUGGLER_CLAUDE_BIN and
# AI_JUGGLER_CODEX_BIN), so a session started in a lab runs this.
#
#   claude --version         2.1.296 (Claude Code), as the adapter checks
#   claude [launch args]     Claude Code's screen at its prompt; a line typed
#                            there makes it ask to run a command; `/exit`
#                            (or `/quit`) there ends it on its own, exit 0, as
#                            Claude Code's does (DW1: a burst of finishes)
#   claude --lab-idle        the same, started by the seeder (PF1's running
#                            sessions: no terminal, so it stays at its prompt)
#   codex  …                 the same, as Codex CLI
#   opencode, kimi, grok     0.4.9: their versions as the adapters read them
#                            (OpenCode 1.19.5, Kimi Code 2.5.3, Grok Build
#                            1.0.77); started, each says it runs nothing and
#                            idles. The real ones sit in Homebrew's folder and
#                            on the shell's PATH, where AI Juggler looks, so a
#                            lab without these found and ran them.
#
# Asking now, without a line typed: `$HOME/.aij-lab-bin/asks-at-start`
# (a scenario's `standin = "asks"`, K3), or `--lab-ask`.
#
# A sign-in (AJ-E1553, the 0.5.1 PL1 walk's G2): started as AI Juggler
# starts Claude Code's sign-in (no arguments, in its own `sign-in` folder or
# the config folder itself), `claude` shows Claude's sign-in page (a
# claude.ai address and "Paste code here if prompted") and waits. The lab's
# approval, `$HOME/.aij-lab-bin/approve-sign-in` (its text: the lab
# Account's name, default "Lab New"), stands in for the person approving in
# the browser: the stand-in then writes the folder's signed-in signal as
# the adapter checks it (a non-empty `.credentials.json`, a placeholder
# that signs nothing in, and the lab's made-up `oauthAccount` in
# `.claude.json`: `lab-account-<slug>` in `lab-org-<slug>`, the names a lab
# Claude app profile's records folder on that Account has, as real Claude
# Code records its organisation, F37), says "Login successful" and goes on
# to its prompt. The approval file is used once. Nothing real is signed in,
# read or sent.
#
# A turn that never ends (AJ-E2312, K3's "may be stuck"): `--lab-open-turn`
# runs SessionStart, then UserPromptSubmit and a tool call that never
# returns (its record `busy`), and never Stop: what a hung Claude Code turn
# sends. Its hooks say Working, the tool's own word, so AI Juggler shows it
# Working; the seeded `open_turn` session (picked up, no hooks) is the one
# its "may be stuck" rule judges.
#
# A turn that edits (0.5.2 RW1: the feed's diff stat, AJ-E3713 R6a, and
# the turn review, AJ-E3820): a line `lab: KIND` at its prompt plays one
# whole turn as Claude Code writes it: UserPromptSubmit, then the turn's
# lines appended to its transcript in Claude Code's own format
# (`<folder>/projects/<cwd as a folder name>/<session>.jsonl`, compact
# JSON: an ask, the edit and write calls with their results, the reply),
# then Stop, every hook naming the transcript (`transcript_path`, as
# Claude Code's do) once the file exists. Nothing in the project is
# written. KIND:
#   edits     Edit src/checkout/cart.ts (+3 −1), Write two new files
#             (totals.ts +5, totals.test.ts +9): 3 files +17 −1; "The tests
#             pass: 12 passed, 0 failed."
#   revert    Edit src/format.ts (+1 −1), Edit src/locale.ts and Edit it
#             back (+2 −2): 2 files +3 −3; the reply says it reverted
#             locale.ts and hasn't run the tests
#   question  Edit src/sync/retry.ts (+4 −1); the reply ends on a question
#   fail      MultiEdit src/parse/dates.ts (+5 −3); "The tests fail: 3
#             failed…"; then it exits 1 on its own (an error, as a crashed
#             Claude Code ends)
#   notebook  NotebookEdit notebooks/report.ipynb: 1 file, no line counts
#             (files only)
# Any other line still asks (Bash: npm test), as before.
#
# Like Claude Code it records its session in its folder while it runs
# (`<CLAUDE_CONFIG_DIR or ~/.claude>/sessions/<pid>.json`: its pid, id,
# folder and status, `waiting` while it asks), and runs the hooks AI
# Juggler gave it (`--settings`): SessionStart, then PermissionRequest and
# Notification when it asks, Stop when answered, SessionEnd. Those reach
# AI Juggler through its own hook program, as a real prompt's do. It runs
# nothing, reads no sign-in and sends nothing else; an answer gets one line
# saying so. It ends on its own after AIJ_LAB_STANDIN_SECS (an hour), or
# when stopped. Only in a lab: a HOME under /tmp, its folder inside it.
#
#   scripts/qa/lab_standin.sh --selftest
set -u

name="$(basename "$0")"
secs="${AIJ_LAB_STANDIN_SECS:-3600}"

if [ "${1:-}" = "--selftest" ]; then
  tmp="$(mktemp -d /tmp/aij-standin-selftest.XXXXXX)"
  # Its stand-ins stop with it, a failed run's too.
  trap 'for p in $(cat "$tmp"/pid* 2>/dev/null); do kill "$p" 2>/dev/null; done; rm -rf "$tmp"' EXIT
  fail() { echo "lab_standin selftest: $*" >&2; exit 1; }
  for tool in claude codex opencode kimi grok; do cp "$0" "$tmp/$tool"; done
  [ "$("$tmp/claude" --version)" = "2.1.296 (Claude Code)" ] || fail "claude --version"
  "$tmp/codex" --version | grep -q '^codex-cli ' || fail "codex --version"
  # As each adapter's parse_version reads them (the bare version; the last
  # word, a leading v dropped; the first word starting with a digit).
  [ "$("$tmp/opencode" --version)" = "1.19.5" ] || fail "opencode --version"
  [ "$("$tmp/kimi" --version)" = "kimi-code v2.5.3" ] || fail "kimi --version"
  [ "$("$tmp/grok" --version)" = "grok 1.0.77 (lab)" ] || fail "grok --version"
  mkdir -p "$tmp/home"
  out="$(HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=0 "$tmp/opencode" run < /dev/null 2>&1)" || fail "opencode run"
  echo "$out" | grep -q 'runs nothing' || fail "opencode said: $out"
  HOME=/Users/you "$tmp/grok" < /dev/null > /dev/null 2>&1 && fail "grok ran on a real HOME"
  mkdir -p "$tmp/home/repo"
  # Its hooks, as AI Juggler writes them: each event's input kept.
  printf '#!/bin/sh\ncat > "%s/hook-$1.json"\n' "$tmp" > "$tmp/hook" && chmod +x "$tmp/hook"
  printf '{"hooks":{' > "$tmp/settings.json"
  for event in SessionStart UserPromptSubmit PermissionRequest Notification Stop SessionEnd; do
    printf '"%s":[{"hooks":[{"command":"'"'"'%s'"'"' %s","type":"command"}]}],' "$event" "$tmp/hook" "$event" >> "$tmp/settings.json"
  done
  printf '"x":[]}}\n' >> "$tmp/settings.json"
  # Asking: its record says waiting, its screen asks, and its hooks ran
  # with what a permission prompt says; stopped, it ends and its record goes.
  mkfifo "$tmp/in"
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" --settings "$tmp/settings.json" \
     --session-id 11111111-2222-4333-8444-555555555555 < "$tmp/in" > "$tmp/out" 2>&1 &
   echo $! > "$tmp/pid")
  exec 4> "$tmp/in"
  pid="$(cat "$tmp/pid")"
  record="$tmp/home/.claude/sessions/$pid.json"
  # (Each wait is for the last thing the step writes: its hook runs first.)
  for _ in $(seq 50); do grep -q '"status":"idle"' "$record" 2>/dev/null && break; sleep 0.1; done
  grep -q '"status":"idle"' "$record" 2>/dev/null || fail "not idle at its prompt: $(cat "$record" 2>&1)"
  grep -q '"hook_event_name":"SessionStart"' "$tmp/hook-SessionStart.json" || fail "no SessionStart"
  echo "run the tests" >&4
  for _ in $(seq 50); do [ -f "$tmp/hook-Notification.json" ] && break; sleep 0.1; done
  grep -q '"status":"waiting"' "$record" && grep -q '"sessionId":"11111111-2222-4333-8444-555555555555"' "$record" \
    && grep -q "\"cwd\":\"$tmp/home/repo\"" "$record" || fail "the record is wrong: $(cat "$record")"
  grep -q 'Do you want to proceed?' "$tmp/out" || fail "it didn't ask"
  grep -q '"tool_name":"Bash","tool_input":{"command":"npm test"' "$tmp/hook-PermissionRequest.json" || fail "no PermissionRequest"
  grep -q '"notification_type":"permission_prompt"' "$tmp/hook-Notification.json" || fail "no Notification"
  echo 1 >&4
  for _ in $(seq 50); do [ -f "$tmp/hook-Stop.json" ] && grep -q '"status":"idle"' "$record" && break; sleep 0.1; done
  [ -f "$tmp/hook-Stop.json" ] || fail "no Stop"
  grep -q '"status":"idle"' "$record" || fail "answered, it isn't idle again"
  kill "$pid"
  for _ in $(seq 30); do kill -0 "$pid" 2>/dev/null || break; sleep 0.1; done
  kill -0 "$pid" 2>/dev/null && fail "still running"
  pgrep -P "$pid" > /dev/null && fail "a child outlived it"
  [ ! -e "$record" ] || fail "its record stayed after it ended"
  [ -f "$tmp/hook-SessionEnd.json" ] || fail "no SessionEnd"
  exec 4>&-
  # DW1: `/exit` at its prompt ends it on its own, exit 0, its record gone.
  rm -f "$tmp"/hook-*.json
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" --settings "$tmp/settings.json" \
     < <(sleep 1; echo /exit) > "$tmp/out5" 2>&1; echo $? > "$tmp/code5")
  [ "$(cat "$tmp/code5")" = 0 ] || fail "/exit ended it with $(cat "$tmp/code5")"
  grep -q '"reason":"prompt_input_exit"' "$tmp/hook-SessionEnd.json" 2>/dev/null || fail "/exit: no SessionEnd"
  [ -z "$(ls "$tmp/home/.claude/sessions/" 2>/dev/null)" ] || fail "/exit left its record"
  # RW1: `lab: edits` plays a whole turn into its transcript, in Claude
  # Code's compact format, every hook after it naming the file; `lab: fail`
  # plays one and ends it with 1.
  rm -f "$tmp"/hook-*.json
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=30 "$tmp/claude" --settings "$tmp/settings.json" \
     --session-id 22222222-2222-4333-8444-555555555555 < <(sleep 1; echo "lab: edits"; sleep 11; echo /exit) > "$tmp/out7" 2>&1; echo $? > "$tmp/code7")
  turn="$(ls "$tmp"/home/.claude/projects/*/22222222-2222-4333-8444-555555555555.jsonl 2>/dev/null)"
  [ -f "$turn" ] || fail "lab: edits wrote no transcript"
  [ "$(grep -c '"type":"tool_use"' "$turn")" = 3 ] || fail "lab: edits: not three calls"
  grep -q '"type":"create"' "$turn" && grep -q '"type":"user"' "$turn" || fail "lab: edits: not Claude Code's compact lines"
  grep -q "\"transcript_path\":\"$turn\"" "$tmp/hook-Stop.json" || fail "lab: edits: Stop doesn't name the transcript"
  grep -q 'The tests pass' "$tmp/out7" || fail "lab: edits: no reply shown"
  [ "$(cat "$tmp/code7")" = 0 ] || fail "lab: edits then /exit ended with $(cat "$tmp/code7")"
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=30 "$tmp/claude" --settings "$tmp/settings.json" \
     < <(sleep 1; echo "lab: fail"; sleep 12) > "$tmp/out8" 2>&1; echo $? > "$tmp/code8")
  [ "$(cat "$tmp/code8")" = 1 ] || fail "lab: fail ended with $(cat "$tmp/code8"), not 1"
  grep -q 'The tests fail' "$tmp/out8" || fail "lab: fail: no reply shown"
  # Asking at start (K3), with no terminal at all.
  mkdir -p "$tmp/home/.aij-lab-bin" && touch "$tmp/home/.aij-lab-bin/asks-at-start"
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" < /dev/null > "$tmp/out2" 2>&1 &
   echo $! > "$tmp/pid2")
  pid2="$(cat "$tmp/pid2")"
  for _ in $(seq 50); do grep -q 'Do you want to proceed?' "$tmp/out2" && break; sleep 0.1; done
  grep -q '"status":"waiting"' "$tmp/home/.claude/sessions/$pid2.json" || fail "asks-at-start didn't ask"
  grep -q 'Do you want to proceed?' "$tmp/out2" || fail "asks-at-start didn't show its question"
  kill "$pid2"
  rm "$tmp/home/.aij-lab-bin/asks-at-start"
  # A hung turn (K3): UserPromptSubmit, busy, and never Stop.
  rm -f "$tmp"/hook-*.json
  (cd "$tmp/home/repo" || exit 1
   env -u CLAUDE_CONFIG_DIR HOME="$tmp/home" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" --settings "$tmp/settings.json" --lab-open-turn \
     < /dev/null > "$tmp/out4" 2>&1 &
   echo $! > "$tmp/pid4")
  pid4="$(cat "$tmp/pid4")"
  for _ in $(seq 50); do grep -q '"status":"busy"' "$tmp/home/.claude/sessions/$pid4.json" 2>/dev/null && break; sleep 0.1; done
  grep -q '"status":"busy"' "$tmp/home/.claude/sessions/$pid4.json" || fail "--lab-open-turn isn't busy"
  [ -f "$tmp/hook-SessionStart.json" ] && [ -f "$tmp/hook-UserPromptSubmit.json" ] || fail "--lab-open-turn: no SessionStart or UserPromptSubmit"
  sleep 1
  [ ! -e "$tmp/hook-Stop.json" ] || fail "--lab-open-turn ran Stop"
  kill "$pid4"
  # PF1's: at its prompt in its folder, no terminal.
  (cd "$tmp/home/repo" || exit 1
   HOME="$tmp/home" CLAUDE_CONFIG_DIR="$tmp/home/.claude-work" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" --lab-idle < /dev/null > /dev/null 2>&1 &
   echo $! > "$tmp/pid3")
  pid3="$(cat "$tmp/pid3")"
  for _ in $(seq 50); do grep -q '"status":"idle"' "$tmp/home/.claude-work/sessions/$pid3.json" 2>/dev/null && break; sleep 0.1; done
  grep -q '"status":"idle"' "$tmp/home/.claude-work/sessions/$pid3.json" || fail "--lab-idle isn't idle"
  kill "$pid3"
  # AJ-E1553 (G2): a sign-in shows Claude's page and waits; the lab's
  # approval signs the folder in by the adapter's signal, once.
  mkdir -p "$tmp/home/sign-in" "$tmp/home/.aij-lab-bin"
  (cd "$tmp/home/sign-in" || exit 1
   HOME="$tmp/home" CLAUDE_CONFIG_DIR="$tmp/home/new" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" < /dev/null > "$tmp/out6" 2>&1 &
   echo $! > "$tmp/pid6")
  pid6="$(cat "$tmp/pid6")"
  for _ in $(seq 50); do grep -q 'https://claude.ai/' "$tmp/out6" 2>/dev/null && break; sleep 0.1; done
  grep -q 'https://claude.ai/' "$tmp/out6" && grep -q 'Paste code here if prompted' "$tmp/out6" || fail "no sign-in page: $(cat "$tmp/out6")"
  sleep 1
  [ ! -e "$tmp/home/new/.credentials.json" ] || fail "signed in before the approval"
  echo "Lab New" > "$tmp/home/.aij-lab-bin/approve-sign-in"
  for _ in $(seq 50); do [ -s "$tmp/home/new/.credentials.json" ] && grep -q 'Login successful' "$tmp/out6" && break; sleep 0.1; done
  [ -s "$tmp/home/new/.credentials.json" ] || fail "the approval didn't sign the folder in"
  grep -q '"accountUuid": "lab-account-lab-new"' "$tmp/home/new/.claude.json" || fail "no lab oauthAccount"
  # F37: its organisation, named as the lab's records folders are.
  grep -q '"organizationUuid": "lab-org-lab-new"' "$tmp/home/new/.claude.json" || fail "no lab organizationUuid"
  [ ! -e "$tmp/home/.aij-lab-bin/approve-sign-in" ] || fail "the approval was kept"
  kill "$pid6"
  # Never outside a lab: a real HOME, or a folder outside its HOME.
  HOME=/Users/you "$tmp/claude" < /dev/null > /dev/null 2>&1 && fail "ran on a real HOME"
  HOME="$tmp/home" CLAUDE_CONFIG_DIR=/Users/you/.claude "$tmp/claude" < /dev/null > /dev/null 2>&1 && fail "wrote outside its HOME"
  for _ in $(seq 30); do kill -0 "$pid2" 2>/dev/null || kill -0 "$pid3" 2>/dev/null || break; sleep 0.1; done
  echo "lab_standin selftest: ok"
  exit 0
fi

case "${1:-}" in
  --version|-v|-V)
    case "$name" in
      codex) echo "codex-cli 0.163.0" ;;
      opencode) echo "1.19.5" ;;
      kimi) echo "kimi-code v2.5.3" ;;
      grok) echo "grok 1.0.77 (lab)" ;;
      *) echo "2.1.296 (Claude Code)" ;;
    esac
    exit 0 ;;
esac

case "$HOME" in
  /tmp/*|/private/tmp/*) ;;
  *) echo "lab stand-in: HOME isn't a lab's; nothing runs" >&2; exit 2 ;;
esac

# OpenCode, Kimi Code and Grok Build: found, never run.
case "$name" in
  opencode|kimi|grok)
    echo "$name (a lab's stand-in): it runs nothing here."
    [ "$secs" -gt 0 ] && { sleep "$secs" & wait $!; }
    exit 0 ;;
esac

asks=0
[ -e "$HOME/.aij-lab-bin/asks-at-start" ] && asks=1
session=""
settings=""
transcript=""
prev=""
for arg in "$@"; do
  case "$arg" in
    --lab-idle) asks=0 ;;
    --lab-ask) asks=1 ;;
    --lab-open-turn) asks=2 ;;
    --resume=*) session="${arg#--resume=}" ;;
  esac
  case "$prev" in
    --session-id|--resume|-r) session="$arg" ;;
    --settings) settings="$arg" ;;
  esac
  prev="$arg"
done
[ -n "$session" ] || session="$(uuidgen | tr '[:upper:]' '[:lower:]')"

record=""
if [ "$name" != codex ]; then
  folder="${CLAUDE_CONFIG_DIR:-$HOME/.claude}"
  case "$folder" in
    "$HOME"/*) ;;
    *) echo "lab stand-in: its folder isn't in the lab; nothing runs" >&2; exit 2 ;;
  esac
  mkdir -p "$folder/sessions"
  record="$folder/sessions/$$.json"
fi
# Its transcript, where Claude Code keeps it (a lab turn writes it).
[ "$name" != codex ] && transcript="$folder/projects/$(printf '%s' "$PWD" | tr -c 'A-Za-z0-9' '-')/$session.jsonl"

# Its session record, as Claude Code keeps one while it runs.
note() {
  [ -n "$record" ] || return 0
  printf '{"pid":%s,"sessionId":"%s","cwd":"%s","status":"%s","entrypoint":"cli","kind":"interactive"}\n' \
    "$$" "$session" "$PWD" "$1" > "$record.tmp" && mv "$record.tmp" "$record"
}

# Runs hook EVENT from its settings with JSON fields FIELDS, as Claude
# Code hands a hook its input on stdin; whatever it prints is dropped.
hook() {
  [ -n "$settings" ] && [ -f "$settings" ] || return 0
  local cmd at=""
  cmd="$(plutil -extract "hooks.$1.0.hooks.0.command" raw -o - "$settings" 2>/dev/null)" || return 0
  [ -n "$cmd" ] || return 0
  # The transcript, as Claude Code names it, once a lab turn wrote it.
  [ -n "$transcript" ] && [ -f "$transcript" ] && at=",\"transcript_path\":\"$transcript\""
  printf '{"session_id":"%s","cwd":"%s","hook_event_name":"%s"%s%s}' "$session" "$PWD" "$1" "$at" "${2:-}" \
    | /bin/sh -c "$cmd" > /dev/null 2>&1 || true
}

finish() {
  trap - TERM INT HUP
  # Its own children (a wait's sleep, a hook) end with it, never after.
  pkill -P $$ 2>/dev/null
  hook SessionEnd ",\"reason\":\"${1:-other}\""
  [ -n "$record" ] && rm -f "$record"
  exit "${2:-0}"
}

# A lab turn's lines (see the header), appended to the transcript; the
# screen's lines printed. Exit 3: no such kind.
LAB_TURN_PY='import datetime as dt, json, os, sys, uuid
kind, path, session, cwd = sys.argv[1:5]
TURNS = {
 "edits": ("Split the cart total into its own module and test it", [
   ("Edit", {"file_path": "src/checkout/cart.ts", "old_string": "export function total(items) {\n  return items.reduce((s, i) => s + i.price, 0);\n}", "new_string": "import { sumPrices } from \"./totals\";\n\nexport function total(items) {\n  return sumPrices(items);\n}"}, None, "Updated src/checkout/cart.ts with 3 additions and 1 removal"),
   ("Write", {"file_path": "src/checkout/totals.ts", "content": "export function sumPrices(items) {\n  let sum = 0;\n  for (const item of items) sum += item.price;\n  return sum;\n}"}, "create", "Wrote 5 lines to src/checkout/totals.ts"),
   ("Write", {"file_path": "src/checkout/totals.test.ts", "content": "import { sumPrices } from \"./totals\";\n\ntest(\"an empty cart is 0\", () => {\n  expect(sumPrices([])).toBe(0);\n});\n\ntest(\"prices add up\", () => {\n  expect(sumPrices([{ price: 2 }, { price: 3 }])).toBe(5);\n});"}, "create", "Wrote 9 lines to src/checkout/totals.test.ts")],
   "I moved the price sum out of cart.ts into a new totals.ts and added totals.test.ts for it. The tests pass: 12 passed, 0 failed."),
 "revert": ("Change the date format in the API responses to ISO 8601", [
   ("Edit", {"file_path": "src/format.ts", "old_string": "  return d.toLocaleDateString();", "new_string": "  return d.toISOString();"}, None, "Updated src/format.ts with 1 addition and 1 removal"),
   ("Edit", {"file_path": "src/locale.ts", "old_string": "const FORMAT = \"DD.MM.YYYY\";", "new_string": "const FORMAT = \"YYYY-MM-DD\";"}, None, "Updated src/locale.ts with 1 addition and 1 removal"),
   ("Edit", {"file_path": "src/locale.ts", "old_string": "const FORMAT = \"YYYY-MM-DD\";", "new_string": "const FORMAT = \"DD.MM.YYYY\";"}, None, "Updated src/locale.ts with 1 addition and 1 removal")],
   "I switched the API dates to ISO 8601 in format.ts. I reverted the change to locale.ts because the German invoices still need their own format. I have not run the tests yet."),
 "question": ("Add retries to the sync client", [
   ("Edit", {"file_path": "src/sync/retry.ts", "old_string": "  return send(req);", "new_string": "  for (let tries = 0; tries < 5; tries++) {\n    try { return await send(req); } catch (e) { await wait(2 ** tries * 100); }\n  }\n  throw new Error(\"sync failed after 5 tries\");"}, None, "Updated src/sync/retry.ts with 4 additions and 1 removal")],
   "I added a retry with backoff to the sync client in retry.ts. The tests pass. Should it stop after five tries, or keep trying until the server answers?"),
 "fail": ("Rewrite the date parser to accept two-digit years", [
   ("MultiEdit", {"file_path": "src/parse/dates.ts", "edits": [
      {"old_string": "const YEAR = /^\\d{4}$/;\nconst SEP = \"-\";", "new_string": "const YEAR = /^\\d{2}(\\d{2})?$/;\nconst SEP = /[-.\\/]/;\nconst PIVOT = 70;"},
      {"old_string": "  const year = Number(parts[0]);\n  return new Date(year, month, day);", "new_string": "  let year = Number(parts[0]);\n  if (year < 100) year += year < PIVOT ? 2000 : 1900;\n  return new Date(year, month, day);"}]}, None, "Applied 2 edits to src/parse/dates.ts")],
   "I rewrote the date parser in dates.ts to accept two-digit years. The tests fail: 3 failed in dates.test.ts, all on years before 1970."),
 "notebook": ("Update the summary cell in the report notebook", [
   ("NotebookEdit", {"notebook_path": "notebooks/report.ipynb", "cell_id": "cell-3", "new_source": "summary = orders.groupby(\"week\").total.sum()", "edit_mode": "replace"}, None, "Updated cell cell-3")],
   "I updated the summary cell in report.ipynb. I did not run the notebook."),
}
if kind not in TURNS:
    sys.exit(3)
ask, calls, reply = TURNS[kind]
at = dt.datetime.now(dt.timezone.utc) - dt.timedelta(seconds=4)
parent = None
try:
    with open(path) as f:
        kept = [l for l in f.read().splitlines() if l.strip()]
    parent = json.loads(kept[-1]).get("uuid") if kept else None
except (OSError, ValueError):
    pass
out = []
def line(kind_, message, extra=None):
    global at, parent
    at += dt.timedelta(milliseconds=600)
    uid = str(uuid.uuid4())
    value = {"parentUuid": parent, "isSidechain": False, "userType": "external", "cwd": cwd, "sessionId": session,
             "version": "2.1.296", "gitBranch": "main", "entrypoint": "cli", "type": kind_, "message": message,
             "uuid": uid, "timestamp": at.strftime("%Y-%m-%dT%H:%M:%S.") + "%03dZ" % (at.microsecond // 1000)}
    value.update(extra or {})
    parent = uid
    out.append(json.dumps(value, separators=(",", ":"), ensure_ascii=False))
def said(content, stop):
    return {"id": "msg_" + uuid.uuid4().hex[:12], "type": "message", "role": "assistant", "model": "claude-sonnet-4-5",
            "content": content, "stop_reason": stop,
            "usage": {"input_tokens": 1200, "cache_read_input_tokens": 8000, "output_tokens": 300, "service_tier": "standard"}}
line("user", {"role": "user", "content": ask})
print(" > " + ask)
for name, given, result, words in calls:
    given = dict(given)
    for key in ("file_path", "notebook_path"):
        if key in given:
            given[key] = cwd + "/" + given[key]
    tid = "toolu_" + uuid.uuid4().hex[:20]
    line("assistant", said([{"type": "tool_use", "id": tid, "name": name, "input": given}], "tool_use"))
    shown = given.get("file_path") or given.get("notebook_path")
    extra = {"toolUseResult": {"type": "create", "filePath": shown, "content": given["content"], "structuredPatch": []}} if result == "create" else {}
    line("user", {"role": "user", "content": [{"tool_use_id": tid, "type": "tool_result", "content": words}]}, extra)
    print(" ⏺ %s(%s)\n   ⎿  %s" % ("Update" if name in ("Edit", "MultiEdit") else name, shown[len(cwd) + 1:], words))
line("assistant", said([{"type": "text", "text": reply}], "end_turn"))
print("\n ⏺ " + reply)
os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "a") as f:
    f.write("\n".join(out) + "\n")'

lab_turn() {
  local shown
  [ -n "$transcript" ] || return 3
  hook UserPromptSubmit ',"prompt":"(lab)"'
  note busy
  printf '\n ✻ Working… (lab stand-in)\n'
  # Long enough to see it Working (its hooks said so) before Stop.
  sleep 8 & wait $!
  shown="$(/usr/bin/python3 -I -c "$LAB_TURN_PY" "$1" "$transcript" "$session" "$PWD")" || return 3
  printf '%s\n' "$shown"
  hook Stop ',"stop_hook_active":false'
  return 0
}
trap finish TERM INT HUP

prompt() {
  note idle
  printf '\n ╭──────────────────────────────────────────────────────────────╮\n'
  printf ' │ >                                                            │\n'
  printf ' ╰──────────────────────────────────────────────────────────────╯\n'
  printf '   ? for shortcuts\n'
}

ask() {
  note waiting
  printf '\n ⏺ I'"'"'ll run the test suite to check the fix.\n\n'
  printf ' ╭──────────────────────────────────────────────────────────────╮\n'
  printf ' │ Bash command                                                 │\n'
  printf ' │                                                              │\n'
  printf ' │   npm test                                                   │\n'
  printf ' │   Run the project'"'"'s tests                                    │\n'
  printf ' │                                                              │\n'
  printf ' │ Do you want to proceed?                                      │\n'
  printf ' │ ❯ 1. Yes                                                     │\n'
  printf ' │   2. Yes, and don'"'"'t ask again for npm test commands          │\n'
  printf ' │   3. No, and tell Claude what to do differently (esc)        │\n'
  printf ' ╰──────────────────────────────────────────────────────────────╯\n'
  hook PermissionRequest ',"tool_name":"Bash","tool_input":{"command":"npm test","description":"Run the project'"'"'s tests"}'
  hook Notification ',"message":"Claude needs your permission to use Bash","notification_type":"permission_prompt"'
}

# A sign-in (see the header): no arguments, in AI Juggler's sign-in folder.
if [ "$name" != codex ] && [ "$#" -eq 0 ] && { [ "$(basename "$PWD")" = sign-in ] || [ "$PWD" = "$folder" ]; }; then
  note idle
  printf '\n ✻ Welcome to Claude Code (lab stand-in)\n\n Browser didn'"'"'t open? Use the url below to sign in:\n\n'
  printf ' https://claude.ai/oauth/authorize?code=true&client_id=lab-stand-in&state=lab\n\n Paste code here if prompted > '
  approve="$HOME/.aij-lab-bin/approve-sign-in"
  deadline=$((SECONDS + secs))
  while [ "$SECONDS" -lt "$deadline" ] && [ ! -f "$approve" ]; do sleep 1 & wait $!; done
  [ -f "$approve" ] || finish other
  who="$(head -c 64 "$approve" | tr -cd 'A-Za-z0-9 -')"
  rm -f "$approve"
  /usr/bin/python3 -I -c 'import json, re, sys
folder, who = sys.argv[1], sys.argv[2] or "Lab New"
slug = re.sub("[^a-z0-9]", "-", who.lower())
path = folder + "/.claude.json"
try:
    state = json.load(open(path))
except (OSError, ValueError):
    state = {}
state["oauthAccount"] = {"accountUuid": "lab-account-" + slug, "organizationUuid": "lab-org-" + slug,
                         "emailAddress": slug + "@example.invalid"}
json.dump(state, open(path, "w"), indent=2)' "$folder" "$who"
  printf '{"lab": "not a sign-in"}' > "$folder/.credentials.json"
  printf '\n\n Login successful. Press Enter to continue\n'
fi

hook SessionStart ',"source":"startup"'
if [ "$name" = codex ]; then
  printf '\n  >_ OpenAI Codex (lab stand-in)\n\n  directory: %s\n' "$PWD"
else
  printf '\n ✻ Welcome to Claude Code (lab stand-in)\n\n   cwd: %s\n' "$PWD"
fi
asking=0
if [ "$asks" = 2 ]; then
  # A hung turn: asked, a tool call started, never a result or Stop.
  hook UserPromptSubmit ',"prompt":"(lab)"'
  note busy
  printf '\n ⏺ Running the whole suite first.\n\n ⏺ Bash(npm test)\n   ⎿  Running…\n'
  asking=2
elif [ "$asks" = 1 ]; then ask; asking=1; else prompt; fi

# Its terminal, a line at a time, until its time is up: a line at its
# prompt makes it ask; any answer gets one line, and it's back at its
# prompt. With no terminal (the seeder's), it only waits.
deadline=$((SECONDS + secs))
while [ "$SECONDS" -lt "$deadline" ]; do
  if IFS= read -r -t 5 line; then
    if [ "$asking" = 2 ]; then
      :  # a hung turn takes nothing in
    elif [ "$asking" = 1 ]; then
      printf '\n   (a lab stand-in: nothing runs here)\n'
      hook Stop ',"stop_hook_active":false'
      asking=0
      prompt
    elif [ "${line// /}" = /exit ] || [ "${line// /}" = /quit ]; then
      # Ended at its prompt, as Claude Code's /exit ends: on its own, 0.
      finish prompt_input_exit
    elif [[ "$line" =~ ^[[:space:]]*lab:[[:space:]]*([a-z]+)[[:space:]]*$ ]] && [ -n "$transcript" ]; then
      # A lab turn (see the header); an unknown kind says so.
      if lab_turn "${BASH_REMATCH[1]}"; then
        if [ "${BASH_REMATCH[1]}" = fail ]; then
          printf '\n   ⎿  API Error (lab stand-in): the session ends with an error\n'
          finish other 1
        fi
      else
        printf '\n   (a lab stand-in: no lab turn named %s; edits, revert, question, fail, notebook)\n' "${BASH_REMATCH[1]}"
        hook Stop ',"stop_hook_active":false'
      fi
      prompt
    elif [ -n "${line// /}" ]; then
      hook UserPromptSubmit ',"prompt":"(lab)"'
      ask
      asking=1
    fi
  elif [ $? -le 128 ]; then
    sleep 5 &
    wait $!
  fi
done
finish
