# Releasing AI Juggler

How a beta release is built, signed, announced, and if needed rolled back
(AJ-E1010). Two workflows do the mechanical work:

- **`release.yml`** builds a draft.
- **`release-channel.yml`** points the beta update channel at a release once
  it is published.

Publishing is always a person's decision.

## How users get updates

- **Release builds** carry the updater's public key. They check
  `releases/download/beta-channel/latest.json` when they start, a setting that
  is on by default and can be turned off. A check only reports: the user sees
  the new version and its notes, and nothing is downloaded or installed until
  they choose **Install**.
- **Installing** first writes a backup of AI Juggler's data (`backups/` in the
  data folder). It then downloads the package, checks it against the release
  signature, and installs it. If any step fails, the running version is left
  as it was, with a reason and **Retry**.
- **Restarting** into the new version is offered, never forced. It waits until
  no Instance is running, and an install waits while a switch is in flight.
- **Development builds** (anything built without the key) never check.

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
by hand. `release.yml` stops before building if any of these is missing.

The app icon is generated at build time from
`crates/juggler-app/icons/icon.png`. Replace it with a 1024×1024 brand
icon before the first public release.

## Each release

1. **Version.** Set the same `X.Y.Z` in `crates/juggler-app/tauri.conf.json`
   and `crates/juggler-app/Cargo.toml`. Add a `## [X.Y.Z]` section to
   `CHANGELOG.md` for contributors.
2. **Notes for users.** Write `docs/releases/X.Y.Z.md`. The website shows this
   text as is, and `scripts/release_notes_check.sh X.Y.Z` enforces the rules:
   - The first line is one plain sentence on what the release is about.
   - Then only the sections that apply, in this order: `## New`,
     `## Improved`, `## Fixed`, `## Known issues`.
   - One bullet per change, saying what the user can now do or will notice.
     Bold the feature name when it helps scanning.
   - Name the tool when a change only affects one ("Codex: …"). Label beta
     features as beta, and never call switching lossless.
   - No task IDs, PR numbers, internal names or file paths. No raw HTML:
     screenshots are uploaded to the release and linked with Markdown.
3. **Tag.** Push `vX.Y.Z`. `release.yml` checks the secrets, versions and
   notes, then:
   - builds the universal app, signs and notarizes it;
   - verifies the signature, the notarization and the updater signature;
   - names the installer `AI-Juggler-X.Y.Z-universal.dmg`;
   - writes `latest.json`;
   - drafts a **pre-release** with every asset and `SHA256SUMS.txt`.
4. **Review the draft.**
   - Install the `.dmg` on a clean Mac and open it: Gatekeeper must not
     complain.
   - Check Settings › About & updates shows the version and Beta.
   - Back up from Settings and keep the file.
5. **Publish.** Publishing runs `release-channel.yml`, which copies the
   release's `latest.json` to the `beta-channel` feed, and from then on beta
   users are offered it. The website's download button uses the newest
   release that has a `.dmg`.
6. **Usage statistics** (only once they are switched on): the maintainer
   registers `X.Y.Z` as a known version in Figurehead's usage-statistics
   configuration. Reports from unregistered versions are left out of the
   dashboards.

## Rollback

A release is never deleted to roll it back. Instead:

1. **Stop offering it.** Run **Release channel** by hand
   (`workflow_dispatch`) with the previous good tag. Builds that have not
   updated are then offered nothing new.
2. **For users who already installed it**, the release notes of a follow-up
   point release say how to go back:
   - download the previous version's `.dmg` from GitHub Releases and install it;
   - if that version cannot open the newer data (it says so: "made by a newer
     AI Juggler"), restore the backup the update wrote in `backups/`
     (Settings › Backup & restore).
3. **Fix forward** with a new version. Version numbers only go up, and the
   updater refuses a downgrade (`requireSignedVersion`).

**Rehearse this once per channel**, before the first beta reaches users:
1. Publish a test release `0.0.2` and update to it.
2. Move the channel back to `0.0.1`.
3. Install `0.0.1` from its `.dmg` and restore the pre-update backup.

Record the run in `docs/qa/evidence/AJ-E1010/`.

## Database changes between versions

Migrations only go forward. Every release states either "opens data from
the previous version" (the default: migrations run on start) or carries a
downgrade note, listed under Known issues, when an older build can no longer
open the data. The backup written before each update is the way back.
