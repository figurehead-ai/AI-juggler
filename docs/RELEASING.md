# Releasing AI Juggler

How AI Juggler is built, signed, released on its three channels, and if
needed rolled back (AJ-E1010, decisions D-20 and D-23). Every change
travels **nightly → beta → stable**:

| Channel | Who it's for | How a build gets there |
|---|---|---|
| **Nightly** | People who want the newest code and accept rough edges. Installed as its own app, **AI Juggler Nightly** | `nightly.yml` builds `main` every day there's a new commit. No approval |
| **Beta** | Newer features, a little less tested | `promote.yml`, once the build has had **3+ days of real use** as AI Juggler Nightly and **no `blocker` issue is open** (D-24), with the maintainer's approval (the `beta` environment) |
| **Stable** | Well-tested releases for everyday work | `promote.yml`, a deliberate release: only a version that has been on beta **14+ days** with **no open `blocker`**, once the beta gate is signed, with the maintainer's approval (the `stable` environment) |

**What counts as a blocker** (label the issue `blocker`): data loss, an
Instance running on the wrong account, stopping a process AI Juggler
doesn't own, a crash at launch, or a broken update or rollback. An open
blocker stops every promotion until it's fixed or relabelled. A rule can
be overridden only with a written reason (Promote's `override` input),
which is kept in the channel tag.

There is **one version stream**. A build is made once, by the nightly
workflow, with the next version (`MAJOR.MINOR` from `Cargo.toml`, the patch
is the nightly run number). Promotion points a channel's feed at that same
signed build, so nothing is rebuilt and versions only go up. Three
workflows do the mechanical work:

- **`nightly.yml`** plans the build and calls **`build-macos.yml`**, which
  builds, signs, notarizes and publishes the version as a pre-release, as
  both apps (below); then it moves the nightly feed.

## Editions: side by side on one Mac (D-23)

Every version is built as two apps from the same commit, and developers
build a third locally. They install next to each other and share nothing:

| Edition | Bundle id | Data folder (Application Support) | Updates from | Release assets |
|---|---|---|---|---|
| **AI Juggler** | `ai.figurehead.juggler` | `AI Juggler` | Beta or Stable (the user's choice) | `AI-Juggler-X.Y.Z-universal.*`, `latest.json` |
| **AI Juggler Nightly** | `ai.figurehead.juggler.nightly` | `AI Juggler Nightly` | Nightly | `AI-Juggler-Nightly-X.Y.Z-universal.*`, `latest-nightly.json` |
| **AI Juggler Dev** | `ai.figurehead.juggler.dev` | `AI Juggler Dev` | never | built by `just dev` / `just app` |

Each edition also has its own Keychain items, Fleet port (47821, 47822,
47823) and single-instance lock, and the Nightly and Dev icons carry a band
with their name. The edition comes from the bundle identifier
(`crates/juggler-app/tauri.nightly.conf.json` and `tauri.dev.conf.json`
overlay `tauri.conf.json`); a debug build is always Dev. So:

- **Use AI Juggler for real work and AI Juggler Nightly to try what's
  coming**, at the same time. Each makes its own copy of an Account inside
  its own data folder, so the two never write to the same Claude setup or
  to `~/.claude`; you sign in once inside each copy, and both draw on the
  same subscription's usage. Both want ⌥Space for Go to instance, and only
  the one opened first gets it: give Nightly another shortcut (⌘K, then
  change it there).
- **Developers and agents run only AI Juggler Dev**, which can never open an
  installed edition's data.
- **Promotion is unchanged**: beta and stable point at the AI Juggler build
  of a version (`latest.json`); the nightly feed points at its AI Juggler
  Nightly build (`latest-nightly.json`). Both workflows check they carry the
  right app.
- **`promote.yml`** moves the beta or stable feed to an existing version
  after its checks and the maintainer's approval.

## How users get updates

- **Release builds** carry the updater's public key. They check their
  channel's feed (`releases/download/<channel>-channel/latest.json`) when
  they start, a setting that is on by default and can be turned off. A check
  only reports: the user sees the new version and its notes, and nothing is
  downloaded or installed until they choose **Install**.
- **The channel** is chosen in AI Juggler's Settings › About & updates:
  Stable or Beta. Installs follow Beta until the first stable release.
  Nightly builds are the separate AI Juggler Nightly app, which always
  follows the nightly feed. Moving to a
  steadier channel never goes back to an older version: that channel is
  offered once it passes the installed one.
- **Installing** first writes a backup of AI Juggler's data (`backups/` in the
  data folder). It then downloads the package, checks it against the release
  signature, and installs it. If any step fails, the running version is left
  as it was, with a reason and **Retry**.
- **Restarting** into the new version is offered, never forced. It waits until
  no Instance is running, and an install waits while a switch is in flight.
- **Development builds** (anything built without the key, and AI Juggler
  Dev) never check.

## One-time setup (maintainer)

Repository settings → Secrets and variables → Actions.

| Name | Kind | What it is |
|---|---|---|
| `APPLE_CERTIFICATE` | secret | The Developer ID Application certificate, exported as `.p12`, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | secret | The password of that `.p12` |
| `APPLE_SIGNING_IDENTITY` | secret | For example `Developer ID Application: Name (TEAMID)` |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | secret | The notarization account; `APPLE_PASSWORD` is an app-specific password |
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | secret | The updater signing key, from `cargo tauri signer generate` |
| `AIJ_UPDATER_PUBKEY` | **variable** | The matching public key (it is public, and it is baked into the app) |

Keep the updater private key outside the repository, with a backup. If it is
lost, installed apps can never verify another update and must be reinstalled
by hand. `build-macos.yml` stops before building if any of these is missing.

Windows releases: the updater plugin is built without its `zip` feature
(Windows installer extraction), because the `zip` version it uses has open
advisories. Turn it back on in `crates/juggler-app/Cargo.toml` when Windows
releases start and the plugin ships a fixed `zip`.

The app icon is generated at build time from
`crates/juggler-app/icons/icon.png`. Replace it with a 1024×1024 brand
icon before the first public release.

## Each release

**Nightly** needs nothing from you: every day with new commits on `main`,
`nightly.yml` builds the next version. Its notes are generated from the
commit subjects (`scripts/nightly_notes.py`; only `feat`, `perf` and `fix`
commits, with task IDs and PR numbers removed). To build now, run
**Nightly** by hand (`workflow_dispatch`, optionally with `force`).

**Promoting a version to beta:**

1. **Pick the build.** Any `vX.Y.Z` pre-release on GitHub that nightly
   testing has been happy with.
2. **Usage-statistics schema.** Run
   `FH_SITE=<website clone> scripts/stats_schema_diff.sh` at that commit. It
   must report identical, because the server rejects anything its copy
   doesn't list.
3. **Notes for users.** Write `docs/releases/X.Y.Z.md` on `main` (they
   replace the generated nightly notes). The website shows this text as is,
   and `scripts/release_notes_check.sh X.Y.Z` enforces the rules:
   - The first line is one plain sentence on what the release is about.
   - Then only the sections that apply, in this order: `## New`,
     `## Improved`, `## Fixed`, `## Known issues`.
   - One bullet per change, saying what the user can now do or will notice.
     Bold the feature name when it helps scanning.
   - Name the tool when a change only affects one ("Codex: …"). Label beta
     features as beta, and never call switching lossless.
   - No task IDs, PR numbers, internal names or file paths. No raw HTML:
     screenshots are uploaded to the release and linked with Markdown.
4. **Run Promote** (`workflow_dispatch`): version `X.Y.Z`, channel `beta`.
   It checks the build, the notes and the D-24 requirements
   (`scripts/promotion_rules.py`: 3+ days since the build was published, no
   open `blocker`), runs the accessibility gate on the exact commit
   (`just a11y-gate`, AJ-E804), then waits for your approval in the run. Approving points the beta feed at the build, puts the notes on
   the release and in its feed, and tags it `beta/vX.Y.Z`.
5. **Check it.** Install the `.dmg` on a clean Mac and open it: Gatekeeper
   must not complain. Settings › About & updates shows the version.

**Promoting a version to stable** works the same way, with channel
`stable`, and also needs:

- the version has been on beta for 14+ days (counted from the annotated
  `beta/vX.Y.Z` tag's date) and no `blocker` issue is open;
- the beta gate is signed (`docs/qa/gate-beta.md`, AJ-E807; build it with
  `just gate-packet`, review it, fill in its Sign-off section, check with
  `just gate-signed`);
- a `## [X.Y.Z]` section in `CHANGELOG.md` for contributors.

Approving makes it GitHub's **Latest** release (the website's download
button uses it) and tags it `stable/vX.Y.Z`.

**Usage statistics** (only once they are switched on): register `X.Y.Z` as a
known version in Figurehead's usage-statistics configuration when it reaches
beta or stable. Reports from unregistered versions are left out of the
dashboards.

## Rollback

A release is never deleted to roll it back. Instead:

1. **Stop offering it.** Point the channel back at the previous good
   version:
   - beta or stable: run **Promote** with that version and channel (it was
     approved before, so its checks pass again);
   - nightly: `gh release download vX.Y.Z --pattern latest-nightly.json` for
     the good version, rename it to `latest.json`, then
     `gh release upload nightly-channel latest.json --clobber`.

   Installs that have not updated are then offered nothing new.
2. **For users who already installed it**, the release notes of a follow-up
   version say how to go back:
   - download the previous version's `.dmg` from GitHub Releases and install it;
   - if that version cannot open the newer data (it says so: "made by a newer
     AI Juggler"), restore the backup the update wrote in `backups/`
     (Settings › Backup & restore).
3. **Fix forward** with a new version. Version numbers only go up, and the
   updater refuses a downgrade (`requireSignedVersion`).

**Rehearse this once**, before the first beta reaches users: let nightly
build two versions, update to the second, point the channel back at the
first, install the first from its `.dmg` and restore the pre-update backup.
Record the run in `docs/qa/evidence/AJ-E1010/`.

## Database changes between versions

Migrations only go forward. Every release states either "opens data from
the previous version" (the default: migrations run on start) or carries a
downgrade note, listed under Known issues, when an older build can no longer
open the data. The backup written before each update is the way back.
