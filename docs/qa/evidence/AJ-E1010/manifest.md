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
