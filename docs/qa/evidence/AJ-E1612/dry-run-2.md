# AJ-E1612 sitting 2: dry run 2 on stand-ins (2026-11-16), PARTIAL: stopped when Dev's window left the screen

Driver: sonnet. Lab `/tmp/aij-lab-t16-dry`, `--standins --with-hermes`, 12 Apps, DRY RUN line present. Dev's lab build made from 8b587b471 (contains ad1bd25ae and every fix named in the brief), Dev pid 60979. Selftests (stand-ins, lab_sign_ins, lab_drive guard) passed. `real --compare` and `muse-gate --compare` unchanged throughout. Driver and Dev were used by pid only; no System Events driving.

## Verdict

The fixes work: press by row name, read-label EVIDENCE, the sign-in re-read when the terminal ends, and the footer naming the right tool all PASS for four tools. The run stopped after ZCode's Sign in: from about 16:08 to 16:50 (45 min) `lab_drive` reported `AI Juggler Dev has no window on screen` (accessibility still read Dev's page, CGWindowList had no on-screen window for Dev's pid; the maintainer was switching between other apps: NordPass, ego lite, Claude). `AIJ_DRIVE_MAY_ACTIVATE=1` did not help (it only acts when another app is frontmost). So Muse, Hermes and steps 2-6 are FAIL "not run". No activation happened; nothing was sent while the guard failed.

## Grid

| Tool | sign-in (Dev's Sign in + read-label EVIDENCE) | session | resume | usage | lab B |
|---|---|---|---|---|---|
| OpenCode | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Kimi Code | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Grok Build | PASS | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| ZCode | FAIL (Sign in ran, file in place; EVIDENCE not read: window gone) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Muse Code | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |
| Hermes | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) | FAIL (not run) |

0.4.6 rows: not in the dry run; `handover-0.4.6` and `evidence-confirmed` recorded skip, "dry run".

## What ran

- `read-label "OpenCode CLI · lab A"` before: `EVIDENCE: Home's Needs you has the row "OpenCode CLI · lab A needs its sign-in"`, `STATE: not-signed-in`.
- **Press by name:** `sign-in "OpenCode CLI · lab A"` pressed `Sign in to OpenCode CLI · lab A` with `--row "OpenCode CLI · lab A needs its sign-in"`; calls.log: `opencode auth login` in OpenCode lab A's own folder (`accounts/opencode/opencode-637079736000`, HOME/XDG in the lab). The same for Kimi, Grok, ZCode (each in its own App's folder per `check`).
- **Keys reach Dev's terminal:** YES. OCR showed "Select provider" / "recommended"; `key return` moved to the OpenCode key prompt ("Enter your API key").
- **OpenCode wrong path:** default chosen (OpenCode, API key prompt), `key ctrl-c`; `check` then showed no sign-in in lab A (PASS). The terminal page stayed on screen ended.
- **OpenCode menu path:** `key down`, wait "ChatGPT Plus/Pro or API key" (OpenAI active, by its hint), `key return`, wait "ChatGPT Pro/Plus (browser)", `key return`, wait "Go to": PASS; the fake page finished by itself after about 20 s.
- **After the terminal ended:** with `+8 more need you` pressed first, `read-label "OpenCode CLI · lab A"`: `EVIDENCE: no row "OpenCode CLI · lab A needs its sign-in" on Home's Needs you (by its absence; valid after its sign-in terminal ended)`, `STATE: signed-in`; lab B: `EVIDENCE: Home's Needs you has the row "OpenCode CLI · lab B needs its sign-in"`, `STATE: not-signed-in`. Identical shapes for Kimi Code and Grok Build (press `+7` / `+6 more need you` first).
- **Footer:** "Complete OpenCode's sign-in above, then check the app again from Accounts — it flips to ready." (OpenCode: now names the right tool.) Kimi/Grok footer not read.
- **Guard:** with Claude frontmost and `AIJ_DRIVE_MAY_ACTIVATE=1`, `press Home` printed `WAITING: AI Juggler Dev isn't the frontmost app (Claude is); the Mac is in use (last input 0 s ago; 20 s without input needed). Nothing is sent. …`, then `stopped by the guard … Nothing was sent.`, exit 3. The unset-switch refusal in its pure form was not run separately.
- **Activations: none.** No `ACTIVATED` line was ever printed (the Mac was never idle for 20 s while another app was in front, and later Dev had no window to bring forward). The STOPPED-after-touch case was not exercised.
- **Teardown guard:** NOT exercised this time (my ordering mistake: every cell had a result, honest FAILs, before I ran `teardown`; `complete` passed). It was proven in dry run 1 (32 FAILs, refused).
- OpenCode's Sign in/Kimi/Grok/ZCode pressed one App only each (calls.log checked by `check`).

## Findings (for Opus)

1. **Dev's window can drop out of the CGWindowList while AX still sees it** (`no window on screen`, 45 min). With `--may-activate` the driver does nothing in that state; it should try to activate/unhide Dev (a minimized or other-Space window) or say which. Cause unknown (minimized, hidden or another Space); the run could not recover.
2. **The sidebar's Needs-you list and header go stale** after a sign-in: Home reads "9 things need you" while the sidebar still says `NEEDS YOU 12` and lists lab A's OpenCode/Kimi/Grok rows (see all three captures). Home's own Needs-you is correct.
3. `press "Home"` once failed with `no window on screen` mid-run (a transient dip, recovered), the first sign of finding 1.
4. Sign-in terminals accumulate in the sidebar ("Sign in · lab A" entries, 4 after four sign-ins; the wrong-path one too).

## Labels table

| What | Exact words |
|---|---|
| Needs-you row | `<App> needs its sign-in` (e.g. `OpenCode CLI · lab A needs its sign-in`); description `OpenCode's own sign-in, in a terminal here — about thirty seconds. AI Juggler never sees your password.` |
| Sign in control | `Sign in to <App>` inside that row (sidebar: `… Sign in…`) |
| "+n more" control | `+N more need you` (N falls with each sign-in: 8, 7, 6) |
| App names | `OpenCode CLI · lab A`, `Kimi Code CLI · lab A`, `Grok Build CLI · lab A`, `ZCode CLI · lab A`, `Muse Code CLI · lab A`, `Hermes Agent CLI · lab A` (and lab B) |
| Footer | `Complete OpenCode's sign-in above, then check the app again from Accounts — it flips to ready.` |
| Sign-in terminal | page `AI Juggler · Sign in · lab A`; sidebar `Sign in · lab A` |
| Session row, Stop, Resume, Usage lines, ZCode field | not reached |

Captures (redact_capture with sidecar, `--no-real-data`, each opened by eye and confirmed; only the avatar initial "F" shows): `dry-run-2-captures/opencode-signed-in.jpg`, `kimi-signed-in.jpg`, `grok-signed-in.jpg`. ZCode's capture failed (no window).

## Cleanup

Dev (pid 60979) quit with `kill`; `teardown` ran after `complete` passed ("/tmp/aij-lab-t16-dry removed; nothing of it runs"). Afterwards no `aij-lab-t16` process, no stand-in (`opencode`, `kimi`, `grok`, `muse`, `hermes`, ZCode engine), port 1455 empty, lab folder gone. Other sessions' Dev instances (aij-rc-046, `aij-lab-rc-k2/k4`) were not touched. `/tmp/aij-lab-dev/AI Juggler Dev.app` left in place.

---

Tool output appended during the run (`check` and `real --compare` records):

### check (signed in: opencode)

```text
ok    DRY RUN: every Sign in runs a lab stand-in (<lab>/standins), nothing real
ok    12 Apps, two per tool: OpenCode ×2, Kimi Code ×2, Grok Build ×2, ZCode ×2, Muse Code ×2, Hermes ×2
ok    opencode-a: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637079736000
ok    opencode-b: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637338186000
ok    kimi-a: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637344784000
ok    kimi-b: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637348991000
ok    grok-a: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637352901000
ok    grok-b: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637357180000
ok    zcode-a: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637357892000
ok    zcode-b: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637358813000
ok    muse-a: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-a: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637359782000
ok    muse-b: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-b: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637361742000
ok    hermes-a: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637363083000
ok    hermes-b: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637364798000
ok    AI Juggler Dev's data: 12 Apps, each with its launch settings, nothing else
ok    the throwaway project: <lab>/repos/lab
ok    opencode-a: signed in in its own folder
ok    opencode-b: no sign-in in its own folder
ok    kimi-a: no sign-in in its own folder
ok    kimi-b: no sign-in in its own folder
ok    grok-a: no sign-in in its own folder
ok    grok-b: no sign-in in its own folder
ok    zcode-a: no sign-in in its own folder
ok    zcode-b: no sign-in in its own folder
ok    muse-a: no sign-in in its own folder
ok    muse-b: no sign-in in its own folder
ok    hermes-a: no sign-in in its own folder
ok    hermes-b: no sign-in in its own folder
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### check (signed in: kimi-code, opencode)

```text
ok    DRY RUN: every Sign in runs a lab stand-in (<lab>/standins), nothing real
ok    12 Apps, two per tool: OpenCode ×2, Kimi Code ×2, Grok Build ×2, ZCode ×2, Muse Code ×2, Hermes ×2
ok    opencode-a: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637079736000
ok    opencode-b: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637338186000
ok    kimi-a: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637344784000
ok    kimi-b: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637348991000
ok    grok-a: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637352901000
ok    grok-b: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637357180000
ok    zcode-a: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637357892000
ok    zcode-b: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637358813000
ok    muse-a: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-a: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637359782000
ok    muse-b: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-b: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637361742000
ok    hermes-a: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637363083000
ok    hermes-b: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637364798000
ok    AI Juggler Dev's data: 12 Apps, each with its launch settings, nothing else
ok    the throwaway project: <lab>/repos/lab
ok    opencode-a: signed in in its own folder
ok    opencode-b: no sign-in in its own folder
ok    kimi-a: signed in in its own folder
ok    kimi-b: no sign-in in its own folder
ok    grok-a: no sign-in in its own folder
ok    grok-b: no sign-in in its own folder
ok    zcode-a: no sign-in in its own folder
ok    zcode-b: no sign-in in its own folder
ok    muse-a: no sign-in in its own folder
ok    muse-b: no sign-in in its own folder
ok    hermes-a: no sign-in in its own folder
ok    hermes-b: no sign-in in its own folder
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### check (signed in: grok-build, kimi-code, opencode)

```text
ok    DRY RUN: every Sign in runs a lab stand-in (<lab>/standins), nothing real
ok    12 Apps, two per tool: OpenCode ×2, Kimi Code ×2, Grok Build ×2, ZCode ×2, Muse Code ×2, Hermes ×2
ok    opencode-a: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637079736000
ok    opencode-b: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637338186000
ok    kimi-a: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637344784000
ok    kimi-b: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637348991000
ok    grok-a: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637352901000
ok    grok-b: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637357180000
ok    zcode-a: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637357892000
ok    zcode-b: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637358813000
ok    muse-a: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-a: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637359782000
ok    muse-b: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-b: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637361742000
ok    hermes-a: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637363083000
ok    hermes-b: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637364798000
ok    AI Juggler Dev's data: 12 Apps, each with its launch settings, nothing else
ok    the throwaway project: <lab>/repos/lab
ok    opencode-a: signed in in its own folder
ok    opencode-b: no sign-in in its own folder
ok    kimi-a: signed in in its own folder
ok    kimi-b: no sign-in in its own folder
ok    grok-a: signed in in its own folder
ok    grok-b: no sign-in in its own folder
ok    zcode-a: no sign-in in its own folder
ok    zcode-b: no sign-in in its own folder
ok    muse-a: no sign-in in its own folder
ok    muse-b: no sign-in in its own folder
ok    hermes-a: no sign-in in its own folder
ok    hermes-b: no sign-in in its own folder
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### check (signed in: zcode, grok-build, kimi-code, opencode)

```text
ok    DRY RUN: every Sign in runs a lab stand-in (<lab>/standins), nothing real
ok    12 Apps, two per tool: OpenCode ×2, Kimi Code ×2, Grok Build ×2, ZCode ×2, Muse Code ×2, Hermes ×2
ok    opencode-a: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637079736000
ok    opencode-b: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637338186000
ok    kimi-a: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637344784000
ok    kimi-b: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637348991000
ok    grok-a: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637352901000
ok    grok-b: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637357180000
ok    zcode-a: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637357892000
ok    zcode-b: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637358813000
ok    muse-a: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-a: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637359782000
ok    muse-b: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-b: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637361742000
ok    hermes-a: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637363083000
ok    hermes-b: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637364798000
ok    AI Juggler Dev's data: 12 Apps, each with its launch settings, nothing else
ok    the throwaway project: <lab>/repos/lab
ok    opencode-a: signed in in its own folder
ok    opencode-b: no sign-in in its own folder
ok    kimi-a: signed in in its own folder
ok    kimi-b: no sign-in in its own folder
ok    grok-a: signed in in its own folder
ok    grok-b: no sign-in in its own folder
ok    zcode-a: signed in in its own folder
ok    zcode-b: no sign-in in its own folder
ok    muse-a: no sign-in in its own folder
ok    muse-b: no sign-in in its own folder
ok    hermes-a: no sign-in in its own folder
ok    hermes-b: no sign-in in its own folder
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### check (signed in: zcode, grok-build, kimi-code, opencode)

```text
ok    DRY RUN: every Sign in runs a lab stand-in (<lab>/standins), nothing real
ok    12 Apps, two per tool: OpenCode ×2, Kimi Code ×2, Grok Build ×2, ZCode ×2, Muse Code ×2, Hermes ×2
ok    opencode-a: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637079736000
ok    opencode-b: Sign in runs `opencode auth login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/opencode/opencode-637338186000
ok    kimi-a: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637344784000
ok    kimi-b: Sign in runs `kimi login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/kimi-code/kimi-code-637348991000
ok    grok-a: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637352901000
ok    grok-b: Sign in runs `grok login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/grok-build/grok-build-637357180000
ok    zcode-a: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637357892000
ok    zcode-b: Sign in runs `ZCode <lab>/standins/ZCode.app/Contents/Resources/glm/zcode.cjs login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/zcode/zcode-637358813000
ok    muse-a: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-a: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637359782000
ok    muse-b: the file backend (TBH_CREDENTIAL_BACKEND=file), HOME and every XDG root in its folder, MUSE_NO_AUTO_UPDATE=1, no MUSE_AUTH_PATH
ok    muse-b: Sign in runs `muse login` in <lab>/Library/Application Support/AI Juggler Dev/accounts/muse-code/muse-code-637361742000
ok    hermes-a: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637363083000
ok    hermes-b: Sign in runs `hermes model` in <lab>/Library/Application Support/AI Juggler Dev/accounts/hermes-agent/hermes-agent-637364798000
ok    AI Juggler Dev's data: 12 Apps, each with its launch settings, nothing else
ok    the throwaway project: <lab>/repos/lab
ok    opencode-a: signed in in its own folder
ok    opencode-b: no sign-in in its own folder
ok    kimi-a: signed in in its own folder
ok    kimi-b: no sign-in in its own folder
ok    grok-a: signed in in its own folder
ok    grok-b: no sign-in in its own folder
ok    zcode-a: signed in in its own folder
ok    zcode-b: no sign-in in its own folder
ok    muse-a: no sign-in in its own folder
ok    muse-b: no sign-in in its own folder
ok    hermes-a: no sign-in in its own folder
ok    hermes-b: no sign-in in its own folder
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### complete: every step has a result

```text
ok    OpenCode: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): pass (Dev Sign in by name; menu OpenAI -> ChatGPT Pro/Plus (browser); read-label EVIDENCE a absent->signed-in, b present->not-signed-in; sidebar Needs you stayed 12 (stale) while Home said 11)
ok    OpenCode: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    OpenCode: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    OpenCode: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    OpenCode: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Kimi Code: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): pass (Dev Sign in by name; device-code terminal; read-label EVIDENCE a absent->signed-in, b present; sidebar Needs you stale at 12 vs Home 10)
ok    Kimi Code: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Kimi Code: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Kimi Code: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Kimi Code: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Grok Build: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): pass (Dev Sign in by name; read-label EVIDENCE a absent->signed-in, b present)
ok    Grok Build: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Grok Build: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Grok Build: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Grok Build: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    ZCode: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): fail (Dev Sign in pressed by name and sign-in file in place (check passes), but read-label EVIDENCE not obtained: not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    ZCode: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    ZCode: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    ZCode: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    ZCode: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Muse Code: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Muse Code: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Muse Code: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Muse Code: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Muse Code: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Hermes: 1. the sign-in stays separate (Dev's Sign in, its Signed in label, check, real, muse-gate): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Hermes: 2. a session starts and answers: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Hermes: 3. it resumes with its earlier turn: fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Hermes: 4. usage is read (what it shows, its source line): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    Hermes: 6. lab B stays out (Not signed in; its session refused): fail (not run: Dev's window left the screen (no window on screen for ~45 min, from about 16:08 to 16:50, maintainer using the Mac; the driver sent nothing))
ok    The sitting: the 0.4.6 session's rows done (its Grok and ChatGPT sign-ins, its OpenCode check on T16 lab A): skip (dry run)
ok    The sitting: the coordinating session confirmed the evidence: skip (dry run)
```

### the real home's sign-in files

```text
ok    ~/.config/muse/auth.json: unchanged
ok    ~/.grok/auth.json: unchanged (not there)
ok    ~/.grok/mcp_credentials.json: unchanged (not there)
ok    ~/.hermes/.env: unchanged (not there)
ok    ~/.hermes/auth.json: unchanged (not there)
ok    ~/.hermes/shared/nous_auth.json: unchanged (not there)
ok    ~/.kimi-code/credentials: unchanged (not there)
ok    ~/.local/share/opencode/auth.json: unchanged (not there)
ok    ~/.local/share/opencode/mcp-auth.json: unchanged (not there)
ok    ~/.zcode/v2/credentials.json: unchanged
ok    ~/.codex/auth.json (digest, size, date): unchanged
```

### Muse's gate (after)

```text
ok    Muse 1.4.13-R6996.1
ok    the real Keychain item is unchanged: the lab's sign-in never reached it
```
