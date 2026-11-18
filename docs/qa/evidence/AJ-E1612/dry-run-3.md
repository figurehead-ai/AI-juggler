# AJ-E1612 sitting 2: dry run 3 (opportunistic) on stand-ins (2026-11-18), PARTIAL: sign-ins 6/6, Kimi session/resume, then Dev's window left the screen

Driver: sonnet. Lab `/tmp/aij-lab-t16-dry`, `--standins --with-hermes`, 12 Apps, DRY RUN line present. Dev's lab build made from HEAD 2cf5ed537 or later (the tree moved on under me; one build at the start, none after), Dev pid 80373 (driven by pid only, through lab_drive, with `AIJ_DRIVE_MAY_ACTIVATE=1 AIJ_DRIVE_OPPORTUNISTIC=1` set per command). Selftests (stand-ins, lab_sign_ins, lab_drive guard) passed. `real --compare` and `muse-gate --compare` unchanged at the end. The maintainer used the Mac throughout the first 1.5 hours, then it sat idle (>55 min).

## Grid

| Tool | sign-in (Dev's Sign in + read-label EVIDENCE) | session | resume | usage | lab B |
|---|---|---|---|---|---|
| OpenCode | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Kimi Code | PASS | FAIL (primitive; probe route PASS) | FAIL (primitive; fallback PASS) | FAIL (primitive; Overview showed windows) | FAIL (not run) |
| Grok Build | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| ZCode | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Muse Code | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Hermes | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |

0.4.6 rows: not in the dry run; `handover-0.4.6` and `evidence-confirmed` recorded skip, "dry run".

## FAILs, one line each

1. `new-session` (sheet step) stops, exit 4: two controls are titled "New session" (the header button and the sidebar's footer). Probe: `press "New session: Kimi Code CLI · lab A"` then `press "Start in lab"` works.
2. `end-session` stops: no control titled "Stop this session"; the session page has "Stop", which `press` refuses ("isn't inside Dev's window on screen"). Fallback `key ctrl-c` ends it (exit code 130, sidebar "Stopped with an error"); `resume "Kimi Code CLI · lab A"` then restored the same session id with its earlier turn.
3. `open-usage` stops on a session page: two controls titled "Usage" (header nav and the session's tab). The session Overview showed Runway: 5-hour window 88% left, This week 69% left, "from Kimi Code's own local server (experimental)".
4. Each `send` of "reply with the word lab" takes about 8 minutes in opportunistic mode: every typed key produces `PAUSED … idle=0.0s` then `RESUMED` 20 s later (23 pauses for 24 characters), so the driver's own keystrokes count as the person's input. Works, but a sitting's 12 sends cost about 1.5 h.
5. Dev's main window ("AI Juggler · lab") left the screen after Kimi's resume (other Space; not minimised, not fullscreen by AX). `ACTIVATED … idle=1676.5s` did not bring it back, `lab_drive press Home` exits "the control isn't inside Dev's window on screen" (only a small 280x168 Dev window was frontmost), and the driver activates once per resume only. Everything after (usage, lab B, and OpenCode/Grok/ZCode/Muse/Hermes steps 2-6) is "not run". Earlier, the same cause (ACTIVATED with no effect, then `WAITING: Dev's window is off screen … never is again`) delayed Home presses 15:00-15:47 and 15:57-16:08.
6. Sidebar "Sign in · lab A" entries pile up (7 after six sign-ins plus the wrong-path one); a stopped Kimi session shows two "Stopped with an error" rows after resume (the ctrl-c'd one stays listed).

By-hand help, for the record: I ran `open` on the lab Dev app (not through the driver) at 15:40:48 (idle 51 s), 16:08:13 (idle 386 s), 19:28:12 (idle 2023 s), 19:50:04 (idle 3335 s), each only with the Mac idle >=25 s, to bring the window back. The first two worked; the last two did not. An unattended watcher loop for this was denied by the permission system and not run.

## What ran, exactly

- Before each sign-in: `read-label "<Tool> CLI · lab A"` printed `EVIDENCE: Home's Needs you has the row "<Tool> CLI · lab A needs its sign-in"`, `STATE: not-signed-in`.
- Labels pressed: `Sign in to <App>` with `--row "<App> needs its sign-in"` (press by name), for OpenCode, Kimi Code, Grok Build, ZCode, Muse Code, Hermes Agent CLI · lab A; calls.log shows each ran in its own App's folder (`auth login`, `login`, `zcode.cjs login`, `hermes model`), HOME in the lab; `check --signed-in` after each: lab A has its file, lab B none, nothing else; `real --compare` unchanged. Muse: `muse-gate --compare` unchanged (Keychain item not touched).
- After each terminal ended, `press "+N more need you"` (N = 7, 6, 5, 4, 3 after the first, as each sign-in lands; OpenCode's was +8), then `read-label … lab A`: `EVIDENCE: no row "<App> needs its sign-in" on Home's Needs you (by its absence; valid after its sign-in terminal ended)` + `STATE: signed-in`; `… lab B`: `EVIDENCE: Home's Needs you has the row "<App> needs its sign-in"` + `STATE: not-signed-in`. Same shape for all six. The rail's needs count and Home's header now agree (12, 11, 10, 9, 8, 7, 6 after each sign-in), unlike dry run 2.
- Footers: "Complete <Tool>'s sign-in above, then check the app again from Accounts — it flips to ready." names the right tool (OpenCode, Kimi Code, Grok Build, ZCode, Muse Code, Hermes Agent).
- **Keys reached Dev's terminal: YES** (OCR showed each menu change after `key down`, `key return`, `type`).
- **OpenCode wrong path:** default chosen (`key return` on "Select provider" with OpenCode active, "recommended"), OCR showed the API key prompt, `key ctrl-c`: `check` showed no sign-in in lab A. PASS.
- **OpenCode's menu path:** `wait "Select provider"`, `wait "recommended"`, `key down`, `wait "ChatGPT Plus/Pro or API key"`, `key return`, `wait "ChatGPT Pro/Plus (browser)"`, `key return`, `wait "Go to"` (page `auth.openai.example.invalid`, "waiting as if you signed in"): all PASS; the fake page finished by itself (~20 s), `check`: lab A signed in.
- Kimi: `wait "one-time code"` PASS (code on screen only, not recorded). Hermes: its provider menu (1 Nous Portal, 2 OpenAI Codex, 3 Anthropic, 4 OpenRouter) took `type 2` + return, then its model menu `type 1` + return; sign-in file in place afterwards.
- Kimi session probe (see FAIL 1-4): answered "lab"; resumed session `Resumed session f8814dda…` showed its earlier turn and answered "lab" again; capture `kimi-resumed-turn2.jpg`.
- **Guard (MAY_ACTIVATE unset case):** not run separately; the guard's refusal with another window on top was seen as `WAITING: another window is on top at the point: Notification Center` (waited, then proceeded). The STOPPED-after-touch case: not seen (opportunistic mode pauses instead; none).
- **Teardown guard:** `lab_scenario.sh teardown T16 --home /tmp/aij-lab-t16-dry` before any step but sign-ins was recorded refused: `FAIL … no result recorded` for the remaining steps and the sitting's two, then `refusing to tear down … run-sheet steps have no result`; lab left. PASS. After all cells had results (honest FAILs, two skips), `complete` passed.

## Pause / resume / activation lines

Totals in the surviving log (`PAUSED 79`, `RESUMED 77`, `ACTIVATED 6`, `REFOCUSED 2`, `UNHID 0`, `UNMINIMISED 0`, `HEARTBEAT 0` in that file, `STOPPED 0`) plus the lines before 15:47 UTC, whose log another process deleted mid-run (rebuilt here from what my terminal showed): `PAUSED 15:00:47` (sign-in), `HEARTBEAT 15:10:48` and `15:20:49`, `RESUMED+ACTIVATED 15:25:11`, `PAUSED 15:25:44`, `RESUMED+ACTIVATED 15:31:25`, `PAUSED 15:31:31`, `RESUMED+ACTIVATED 15:34:17`, `PAUSED 15:35:21`, `RESUMED+ACTIVATED 15:40:17`, `ACTIVATED 15:41:02`, `PAUSED 15:41:03`, `RESUMED 15:41:22`, `PAUSED 15:41:46`, `RESUMED 15:42:06`, `PAUSED 15:42:06`, `RESUMED 15:42:26`, `PAUSED 15:46:35`, `RESUMED+ACTIVATED 15:47:22` (all `idle` 20.0-20.9 s on resume). Overall about 90 PAUSED, 88 RESUMED, 11 ACTIVATED, 3 HEARTBEAT. The surviving lines (activations and refocus) are listed here in full:

```text
ACTIVATED 2026-11-18T15:57:15Z step=press idle=20.0s
ACTIVATED 2026-11-18T15:58:48Z step=press idle=20.8s
ACTIVATED 2026-11-18T16:02:07Z step=press idle=20.9s
REFOCUSED 2026-11-18T16:23:25Z step=send (Dev's focus moved during the pause; put back where this step had it)
ACTIVATED 2026-11-18T16:23:25Z step=send idle=20.4s
REFOCUSED 2026-11-18T16:46:19Z step=send (Dev's focus moved during the pause; put back where this step had it)
ACTIVATED 2026-11-18T16:46:19Z step=send idle=21.1s
ACTIVATED 2026-11-18T19:22:25Z step=press idle=1676.5s
```

The rest are the `PAUSED … idle=0.0s` / `RESUMED … idle=20.x s` pairs of FAIL 4 (one per typed key) and a handful of person-caused pauses (idle 0.1-17.9 s, 15:00, 15:25, 15:31, 15:35, 15:46, 15:56, 16:00, 16:09, 16:13, 16:20).

## Labels table

| What | Exact words |
|---|---|
| Needs-you row | `<App> needs its sign-in` (page); description `<Tool>'s own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password.` |
| Sign in control | `Sign in to <App>` inside that row; sidebar: `lab A needs its sign-in, …, Sign in…` |
| "+n more" control | `+N more need you` |
| App names | `OpenCode CLI · lab A`, `Kimi Code CLI · lab A`, `Grok Build CLI · lab A`, `ZCode CLI · lab A`, `Muse Code CLI · lab A`, `Hermes Agent CLI · lab A` (and lab B) |
| New session | two controls `New session` (ambiguous); `New session: <App>` (one per App) then `Change app: <App>`, `Change folder: lab`, `Start in lab` |
| Session row | `Working, reply with the word lab, lab · 12 min ago, on lab A`; ended: `Stopped with an error, reply with the word lab, lab · Kimi Code CLI · lab A` |
| Stop / Resume | `Stop` (not pressable by the driver); `Resume` (found `--in "Kimi Code CLI · lab A"`) |
| Usage | two `Usage` controls on a session page; Overview "Runway" block |
| ZCode message field | not reached |

Captures (redact_capture with sidecar, `--no-real-data`, each opened by eye and confirmed; only the avatar initial "F" and lab names show): `dry-run-3-captures/` `opencode-signed-in`, `kimi-signed-in`, `grok-signed-in`, `zcode-signed-in`, `muse-signed-in`, `hermes-signed-in`, `kimi-session`, `kimi-resumed`, `kimi-resumed-turn2`.

## Cleanup

See the final line below.

Final: Dev (pid 80373) quit with `kill`; `lab_scenario.sh teardown` ran only after `complete` passed ("/tmp/aij-lab-t16-dry removed; nothing of it runs"). Afterwards `pgrep -fl aij-lab` printed nothing (only my own shell), no stand-in process runs, port 1455 empty, lab folder gone; my helper scripts and pollers stopped. `/tmp/aij-lab-dev/AI Juggler Dev.app` left in place.
