#!/bin/bash
# 0.4.7 (the soak's persona runs): a lab's stand-in for Claude Code and
# Codex CLI. A lab never runs the real tools: `lab_scenario.sh setup` puts
# this file in the lab as `claude` and `codex` (`$HOME/.aij-lab-bin/`), and
# `open` points AI Juggler at them (its AI_JUGGLER_CLAUDE_BIN and
# AI_JUGGLER_CODEX_BIN), so a session started in a lab runs this.
#
#   claude --version         2.1.296 (Claude Code), as the adapter checks
#   claude [launch args]     Claude Code's screen at its prompt; a line typed
#                            there makes it ask to run a command
#   claude --lab-idle        the same, started by the seeder (PF1's running
#                            sessions: no terminal, so it stays at its prompt)
#   codex  …                 the same, as Codex CLI
#
# Asking now, without a line typed: `$HOME/.aij-lab-bin/asks-at-start`
# (a scenario's `standin = "asks"`, K3), or `--lab-ask`.
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
  cp "$0" "$tmp/claude" && cp "$0" "$tmp/codex"
  [ "$("$tmp/claude" --version)" = "2.1.296 (Claude Code)" ] || fail "claude --version"
  "$tmp/codex" --version | grep -q '^codex-cli ' || fail "codex --version"
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
  # PF1's: at its prompt in its folder, no terminal.
  (cd "$tmp/home/repo" || exit 1
   HOME="$tmp/home" CLAUDE_CONFIG_DIR="$tmp/home/.claude-work" AIJ_LAB_STANDIN_SECS=20 "$tmp/claude" --lab-idle < /dev/null > /dev/null 2>&1 &
   echo $! > "$tmp/pid3")
  pid3="$(cat "$tmp/pid3")"
  for _ in $(seq 50); do grep -q '"status":"idle"' "$tmp/home/.claude-work/sessions/$pid3.json" 2>/dev/null && break; sleep 0.1; done
  grep -q '"status":"idle"' "$tmp/home/.claude-work/sessions/$pid3.json" || fail "--lab-idle isn't idle"
  kill "$pid3"
  # Never outside a lab: a real HOME, or a folder outside its HOME.
  HOME=/Users/nobody "$tmp/claude" < /dev/null > /dev/null 2>&1 && fail "ran on a real HOME"
  HOME="$tmp/home" CLAUDE_CONFIG_DIR=/Users/nobody/.claude "$tmp/claude" < /dev/null > /dev/null 2>&1 && fail "wrote outside its HOME"
  for _ in $(seq 30); do kill -0 "$pid2" 2>/dev/null || kill -0 "$pid3" 2>/dev/null || break; sleep 0.1; done
  echo "lab_standin selftest: ok"
  exit 0
fi

case "${1:-}" in
  --version|-v|-V)
    if [ "$name" = codex ]; then echo "codex-cli 0.163.0"; else echo "2.1.296 (Claude Code)"; fi
    exit 0 ;;
esac

case "$HOME" in
  /tmp/*|/private/tmp/*) ;;
  *) echo "lab stand-in: HOME isn't a lab's; nothing runs" >&2; exit 2 ;;
esac

asks=0
[ -e "$HOME/.aij-lab-bin/asks-at-start" ] && asks=1
session=""
settings=""
prev=""
for arg in "$@"; do
  case "$arg" in
    --lab-idle) asks=0 ;;
    --lab-ask) asks=1 ;;
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
  local cmd
  cmd="$(plutil -extract "hooks.$1.0.hooks.0.command" raw -o - "$settings" 2>/dev/null)" || return 0
  [ -n "$cmd" ] || return 0
  printf '{"session_id":"%s","cwd":"%s","hook_event_name":"%s"%s}' "$session" "$PWD" "$1" "${2:-}" \
    | /bin/sh -c "$cmd" > /dev/null 2>&1 || true
}

finish() {
  trap - TERM INT HUP
  # Its own children (a wait's sleep, a hook) end with it, never after.
  pkill -P $$ 2>/dev/null
  hook SessionEnd ',"reason":"other"'
  [ -n "$record" ] && rm -f "$record"
  exit 0
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

hook SessionStart ',"source":"startup"'
if [ "$name" = codex ]; then
  printf '\n  >_ OpenAI Codex (lab stand-in)\n\n  directory: %s\n' "$PWD"
else
  printf '\n ✻ Welcome to Claude Code (lab stand-in)\n\n   cwd: %s\n' "$PWD"
fi
asking=0
if [ "$asks" = 1 ]; then ask; asking=1; else prompt; fi

# Its terminal, a line at a time, until its time is up: a line at its
# prompt makes it ask; any answer gets one line, and it's back at its
# prompt. With no terminal (the seeder's), it only waits.
deadline=$((SECONDS + secs))
while [ "$SECONDS" -lt "$deadline" ]; do
  if IFS= read -r -t 5 line; then
    if [ "$asking" = 1 ]; then
      printf '\n   (a lab stand-in: nothing runs here)\n'
      hook Stop ',"stop_hook_active":false'
      asking=0
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
