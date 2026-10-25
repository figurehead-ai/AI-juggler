# Maintainer steps

The things only the maintainer can do: they need the maintainer's
accounts, machines, judgement or testers. Everything else is automated
or done by agents. Work top to bottom; each step says why it matters,
exactly what to do, and what happens next.

Status as of 2026-10-19. Tick the boxes as you go.

---

## Part 1 — Set up once (about an hour in total)

### 1. Turn on secret scanning (2 min, needs an org owner)

**Why:** the repository is public. Secret scanning finds credentials
committed by mistake, and push protection blocks the push before it
lands. The organisation's "No-cost security baseline" is enforced on
every repo and has both switched off, so they can't be turned on from
the repository itself.

- [ ] Your profile picture (top right) → **Organizations** →
      **figurehead-ai** → **Settings**.
- [ ] In the sidebar's **Security** section, open **Advanced Security**
      → **Configurations**, then the **Repositories** tab.
- [ ] Tick **AI-juggler** only → **Apply configuration** → **GitHub
      recommended** → review → **Apply**. This also turns on CodeQL code
      scanning and private vulnerability reporting. For a public
      repository all of it is free, and the baseline still governs every
      other repo.
- [ ] Check: AI-juggler → **Settings** → **Advanced Security** shows
      Secret Protection and Push protection as enabled.

GitHub renames these menus from time to time. If they don't match, the
current steps are in GitHub's guide [Applying a custom security
configuration](https://docs.github.com/en/code-security/how-tos/secure-at-scale/configure-organization-security/establish-complete-coverage/applying-a-custom-security-configuration).

*Or* let an agent do it: run `gh auth refresh -h github.com -s admin:org`
in your terminal once, then ask.

### 2. The app's bundle id ✓ decided: `ai.figurehead.juggler`

Decided 2026-10-19 and set in `crates/juggler-app/tauri.conf.json`. The
bundle id is the app's permanent identity on macOS. It doesn't have to
match the signing team: Vuxtra LLC's certificate signs it today, and if
Figurehead later has its own Apple account, only the signer changes.
No App ID registration is needed. The app ships outside the Mac App
Store and uses no Apple capabilities. Register one later only if that
changes.

### 3. Apple: certificate, team, notarization (about 30 min)

You need the Apple Developer account of Vuxtra LLC, signed in as its
**Account Holder** (or an Admin allowed to create Developer ID
certificates).

**a. Team ID** ✓ — `TH8VS427L5` (Vuxtra LLC). `APPLE_TEAM_ID` and
`APPLE_SIGNING_IDENTITY` are already set in GitHub. In the portal, the
team switcher at the top right must say **Vuxtra LLC** if your Apple ID
belongs to more than one team.

**b. Developer ID Application certificate.** This is the certificate for
apps distributed outside the Mac App Store.
- [ ] On your Mac: **Keychain Access** → menu **Keychain Access** →
      **Certificate Assistant** → **Request a Certificate From a
      Certificate Authority…** → your email, common name "Vuxtra LLC",
      **Saved to disk**.
- [ ] developer.apple.com/account/resources/certificates → **+** →
      **Developer ID Application** → **G2 Sub-CA** → upload the request
      → download the `.cer` and double-click it.
- [ ] Keychain Access → **My Certificates** → right-click
      "Developer ID Application: Vuxtra LLC (TEAMID)" → **Export…** →
      `.p12` with a strong password. Keep the file and password in your
      password manager.
- [ ] Base64-encode it for GitHub: `base64 -i DeveloperID.p12 | pbcopy`.

**c. App ID** — not needed (see step 2).

**d. Notarization password**
- [ ] account.apple.com → **Sign-In and Security** → **App-Specific
      Passwords** → **+** → name it "AI Juggler notarization". Use the
      Apple ID of a team member allowed to notarize.

### 4. The updater signing key (5 min)

**Why:** installed apps only accept updates signed with this key. If it
is lost, every installed copy has to be reinstalled by hand. Keep it
outside the repository, with a backup.

- [ ] In the repository's `ui/` folder:
      `pnpm tauri signer generate -w ~/.tauri/ai-juggler-updater.key`
      Choose a password when asked. It prints the **public key** and
      writes the private key file plus a `.pub` file.
- [ ] Store the private key file and its password in your password
      manager.

### 5. Add everything to GitHub (5 min)

**Quickest:** run `scripts/set_release_secrets.sh` (it takes the `.p12`
path, default `~/Desktop/DeveloperID.p12`). It reads the `.p12` and the
updater key from their files and asks for each password at gh's hidden
prompt. It generates the updater key (step 4) if you don't have one yet.
Or do it by hand:

github.com/figurehead-ai/AI-juggler → **Settings** → **Secrets and
variables** → **Actions**.

| Name | Where | Value |
|---|---|---|
| `APPLE_CERTIFICATE` | Secrets | the base64 text from 3b |
| `APPLE_CERTIFICATE_PASSWORD` | Secrets | the `.p12` password |
| `APPLE_SIGNING_IDENTITY` | Secrets | ✓ already set: `Developer ID Application: Vuxtra LLC (TH8VS427L5)` |
| `APPLE_ID` | Secrets | the Apple ID email used in 3d |
| `APPLE_PASSWORD` | Secrets | the app-specific password from 3d |
| `APPLE_TEAM_ID` | Secrets | ✓ already set: `TH8VS427L5` |
| `TAURI_SIGNING_PRIVATE_KEY` | Secrets | the contents of `~/.tauri/ai-juggler-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Secrets | its password |
| `AIJ_UPDATER_PUBKEY` | **Variables** | the contents of the `.pub` file (public by design) |

- [ ] Tell the agent when they're in. It then runs the first release
      through the new channels (nightly → beta → stable, decision D-20).

### 6. The app icon (when you have it)

- [x] Done (2026-10-22): `scripts/app_icon.swift` draws the icon as
      vectors at 1024×1024 on Apple's grid, into
      `crates/juggler-app/icons/icon.png`. The build generates every size
      from it. To change the artwork, edit the script and re-run it.

---

## Part 2 — Once the first signed build exists

### 7. First release and rollback rehearsal (AJ-E1010, 30 min)

**Why:** this proves that signing, notarization, the update feed and
rollback all work before any user depends on them.

- [ ] GitHub → **Actions** → **Nightly** → **Run workflow** (tick
      *force*). It builds, signs and notarizes the next version, publishes
      it as a pre-release, and points the nightly feed at it. No approval.
- [ ] The release has two apps (D-23): `AI-Juggler-X.Y.Z-universal.dmg`
      and `AI-Juggler-Nightly-X.Y.Z-universal.dmg`. Install **both** on
      your Mac; they sit side by side in Applications and share nothing.
      Gatekeeper must open each without warnings. AI Juggler's Settings ›
      General › About & updates shows the version and the Stable/Beta picker; AI
      Juggler Nightly's shows "nightly channel" and a **Nightly** badge
      in the sidebar. Use AI Juggler for real work and Nightly to try
      new builds: each keeps its own copies of your Accounts. The
      shortcut that opens AI Juggler from anywhere has no default; if
      you set one, give Nightly a different one (Settings › Desktop apps).
- [ ] Use AI Juggler Nightly for at least 3 days of real work (D-24).
      Report what you find; anything that loses data, uses the wrong
      account, stops the wrong process, crashes at launch or breaks an
      update gets the `blocker` label.
- [ ] Write its user notes (`docs/releases/X.Y.Z.md`; the agent drafts
      them for you to approve), then **Actions** → **Promote** → version
      `X.Y.Z`, channel `beta`. When the run waits for you, open it and
      click **Review deployments → Approve**.
- [ ] Back up from Settings and keep the file.
- [ ] Rollback rehearsal: follow `docs/RELEASING.md` § Rollback (let
      nightly make a second version first).

Stable comes later: **Promote** with channel `stable` only accepts a
version that was on beta, and only once the beta gate is signed (Part 5).

### 8. Measurements on your Mac (AJ-E805, 20 min)

**Why:** the performance numbers so far come from test harnesses. These
two come from the real, signed app.

- [ ] **Cold start:** quit AI Juggler, then time from double-click until
      the rail shows your Instances. Do it 5 times and note the middle
      value. Target: under 2.5 s.
- [ ] **Idle CPU and memory:** start 10 real Instances (any Accounts),
      leave them idle, then run this in Terminal (10 minutes):

      ```bash
      for i in $(seq 1 10); do ps -o %cpu=,rss= -p "$(pgrep -f 'AI Juggler.app/Contents/MacOS' | head -1)"; sleep 60; done
      ```

      It prints CPU % and memory in KB once a minute, for the app
      itself and not its children. Targets: CPU under 2 %, memory under
      600 MB.
- [ ] Send the agent the numbers and your Mac's model. It files them in
      `docs/qa/evidence/AJ-E805/`.

### 9. Production Figurehead link (AJ-E1202, 5 min)

- [ ] In the signed app: **Settings → Account & Cloud → Connect
      account**. Your browser opens figurehead.ai. Sign in and approve
      this Mac.
- [ ] The app shows **Connected** with your email and plan.
- [ ] Tell the agent it worked (or what it showed). Never paste the
      token anywhere: it lives only in the Keychain.

### 10. Crash reports service (AJ-E1107, 20 min)

**Why:** crash reports are scrubbed on the Mac and sent only if the user
allows it, but there is no service to receive them yet.

- [ ] Create a crash-reporting project (the spec names Sentry; native /
      Rust platform) under the company account. Set a rate limit.
- [ ] Copy its ingestion address (the DSN) and add it as a repository
      secret named `AI_JUGGLER_CRASH_ENDPOINT`. Release builds read it at
      compile time; debug builds never send.
- [ ] Tell the agent. It wires the release build to it and runs the
      native-crash test with you on a signed build.

---

## Part 3 — Every release

### 11. VoiceOver pass (AJ-E804, 30 min)

**Why:** automated checks can't tell whether announcements make sense
when heard.

- [ ] Follow `docs/qa/evidence/AJ-E804/voiceover-script.md` on the
      release candidate (⌘F5 turns VoiceOver on).
- [ ] Fill its findings table and commit it, or send the agent your
      notes. Anything that doesn't match "Expect to hear" becomes a
      task.

---

## Part 4 — The dogfood period (4 weeks)

### 12. Live switch runs for the fidelity register (AJ-E808)

**Why:** a Provider pair may only show "Validated" when every area has
passed on real binaries. No live run exists yet, so every pair shows
"Beta · Not yet validated".

- [ ] Use a throwaway project and two real Accounts. For each pair you
      use (start with Claude Code → Claude Code), switch a conversation
      that has rules, todos, a subagent, edited files and MCP servers,
      in a dotted or long path. Then switch back (A→B→A).
- [ ] For each area, note what carried and what didn't, and save the
      log or screen recording to
      `docs/qa/evidence/AJ-E808/cells/<from>-<to>-<area>`.
- [ ] Hand the notes to the agent. It records each cell (PASS or FAIL
      with versions, date and evidence), and the tests refuse a cell
      without them.

### 13. The two-Mac Fleet runs (AJ-E710)

Needs two Macs on one network (a third for the board test, or a
waiver). The list is in `docs/qa/gate-E7.md` § Dogfood:

- [ ] Pair the two Macs (under 60 s), then repeat over Tailscale.
- [ ] Leave a real `vite` dev server behind on one Mac, then clean it
      from the other.
- [ ] Focus a desktop agent's window on the other Mac.
- [ ] Take the laptop offline for a day, then reconnect.
- [ ] Laptop → desktop → laptop switch, with the continuity checklist.
- [ ] Two testers sign off.

Screen recordings and timings go to the agent, which files them and
fills the gate.

### 14. The tester programme (AJ-E809)

- [ ] Recruit at least 5 testers covering the three personas in
      `docs/qa/evidence/AJ-E809/programme.md` (multi-subscription solo,
      parallel power user, consultant).
- [ ] Have each sign the consent checklist in that file.
- [ ] Each week, fill one row per task run in `runs.csv`
      (`run-sheet.md` explains the columns), or send the notes to the
      agent. Never record credentials, transcripts or real names.
- [ ] Fridays: `just dogfood-report`. The agent files each friction
      note as a task within 48 hours.
- [ ] The live runs in `docs/qa/dogfood-backlog.md` fit into the same
      sessions.

---

## Part 5 — Signing the gates (AJ-E807)

**Why:** a signature records that you reviewed the evidence at a
specific commit. The beta update feed doesn't move until the beta gate
is signed.

- [ ] For M0, M1, M3, E5 and E7: read the gate file in `docs/qa/`. If
      you accept it, fill its **Sign-off** table: Decision `APPROVED`,
      Reviewer (your name), Date (`YYYY-MM-DD`), Commit (the current
      `main` commit). `git rev-parse --short origin/main` prints it.
- [ ] Beta: run `just gate-packet`, read `docs/qa/gate-beta.md`, and sign
      it only when every criterion is PASS. `just gate-signed` confirms
      the signature is complete.

---

## Part 6 — As UI changes land

### 15. Choose the visual check (D-21)

**Why:** the required `visual` check only compares the core screens.
A screen that is new or rebuilt, or a change to something every screen
shares, needs the whole gallery compared.

- [ ] For such a pull request, run the full sweep:
      `gh workflow run visual-sweep.yml -f head='#<number>'` (or
      Actions → visual-sweep → Run workflow). It takes about 15
      minutes.
- [ ] Open the run's summary: every changed state listed should be one
      the change meant to touch. The `visual-sweep-report` artifact
      holds the base/head/diff images (`index.html`).
- [ ] Anything unexpected: ask for a fix before merging. Otherwise
      nothing else to do — routine changes need only the core check.

---

## Part 7 — As new tools land

### 16. Live checks of new tools on spare accounts (AJ-E1612)

**Why:** every new tool is built from its source and fixtures only. One
short run on an account that doesn't matter proves what really happens,
and the evidence records it. Use a throwaway project, and sign-ins you
don't mind losing (a free model, or a small OpenCode Zen balance).

#### OpenCode (AJ-E1602; about 30 min per version)

Built from the 1.19.3 and 2.1.2 source. If you can, run it once on each
(`brew install anomalyco/tap/opencode` gives 1.x;
`anomalyco/tap/opencode-v2` gives 2.x — they replace each other).

- [ ] **Found.** Accounts › Add account lists OpenCode with its version
      ("not yet checked live" is expected until this step is filed).
- [ ] **Two separate sign-ins.** Add two fresh OpenCode accounts, A and
      B. Sign A in from AI Juggler (`opencode auth login` opens in a
      terminal); leave B signed out. A shows signed in, B doesn't, and
      `opencode auth list` run by hand (outside AI Juggler) doesn't
      list A's sign-in.
- [ ] **A session starts and resumes.** Start a session on A in the
      throwaway project and send one message. Stop it, then resume it
      from AI Juggler: the same conversation comes back. Resume with a
      message that starts with "-" (for example `- go on`): it arrives
      as text, not as an option.
- [ ] **One-off runs.** If you use headless runs, start one with a
      model set in Launch settings: it uses that model, and a prompt
      starting with "-" is text (this checks `run --model=… -- <text>`,
      which 2.x has not been seen to accept yet).
- [ ] **Model in the terminal.** With a model in Launch settings, a
      normal session opens on that model (set through
      `OPENCODE_CONFIG_CONTENT`; 2.x's terminal has no `--model`).
- [ ] **Usage is read.** Within about 10 seconds of a reply, the
      account's Usage shows its tokens, with OpenCode's dollar figure as
      an estimate, and the limit rings say "OpenCode doesn't report its
      limits to AI Juggler yet".
- [ ] **Its sessions are listed.** A session you start by hand in a
      terminal (`opencode` in the throwaway project, on the default
      folder) shows in the rail's sessions from outside AI Juggler within
      a minute, with its folder, and is marked running while it runs;
      Copy resume command gives `opencode --session <id>`.
- [ ] **Rules and servers sync.** In the throwaway project, Sync shows
      OpenCode's `AGENTS.md` beside the others. Copy an MCP server from a
      Claude account to account A: it appears in A's
      `config/opencode/opencode.json` under `mcp`, OpenCode lists it
      (`opencode mcp list`), and Undo takes it out again.
- [ ] **Quota exhaustion is seen.** Run a Zen balance or a Go plan to its
      limit (or use a key with a tiny spend limit). Note the exact
      message OpenCode shows, and whether AI Juggler shows the session
      as needing you.
- [ ] **The variables reach the agent's tools.** In an OpenCode session,
      ask it to run `gh auth status` and `git config user.name`. Note
      the answers: the account's own `XDG_CONFIG_HOME` hides `gh`'s
      sign-in (a known trade-off of this isolation; `git` reads
      `~/.gitconfig` and should be fine).
- [ ] **A copy keeps no sign-in.** Bring your default OpenCode folder
      in as a copy. The copy is signed out; `opencode session list` in
      the copy (Accounts › the copy › the expert shell line) lists your
      sessions; your original still works and is signed in.
- [ ] **Used where it is, unwritten.** Bring the default folder in
      where it is. With OpenCode closed, run
      `ls -la ~/.local/share/opencode` before and after a few minutes of
      AI Juggler running: no new files (no `opencode.db-wal`).

Save the terminal output and a screen recording per check to
`docs/research/evidence/opencode/<version>/`, then hand them to the
agent: it files them, sets the verified versions and flips the
"not yet checked live" note.

#### Kimi Code (AJ-E1603; about 30 min)

Built from the 2.3.0 docs and source. Install it the vendor's way
(`curl -fsSL https://code.kimi.com/kimi-code/install.sh | bash`, or
`npm install -g @moonshot-ai/kimi-code`). Use a spare Kimi membership
(Plus or above) and, if you have one, a Kimi Platform API key with a small
balance.

- [ ] **Found.** Accounts › Add account lists Kimi Code with its version
      ("not tested with AI Juggler yet" is expected until this step is
      filed). Note `kimi --version`'s exact output.
- [ ] **Two separate sign-ins.** Add two fresh Kimi Code accounts, A and
      B. Sign A in from AI Juggler (`kimi login` opens in a terminal);
      leave B signed out. A shows signed in, B doesn't. Note the files
      `kimi login` made in A's `credentials/` (names only): the membership
      sign-in is expected as `kimi-code.json`.
- [ ] **Devices.** In the Kimi Code Console, note how many devices the
      membership shows after A's sign-in (each home is expected to count
      as one device) and whether a device cap is stated.
- [ ] **A session starts and resumes.** Start a session on A in the
      throwaway project and send one message. Stop it, then resume it from
      AI Juggler: the same conversation comes back
      (`kimi --session <id>`). Note the lines of `session_index.jsonl` and
      the fields of that session's `state.json` (names only, no content).
- [ ] **Its sessions are listed.** A session you start by hand
      (`kimi` in the throwaway project) shows in the rail's sessions from
      outside AI Juggler, with its folder and title, and is marked running
      while it runs.
- [ ] **Interactive use only.** With Settings › Sessions › Restart
      crashed sessions on, end A's running `kimi` with `kill -9`: it is not
      started again, and its history says why. A headless run on A is
      refused with the same words.
- [ ] **An API key may run headless.** On a third account signed in with a
      Kimi Platform API key (`kimi login`, "Kimi Platform API key"), note
      the file it made in `credentials/` (name only). A headless run with a
      prompt starting with "-" works and the prompt arrives as text
      (`kimi -p <text> --output-format stream-json`).
- [ ] **Usage.** The account's limits say "Kimi Code doesn't report its
      limits to AI Juggler yet". Note what `/usage` in Kimi Code shows, for
      the ruling on its quota source.
- [ ] **Quota exhaustion is seen.** If a 5-hour limit is reached, note the
      exact message and whether AI Juggler shows the session as needing
      you.
- [ ] **A copy keeps no sign-in.** Bring `~/.kimi-code` in as a copy: the
      copy is signed out, has your settings, skills and MCP servers, and
      none of your sessions; your original still works and is signed in.
- [ ] **Rules and servers sync.** Copy an MCP server from a Claude account
      to A: it appears in A's `mcp.json` and in Kimi Code's `/mcp`, and
      Undo takes it out again.

Save the output and recordings to `docs/research/evidence/kimi-code/2.x/`
and hand them to the agent, as for OpenCode.

#### Grok Build (AJ-E1604; about 30 min)

Built from the 1.0.45 docs and source. Install it the vendor's way
(`curl -fsSL https://x.ai/cli/install.sh | bash`), on a spare X
Subscription account (or an xAI API key with a small balance).

- [ ] **Found.** Add account lists Grok Build with its version. Note
      `grok --version`'s exact output. If you also have the unofficial
      `grok-cli` (`npm i -g grok-dev`), it isn't taken for Grok Build.
- [ ] **Two separate sign-ins.** Two fresh accounts, A and B: sign A in
      from AI Juggler (`grok login`), leave B signed out. A shows signed
      in, B doesn't, and your own `~/.grok` sign-in is unchanged.
- [ ] **Your Claude setup stays out.** With a Claude `PreToolUse` hook
      that writes a file, run a Grok session on A that edits a file: the
      hook doesn't fire. `grok inspect` in that session lists no rules,
      MCP servers or skills from `~/.claude` or `~/.cursor`.
- [ ] **A session starts and resumes.** Start one on A in the throwaway
      project, send a message, stop it, resume it from AI Juggler
      (`grok --resume <id>`): the same conversation. Note the fields of
      its `summary.json` and `signals.json` (names only), and whether a
      sub-agent's session names its parent there.
- [ ] **Its sessions are listed.** A session started by hand shows in the
      rail's outside sessions, with its folder and title, marked running.
- [ ] **A model in Launch settings** reaches the session (`--model`).
- [ ] **Usage.** Its limits say "Grok Build doesn't report its limits to
      AI Juggler yet". Note what `/usage` shows, and the exact message
      when the weekly pool runs out (a 402 is expected).
- [ ] **A copy keeps no sign-in.** Bring `~/.grok` in as a copy: signed
      out, settings, rules and sessions there; the original still works.

Save the output to `docs/research/evidence/grok-build/<version>/` and
hand it to the agent.

#### New tool versions (AJ-E1611; 5 min, on an installed build)

- [ ] About 10 minutes after AI Juggler starts, each tool's page (More ›
      Tools) says nothing, or "<tool> <version> is out" with its update
      command. Note one tool's line, and compare the version with
      `npm view <package> version` in a terminal (`@anthropic-ai/claude-code`,
      `@openai/codex`, `opencode-ai`, `@moonshot-ai/kimi-code`): the check
      asks `https://registry.npmjs.org/<package>/latest`, with a scoped
      name's `/` written `%2F` (not yet seen live).
- [ ] Untick "Look for new versions of your tools once a day", restart AI
      Juggler, and after 15 minutes check with Little Snitch or `nettop`
      that it made no request to registry.npmjs.org.

---

## Part 8 — Pools and the agent bridge

### 17. The agent bridge spike on spare accounts (AJ-E1701 Req.2; about 30 min)

**Why:** AJ-E1704 built the bridge to the vendors' documented behaviour,
and its tests run on the fake tool only. Three things only a real run
can show, and the evidence marks them **unverified** until you've done
this. Use spare accounts, a throwaway lab project, and AI Juggler Dev
(`just dev`, which builds `juggler-agent` beside it). Allow nothing,
sign in to nothing new.

- [ ] In AI Juggler Dev, open any pool's page and answer **Yes, let them**
      to "Let agents in your projects see each other and work together?".
- [ ] **Claude Code.** Start a session on a spare Claude Code account in
      the lab project. Ask it: "Call the juggler whoami tool."
  - [ ] Does it run **without asking you to approve** the server or the
        tool? (`--mcp-config` + `--allowedTools`; record any prompt
        word for word.)
  - [ ] The answer names this session and the project's pool.
  - [ ] `/mcp` lists `juggler` as connected, and your own MCP servers
        are still there (no `--strict-mcp-config`).
- [ ] **Codex.** The same on a spare Codex account: "Call the juggler
      whoami tool."
  - [ ] Does the key reach the server (`env_vars`)? A key that didn't
        arrive answers "This session wasn't started by AI Juggler with
        its agent tools, so it has no key".
  - [ ] Does it run **without an approval prompt**
        (`tools.<tool>.approval_mode="approve"`)? Record any prompt.
- [ ] Both: ask each to `set_status` "spike", then open the pool's page:
      both cards say it. Ask one for `pool_roster`: it lists the other.
- [ ] Optional: which MCP version each sent (Claude Code's `--debug`
      output or Codex's log names it); the bridge answers both lines.
- [ ] Put the captures in `docs/research/evidence/` (redacted with the
      fail-closed tool) and tick AJ-E1701 Req.2 and the three
      "unverified" lines in `docs/qa/evidence/AJ-E1704/manifest.md`.

If a prompt appears, say which: the fix is in the adapter's
`claude_bridge.rs` or `codex_bridge.rs` only.
