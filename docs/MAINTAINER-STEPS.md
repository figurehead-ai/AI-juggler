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
team switcher at the top right must say **Vuxtra LLC**: your Apple ID
also belongs to Advancedcompany.com, whose membership has expired.

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

- [ ] A 1024×1024 PNG brand icon. Hand it over, or replace
      `crates/juggler-app/icons/icon.png`. The build generates every
      size from it.

---

## Part 2 — Once the first signed build exists

### 7. First release and rollback rehearsal (AJ-E1010, 30 min)

**Why:** this proves that signing, notarization, the update feed and
rollback all work before any user depends on them.

- [ ] GitHub → **Actions** → **Nightly** → **Run workflow** (tick
      *force*). It builds, signs and notarizes the next version, publishes
      it as a pre-release, and points the nightly feed at it. No approval.
- [ ] Install that version's `.dmg` on a clean Mac (or a new macOS user
      account). Gatekeeper must open it without warnings. Settings ›
      About & updates shows the version and the channel picker.
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
