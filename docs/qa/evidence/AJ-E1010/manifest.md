# Evidence manifest — AJ-E1010

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1010 (updates and
release integrity for beta). Status: **IN-PROGRESS**. Everything that can be
built and tested without the maintainer's signing identity is done. The
task's proof is "all four requirements demonstrated on a real beta build",
and that needs a signed, notarized release, which only the maintainer can
produce (`docs/RELEASING.md` § One-time setup). 2026-10-18, reviewer
claude-opus.

## Where it started

Before this task the repository had no release machinery: bundling was
off, nothing was signed or notarized, and there was no updater and no
release workflow. The CHANGELOG existed but had no release process around
it.

## What exists now

- **App.** Settings › **About & updates** shows the version and a **Beta**
  label on every state.
  - Release builds (with the updater key baked in) check the beta feed when
    they start, a setting that is on by default and can be turned off.
  - A check only reports: the version and its notes, rendered from the
    release's Markdown as plain elements.
  - **Install** writes a backup of AI Juggler's data first (AJ-E1009), then
    downloads, verifies the minisign signature, and installs. Any failure
    leaves the running version untouched, gives the reason, and offers
    **Retry**.
  - Install waits while a switch is in flight. Restart is offered, never
    forced, and is refused while an Instance is running.
  - The updater is configured with `requireSignedVersion`, which blocks a
    feed that pairs a new version number with an older signed package.
  - Development builds never check.
  - A one-time notice per version ("Nothing is installed until you choose")
    links to Settings.
- **Workflows.**
  - `release.yml`, on a `vX.Y.Z` tag, fails closed without every secret,
    matching versions, a CHANGELOG section and user-facing notes. It then
    builds universal, signs, notarizes, verifies (codesign, Gatekeeper,
    stapler, updater signature), names the installer
    `AI-Juggler-X.Y.Z-universal.dmg`, writes `latest.json` and
    `SHA256SUMS.txt`, and drafts a pre-release.
  - `release-channel.yml` moves the beta feed when a release is published,
    and rolls it back when run with an earlier tag.
- **Runbook.** `docs/RELEASING.md` covers setup, each release, the
  release-notes rules (`scripts/release_notes_check.sh`), rollback with a
  rehearsal, and database changes between versions.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Signed artefacts with a changelog entry per release; update check on launch (user-disableable); version and changes shown before installing; never a silent auto-install | Req.1 | `updates :: no_signing_key_or_check_off…`: launch check on by default; turned off, no request; a check reports version and notes and installs nothing. vitest `updates.test.tsx`: changes shown before Install; install only on the click. `release.yml` requires signing secrets, a CHANGELOG section and the notes file before building. **Signed artefacts on a real build: pending the maintainer's secrets** | PARTIAL (signed build pending) | 2026-10-18 | claude-opus |
| A rollback path documented and rehearsed once per channel; migrations forward-compatible or with a downgrade note | Req.2 | `docs/RELEASING.md` § Rollback (move the channel back, reinstall the previous `.dmg`, restore the pre-update backup) and § Database changes. The pre-update backup is real (`a_bad_signature_never_installs…` asserts it exists). Migration forward-compatibility: the storage round-trip tests open every earlier schema. **Rehearsal: pending the first signed builds** | PARTIAL (rehearsal pending) | 2026-10-18 | claude-opus |
| Beta labelled in-app (About and the update prompt), consistent with D-04/D-11 | Req.3 | A Beta chip on every About & updates state; "beta channel"; the prompt reads "AI Juggler X (beta) is available". Gallery `58-updates` | PASS | 2026-10-18 | claude-opus |
| A failed update (checksum, signature or network) leaves the running version untouched and reports the reason with Retry | Req.4 | `a_bad_signature_never_installs_and_retry_works`: a signature failure → `failed`, the reason names the signature and says the version is untouched, install is never called, Retry then installs. The real path relies on the plugin's `download`/`install`, which both verify the signature before returning or replacing anything | PASS (fake feed); real-build demonstration pending | 2026-10-18 | claude-opus |
| Update check < 1 s in the background, no launch blocking; download progress at least every 5 %; check off = zero network calls (verified) | perf | The launch check runs on its own thread 5 s after start and never blocks the window. Progress is emitted at every whole percent; the test asserts steps ≤ 5 % and ending at 100. With the check off, or no key (any development build), the fake feed sees **0** requests | PASS | 2026-10-18 | claude-opus |
| A signature-failure fixture refuses install; rollback rehearsed per release; migration fixture | quality gate | Signature fixture as Req.4. `actionlint` clean (`actionlint.log`, info notes only). Rollback rehearsal: pending | PARTIAL | 2026-10-18 | claude-opus |
| Visual: update-available, downloading, failed and up-to-date states, 1280×800, both themes; version and changes before install; beta label; no modal trap | visual / a11y | Gallery `58-updates` (available, downloading, failed, up to date, development build × 2 themes). The card is inline, with no modal. Playwright `updates card — axe 0`: keyboard from Install to Check now | PASS | 2026-10-18 | claude-opus |
| Signed-artefact verification log and rollback rehearsal on a real beta build | evidence | Pending: the maintainer runs `release.yml` once the secrets are set, then the rehearsal in `docs/RELEASING.md` | PENDING (maintainer) | 2026-10-18 | claude-opus |

## Test counts

- `cargo test -p juggler-app --test updates`: 3/3 (`update-policy.log`).
- vitest `updates.test.tsx` 4/4. Playwright `58 updates` 10,
  `updates card — axe 0` 2.

## Superseded by D-20 (2026-10-19)

The single beta channel built from a version tag is replaced by three
channels on one version stream. `release.yml` and `release-channel.yml`
are gone. `build-macos.yml` keeps every build step recorded above
(secrets fail closed, signing, notarization, the updater signature,
`SHA256SUMS.txt`), now called by `nightly.yml`. `promote.yml` moves the
beta and stable feeds with the maintainer's approval. The app chooses its
channel in Settings (`updates :: the_channel_is_a_persisted_choice_with_its_own_feed`,
vitest `updates.test.tsx`). The signed-build and rollback rows stay
pending on the maintainer's secrets.

## Editions, D-23 (2026-10-19)

AI Juggler, AI Juggler Nightly and AI Juggler Dev install side by side
and share nothing. Evidence: `editions.log`.

| Requirement | Proof | Result |
|---|---|---|
| The edition follows the bundle id; a debug build or any unexpected id is Dev | `edition::tests` (5) | PASS |
| Editions share no identifier, data folder, Keychain item or Fleet port | `edition::tests::editions_share_nothing` | PASS |
| AI Juggler keeps its existing folder, Keychain items and port (no migration) | `edition::tests::ai_juggler_keeps_its_existing_folder_keychain_items_and_port` | PASS |
| Nightly: own data, Keychain, port 47822; nightly only, a stored beta can't move it | `edition_nightly` (own process) | PASS |
| Dev: own data, Keychain, port 47823; no channel, no updates | `edition_dev` (own process) | PASS |
| AI Juggler offers Stable and Beta only; nightly is refused with a pointer to the Nightly app | `updates :: the_channel_is_a_persisted_choice_with_its_own_feed` | PASS |
| Settings names the edition, shows only its channels, explains Nightly and Dev | vitest `updates.test.tsx` 8/8 | PASS |
| A real Dev build creates only `AI Juggler Dev` (launched with a throwaway HOME) | manual run, 2026-10-19: `Application Support/` held only `AI Juggler Dev` (`juggler.sqlite` + WAL) | PASS |
| Nightly run builds, signs and notarizes both apps; nightly feed carries the Nightly app, beta/stable never do | Nightly [run 39288351167](https://github.com/figurehead-ai/AI-juggler/actions/runs/39288351167) → release `v0.0.4` (2026-10-19): both editions built in parallel, app and DMG notarized and stapled; `latest.json` → `AI-Juggler-0.0.4-universal.app.tar.gz`, `latest-nightly.json` = the `nightly-channel` feed → `AI-Juggler-Nightly-0.0.4-…`. Downloaded DMGs re-checked on the maintainer's Mac: SHA-256 match `SHA256SUMS.txt`, `stapler validate` OK, `spctl --type open`: accepted, Notarized Developer ID, team TH8VS427L5 | PASS |

Gallery states `58-updates/nightly-edition` and `58-updates/dev-edition`
show the Settings card and the rail's edition badge.

### First signed nightly (2026-10-19)

Four runs to the first good one, each failure fixed at its cause:

1. Run 39258103322: Tauri's `pnpm build` ran in `crates/juggler-app`, where the CLI finds no UI project → the build runs from the repository root (#44).
2. Run 39264342560 (0.0.2): `stamp` left the crates' internal dependency requirements at 0.0.1 → it rewrites them too (#46).
3. Run 39270091506 (0.0.3): app built, signed, **notarized (Accepted, 43 min)**; the DMG itself wasn't notarized → DMG notarized and stapled, editions built in parallel (#47).
4. Run 39288351167 (0.0.4): **success**, 25 minutes end to end.

## Finishing the beta update path (2026-10-20)

What a beta user needed that wasn't there yet: failures said in plain
words, no restart offered mid-switch or mid-bring-in, the Beta label on
every prompt, proof at the socket and with the real updater plugin, and a
way back that works. Walked by hand in the browser preview (vite, mock
bridge) and rehearsed on AI Juggler Dev.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Check on launch; off = zero network calls | Req.1 / perf | `updates_plugin :: check_off_means_zero_connections_and_on_reaches_the_feed`: the **real updater plugin** against a local feed counting connections at the socket — off: **0**; on, the same call: 1, version and notes reported, nothing downloaded. Removing the setting's gate makes it fail (left 1, right 0) | PASS | 2026-10-20 | claude-opus |
| Version and that version's notes before installing; never silent | Req.1 | The notes are `docs/releases/X.Y.Z.md`, put into the feed by `promote.yml`. Card and Home notice show version, notes and the Beta chip before Install (vitest `updates.test.tsx`, gallery `58-updates/available`, `home-available`). Review opens Settings on the update, focused | PASS | 2026-10-20 | claude-opus |
| Beta labelled in About and every update prompt | Req.3 | One chip component: About, the available box, the Home notice and the restart prompt (vitest; Playwright asserts the chip on both Home prompts). The notice no longer says "(beta)" on any channel | PASS | 2026-10-20 | claude-opus |
| Failed update leaves the running version untouched, says why in plain words, with Try again | Req.4 | `updates_plugin :: a_package_that_fails_its_signature_never_installs`: packages signed in the test (minisign format) — changed after signing, signed with another key, signed for another version (downgrade), garbage signature, a download that breaks off — **none installs**; a throwaway `Fixture.app` stays byte for byte; each says why. Control: correctly signed, it installs and replaces the bundle. `updates :: every_failure_is_said_in_plain_words…`: one sentence per kind, the running version named, the technical reason kept for Details | PASS | 2026-10-20 | claude-opus |
| Download progress at least every 5 % | perf | Same plugin test: a 4 MB package arriving in pieces, progress steps ≤ 5 %, ending at 100 | PASS | 2026-10-20 | claude-opus |
| No restart prompt mid-switch or during a bring-in; offered, deferred, never forced | UI/UX | `updates :: restart_waits_for_a_bring_in_and_is_offered_once_safe` (a real import holds it; seconds after the last import it is offered; a staged switch holds it again; nothing restarts) and `…restart_for_live_instances`. After an install the app re-reads every 3 s and tells the window when it changes. vitest: no button and the reason while held; the Home prompt only when safe; Later puts it away | PASS | 2026-10-20 | claude-opus |
| Rollback documented and rehearsed; migration fixture or downgrade note | Req.2 | [`rollback-rehearsal.md`](docs/qa/evidence/AJ-E1010/rollback-rehearsal.md): 0.0.7 on 0.0.8's data **quit silently** (a bug, AJ-E806 ledger); the documented way back restores both accounts. Downgrade note in `docs/releases/0.0.8.md` (Known issues). New launch path tested by `newer_data` (3) and seen in the rehearsal (step 5). The alert's buttons weren't clicked by hand | PASS (Dev); signed-build rehearsal pending | 2026-10-20 | claude-opus |
| Gallery 1280×800, both themes; axe 0; no modal trap; hand cursor | visual / a11y | `58-updates`: available, downloading, failed, check-failed, restart-waiting, restart-ready, up-to-date, development, the two editions, home-available, home-restart-ready. a11y: 5 card states + 2 Home prompts × 2 themes, axe 0 serious/critical; the long-notes box is a focusable named region (axe flagged it before); Tab moves past each prompt. `cursor.spec.ts › the update UI`: every control in 4 card states and both prompts shows the hand | PASS | 2026-10-20 | claude-opus |

Test logs: [`update-tests.txt`](docs/qa/evidence/AJ-E1010/update-tests.txt) (11),
[`update-vitest.txt`](docs/qa/evidence/AJ-E1010/update-vitest.txt) (12).

**Walked by hand** (browser preview, mock bridge): idle → Check now →
available (notes, Beta) → Install → signature failure (Details opened,
technical reason) → Try again → installed while a switch runs (no button,
the reason) → Home: Restart to update → refused because a session started
(plain words) → launch notice → Review lands on the update → Install →
downloading → a failed check → Try again → up to date.

**Still the maintainer's:** the four requirements on a real signed beta
build (install a build with this change, then update it from the beta
feed), and clicking "Use the backup from …" once on a Dev build
(rehearsal step 5).

### Going back, made reliable (2026-10-20, coordinator's request)

0.0.8 reached beta with schema 28 and no way back but a backup taken by
the in-app updater. Added to this PR:

| Claim | Proof | Verdict |
|---|---|---|
| Before any migration touches an existing database, a consistent copy is kept (`juggler.sqlite.before-v<N>`, SQLite's backup API, written under a temporary name first); the two newest are kept; a fresh or current database makes none; if the copy can't be written, nothing is migrated | `cargo test -p juggler-core --test pre_migration_copy` (4): the v27 copy read back read-only has exactly the rows of every table the database had, at schema 27; retention keeps 27 and 26 of {19, 20, 26, 27} and clears a half-written copy; no copy for new, current or in-memory databases; a blocked copy fails the open with "nothing was changed" and the database stays at 27 | PASS |
| The newer build offers "Go back to an earlier version": says what is lost (changes since the update) and kept, asks once inline, puts the copy back as it is, keeps the data as it was (`juggler.sqlite.from-v<N>-<ms>`), quits; refused while a switch, a bring-in or a session would be cut short | `newer_data :: the_newer_build_prepares_going_back_to_the_version_before` (a v27 database upgraded by this build, an account added after, then: the older build reads schema 27 with the old account only; the kept file has both); vitest (hidden without a copy; confirm text; focus into the question and back on Cancel; refusal in plain words); a11y `going-back` both themes (keyboard only, axe 0); cursor; gallery `58-updates/going-back-confirm`; walked by hand in the preview | PASS |
| An older build at launch offers the copy kept before the upgrade (or a backup) | `newer_data :: the_launch_offers_the_copy_kept_before_the_upgrade` | PASS |
| A release whose schema is newer than where users come from can't ship without a downgrade note | `scripts/downgrade_note.py` (self-test in `just release-selftest`), called by `release_notes_check.sh`: 0.0.5–0.0.7 "no note needed"; 0.0.8 "27 → 28, the note is there"; 0.0.8 notes without the note fail. Nightly compares with the previous nightly and `nightly_notes.py` writes the line itself; Promote fetches tags and compares with the version the channel serves | PASS |

### Review of #82 (2026-10-20): fixes

| Finding | Fix | Proof |
|---|---|---|
| Linux clippy: the launch alert's choices never built off macOS; Linux/Windows still quit with stderr only | The alert, the notice and "can't start" use the native dialog on every desktop platform (`rfd`, already in the tree via tauri-plugin-dialog, same features: gtk3 / common controls). Plugin test announces every platform; its install control is macOS-only (the package is an app bundle), elsewhere a correctly signed download verifies | clippy both configs (macOS); cfg reviewed for Linux |
| A failed go-back from a backup left older `heuristics.json`/`parked/` beside the newer database | `go_back` moves them aside with the newer data first and puts every path back on failure (and clears the staging folder) | `newer_data :: a_restore_that_fails_half_way_puts_everything_back` (fails without the fix: the older settings stay) |
| The launch text promised everything back; the restore's next steps were dropped | The text says a backup may need sign-ins and withheld values again; after going back from a backup, a notice lists what is left to do | `an_older_build_refuses_newer_data…` asserts both |
| Launch handling ran before the single-instance lock | A data-folder lock (`juggler.lock`) is taken before the data is opened and held until quit; going back at launch touches nothing while another instance holds it | `going_back_needs_the_data_folder_to_itself` |
| Prepare: race between the busy check and the lock; lock released before exit | The check, the swap, a WAL checkpoint and the exit all happen under one storage lock (`finish` runs inside it; the command exits there) | `the_newer_build_prepares_going_back…` asserts the lock is held in `finish` |
| `downgrade_note.py` passed when it couldn't read a schema; comma-split count | Fails closed (`AIJ_DOWNGRADE_NOTE_UNCHECKED=<reason>` to override, said in the output); counts `V<n>` entries (comments stripped, V1..Vn in order or it fails); a shallow checkout without tags fails | self-test (in `just release-selftest`) |
| Minor | The copy is flushed to disk with its folder before migrating; a failed copy removes its temporary file and says "free some space" when the disk is full; copies and the database are owner-only (0600) | `pre_migration_copy :: the_copy_has_what_is_still_only_in_the_wal` (also asserts 0600) |

Not done by hand: clicking the launch alert's buttons. This session can't
drive native windows (System Events needs a permission only the
maintainer can grant). Steps for the maintainer are in the PR.

### A second launch hands over before it opens the data (2026-10-20)

A second launch of the same edition used to open the data — migrations,
startup recovery — before the single-instance plugin handed it to the
running app. Now, when another instance holds `juggler.lock`, the launch
only starts a bare app (no state, no windows) whose single-instance plugin
hands over and exits; if the running app doesn't answer (it is closing),
it says so instead of opening the data.

| Claim | Proof | Verdict |
|---|---|---|
| A second launch doesn't open the data | [`second-instance.txt`](docs/qa/evidence/AJ-E1010/second-instance.txt), two Dev builds with a throwaway HOME: with the build before, the second launch reset the database's mode (it opened storage) and handed over after 3.1 s; with this change the mode is untouched and it hands over in 1.0 s; both exit 0 (the running app got the hand-over) | PASS |
| The lock itself | `newer_data :: going_back_needs_the_data_folder_to_itself` | PASS |
| Automated: the real binary as the second launch, against a throwaway home whose data folder the test holds | `cargo test -p juggler-app --test second_launch` (Unix): it ends, the folder holds only `juggler.lock` (stopped at once if anything else appears), and it says what to do. Without the gate it fails in 1.3 s ("opened the data folder") | PASS |
| Review race: a second launch that finds no one to hand over to (the other is still starting, or on its launch message) must not stay the single instance while its message is up, or the other would hand itself over to it and exit | The bare app releases the single-instance claim before it reports; on macOS it first waits (≤ 1 s) for the plugin's socket, which is bound on a task of its own. `second_launch` asserts no socket is left; without the wait it fails ("let go of the single-instance socket") | PASS |
| The message when the other doesn't answer says how to get out: quit it, or force-quit it (⌥⌘Esc on macOS; Task Manager on Windows) | `second_launch` checks the words (macOS) | PASS |
