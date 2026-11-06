# Releasing AI Juggler

How AI Juggler is built, signed, released on its three channels, and if
needed rolled back (AJ-E1010, decisions D-20 and D-23). Every change
travels **nightly → beta → stable**:

| Channel | Who it's for | How a build gets there |
|---|---|---|
| **Nightly** | People who want the newest code and accept rough edges. Installed as its own app, **AI Juggler Nightly** | `nightly.yml` builds `main` every day something that ships has changed. No approval |
| **Beta** | Newer features, a little less tested | `promote.yml`, once the build has had **3+ days of real use** as AI Juggler Nightly and **no `blocker` issue is open** (D-24), with the maintainer's approval (the `beta` environment) |
| **Stable** | Well-tested releases for everyday work | `promote.yml`, a deliberate release: only a version that has been on beta **14+ days** with **no open `blocker`**, once the beta gate is signed, with the maintainer's approval (the `stable` environment) |

**What counts as a blocker** (label the issue `blocker`): data loss, an
Instance running on the wrong account, stopping a process AI Juggler
doesn't own, a crash at launch, or a broken update or rollback. An open
blocker stops every promotion until it's fixed or relabelled. A rule can
be overridden only with a written reason (Promote's `override` input),
which is kept in the channel tag.

macOS is the only OS shipped today. Windows and Linux each get one
dedicated first release (D-52); from then on they are built, signed and
published by this same flow, with the same version and channels: every
nightly carries every shipped OS, and one promote moves them all. Their
packaging and signing sections will be added here with their first
releases (AJ-E2614, AJ-E2713).

There is **one version stream**. A build is made once, by the nightly
workflow, with the next version as Semantic Versioning has it: `MAJOR.MINOR`
from `Cargo.toml` (raised by hand), and a patch that starts at 0 for each new
`MAJOR.MINOR` and goes up by one per build (0.1.0, 0.1.1, …), counted from the
release tags by `scripts/release_version.py`. Promotion points a channel's feed at that same
signed build, so nothing is rebuilt and versions only go up. Three
workflows do the mechanical work:

- **`nightly.yml`** plans the build and calls **`build-macos.yml`**, which
  builds, signs, notarizes and publishes the version as a pre-release, as
  both apps (below); then it moves the nightly feed.
- **`promote.yml`** moves the beta or stable feed to an existing version
  after its checks and the maintainer's approval.

### Where the keys are (AJ-E2001)

No build or dependency code ever runs where the signing keys are.
`build-macos.yml` has four jobs, and `nightly.yml` publishes:

| Job | Holds | Runs |
|---|---|---|
| **preflight** | the `release` environment's secrets | a check that every one is set (nothing is built otherwise); no repository code |
| **compile**, per edition and architecture | no secret; a read-only token | the compile, aarch64 and x86_64 on their own runners at the same time: every crate's build script and `pnpm build` |
| **build**, per edition | no secret; a read-only token | joins the two architectures (lipo) and bundles the app with the compile's configuration; compiles nothing. Uploads the unsigned `.app` |
| **sign**, per edition | the `release` environment's secrets; a read-only token | `scripts/sign_macos.sh` only: Apple's `codesign`, `notarytool`, `stapler`, `hdiutil`, and the pinned prebuilt tauri-cli's `signer sign`. Signs, notarizes and staples the app and its disk image, makes and signs the updater archive, and checks all of it |
| **publish** (`nightly.yml`) | the only token that may write (with the nightly feed job) | the feeds and checksums, then the pre-release; no build code |

The `release` environment only allows `main`, so a branch can never reach
the keys. Tools in the release jobs are pinned to an exact version and
checksum:
- tauri-cli: Tauri's prebuilt binary, checked against its SHA-256 by
  `scripts/install_tauri_cli.sh`;
- pnpm: the exact version and sha512 in `build-macos.yml`, which corepack
  checks.

Before anything is published, the sign job checks:
- the app's signature, hardened runtime and lack of entitlements;
- Gatekeeper, for the app and the disk image;
- both stapled tickets;
- the updater signature, against the public key baked into the app, and
  that it's signed for its own version (`version:` in the trusted comment,
  from `signer sign --app-version`), which the app's updater requires
  (`scripts/updater_sig_check.py`). 0.1.0 was published without it, and
  0.0.x couldn't update to it.

**A dry run** (`release-rehearsal.yml`) builds both editions for real and
rehearses the sign job with an ad-hoc signature and a throwaway updater
key. No secret is passed or read, and nothing is notarized or published.
It runs on any pull request that changes the release workflows or their
scripts, and by hand on any branch (Actions › Release rehearsal › Run
workflow).

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
  coming**, at the same time. Each edition keeps its own list of accounts:
  it can use a setup where it is (ADR-0018) or make its own copy inside its
  own data folder, and the two editions never share a copy. A copy needs its
  own sign-in, and every edition draws on the same subscription's usage.
  The shortcut that opens AI Juggler from anywhere has no default (⌥Space
  is refused because ChatGPT uses it): give each edition its own in
  Settings › Desktop apps.
- **Developers and agents run only AI Juggler Dev**, which can never open an
  installed edition's data.
- **Promotion is unchanged**: beta and stable point at the AI Juggler build
  of a version (`latest.json`); the nightly feed points at its AI Juggler
  Nightly build (`latest-nightly.json`). Both workflows check they carry the
  right app.

## How users get updates

- **Release builds** carry the updater's public key. They check their
  channel's feed (`releases/download/<channel>-channel/latest.json`) when
  they start, a setting that is on by default and can be turned off. A check
  only reports: the user sees the new version and its notes, and nothing is
  downloaded or installed until they choose **Install**.
- **The channel** is chosen in AI Juggler's Settings › About & updates:
  Stable or Beta. Until the user chooses, an install follows the channel
  it was published to. Promotion doesn't rebuild, so a fresh install
  learns this on its first update check: when the stable feed carries
  this exact version (it was downloaded as a stable release), it follows
  Stable; otherwise (a beta release, or no stable feed yet) Beta. If that
  check can't read the stable feed, it follows Beta until a later check
  can. An install that existed before this rule keeps Beta. The user's
  own choice always wins.
  Nightly builds are the separate AI Juggler Nightly app, which always
  follows the nightly feed. Moving to a
  steadier channel never goes back to an older version: that channel is
  offered once it passes the installed one.
- **Installing** first writes a backup of AI Juggler's data (`backups/` in the
  data folder). It then downloads the package, checks it against the release
  signature, and installs it. If any step fails, the running version is left
  as it was, the user reads why in plain words (the network, a signature that
  doesn't check out, a package signed for another version, the backup), the
  technical reason is behind **Details**, and **Try again** repeats the step.
  An install waits while a switch is in flight.
- **Restarting** into the new version is offered, never forced: **Restart to
  update** (Settings and a prompt on Home) appears only while no session is
  running, no switch is in flight and no bring-in is under way; until then
  Settings says what it is waiting for. Quitting normally also starts the new
  version next time.
- **The prompts are labelled** with the channel: **Beta** on the beta channel
  (D-04/D-11), in About and on every update prompt.
- **Development builds** (anything built without the key, and AI Juggler
  Dev) never check.

## One-time setup (maintainer)

Settings › Environments, then Settings › Secrets and variables › Actions.
Repository settings and secrets are yours to change; no workflow or agent
changes them.

**1. Create the `release` environment** (Settings › Environments › New
environment, name `release`):

- **Deployment branches and tags:** Selected branches and tags → add the
  rule `main`. Only `main` may use the keys; a run from any other branch
  waits and then fails at its preflight.
- **Required reviewers:** none. Nightly runs unattended; beta and stable
  already wait for you in Promote.
- **Allow administrators to bypass:** off, if your plan offers it.

**2. Move the signing secrets into it.** Add each one under the
environment's **Environment secrets**, then delete it from **Repository
secrets**. Until you delete them there, nightly keeps working from the
repository copies, and only the preflight and sign jobs read them.

| Name | Kind | What it is |
|---|---|---|
| `APPLE_CERTIFICATE` | environment secret | The Developer ID Application certificate, exported as `.p12`, base64-encoded |
| `APPLE_CERTIFICATE_PASSWORD` | environment secret | The password of that `.p12` |
| `APPLE_SIGNING_IDENTITY` | environment secret | For example `Developer ID Application: Name (TEAMID)` |
| `APPLE_TEAM_ID` | environment secret | The team id (10 characters) |
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | environment secret | The updater signing key, from `cargo tauri signer generate` |
| `APPLE_API_KEY`, `APPLE_API_KEY_ID`, `APPLE_API_ISSUER` | environment secret | Notarization with an App Store Connect API key (step 3; preferred) |
| `APPLE_ID`, `APPLE_PASSWORD` | environment secret | Notarization with an Apple ID and an app-specific password, if there is no API key |
| `AIJ_UPDATER_PUBKEY` | **repository variable** (stays) | The updater's public key (public; baked into the app) |

**3. Optional, recommended: an App Store Connect API key for
notarization**, instead of the Apple ID and app-specific password:

1. App Store Connect › Users and Access › Integrations › App Store
   Connect API › Team Keys › Generate API Key. Name it
   `AI Juggler notarization`, access **Developer**.
2. Download the `.p8` (it can be downloaded once). Note the **Key ID** and
   the **Issuer ID** shown above the keys list.
3. In the `release` environment, set `APPLE_API_KEY` to the `.p8` file's
   whole text (`-----BEGIN PRIVATE KEY-----` … `-----END PRIVATE KEY-----`),
   `APPLE_API_KEY_ID` to the Key ID and `APPLE_API_ISSUER` to the Issuer ID.
4. Once a nightly has notarized with it (its preflight summary says
   "App Store Connect API key"), delete `APPLE_ID` and `APPLE_PASSWORD`
   and revoke the app-specific password at account.apple.com.

With all three `APPLE_API_*` set, the key is used; with none, the Apple ID.
A partial set stops the preflight.

**4. Check it.** Run **Release rehearsal** on any branch: it must pass,
and touch none of these secrets. Then run **Nightly** by hand with
`force`. Its preflight and sign jobs show **release** as their
environment, and the build jobs show none.

Keep the updater private key outside the repository, with a backup. If it is
lost, installed apps can never verify another update and must be reinstalled
by hand. `build-macos.yml` stops before building if any of these is missing.
If a signing secret may have leaked, rotate it at once: a new `.p12`, a new
API key or app-specific password, or (last resort, see above) a new updater
key.

Windows releases: the updater plugin is built without its `zip` feature
(Windows installer extraction), because the `zip` version it uses has open
advisories. Turn it back on in `crates/juggler-app/Cargo.toml` when Windows
releases start and the plugin ships a fixed `zip`.

The app icon is generated at build time from
`crates/juggler-app/icons/icon.png`. Replace it with a 1024×1024 brand
icon before the first public release.

### Review for sensitive paths (AJ-E2004)

`.github/CODEOWNERS` names the paths a release, a permission, a key or the
network hang on: `.github/`, the capability and permission files,
`tauri.conf*.json`, every `build.rs`, `deny.toml`, `juggler-link`,
`juggler-account` and `SECURITY.md`. GitHub asks the maintainer to review
any pull request touching them.

The setting that makes the review required is the maintainer's, once:
Settings › Branches › the rule for `main` (and `release/*`) › **Require a
pull request before merging** › **Require review from Code Owners**, with
one approval. What it changes: a pull request touching those paths waits
for the maintainer's approval before it can merge, a release pull request
included, since most touch `.github/`. Everything else merges after review
as before. Until it's set, CODEOWNERS only requests the review.

## From a change to a release (ADR-0025)

GitHub runs every gate; this Mac, shared with other projects, runs a fast,
low-load check before a push. A release goes like this:

1. **Work on `release/X.Y`**, one branch per release, committed to directly.
2. **Before each push: `just preflight`** (once: `just hooks`). It checks a
   snapshot of `HEAD`, so you can keep editing while it runs, at low
   priority (`nice`, 4 build jobs, 4 test threads), one at a time. In order:
   - the release notes the branch changed, as the nightly checks them;
   - the static checks (`just verify-static`: lints, fmt, clippy ×3,
     rustdoc, cargo-deny). A failure stops it here, within minutes;
   - the tests of the packages the change can affect
     (`scripts/affected_crates.py`; all of them when `Cargo.toml`,
     `Cargo.lock`, the toolchain or the nextest settings changed);
   - vitest and the UI build.

   It writes each step's log and `summary.txt` under
   `target/preflight/<commit>/` and stamps the commit; the pre-push hook lets
   a stamped commit through to `main` and `release/*`. The visual, a11y and
   Linux-clippy containers don't run here. `just preflight-full` is the old
   whole-of-CI run, for when GitHub can't be used.
3. **Push, and open or update the release PR to `main`.** CI runs every gate
   on GitHub's standard runners (free for this public repository; larger
   runners are billed and never used):

   | Check | What runs |
   |---|---|
   | `plan` | which UI gates apply; on a push to main, whether its tree was already tested |
   | `static` | release notes, the project lints, fmt, clippy ×3, rustdoc, cargo-deny |
   | `tests (build)`, `tests (1/6)`…`(6/6)` | the test binaries built once (nextest archive); every test in six partitions, no retries |
   | `doctests, dev-bridge`, `coverage` | the doctests, the read-only test bridge's tests and its never-ships check; the coverage artifact (never gating) |
   | **`verify`** | passes only when all of the above passed |
   | **`ui`** | vitest and the UI build |
   | `visual (1/4)`…`(4/4)`, **`visual`** | the visual gate in the pinned x64 container, split by test; `visual` checks the baseline notes and merges the reports |
   | `a11y (1/6)`…`(6/6)`, `a11y` | the accessibility gate, split by test |
   | **`build-macos`**, **`build-linux`**, **`build-windows`** | the cross-platform builds |

   The checks in bold are the ones main's ruleset requires; they kept their
   names, so the ruleset didn't change. A merge needs every one green on the
   PR's exact head commit.
4. **Merge.** Once main's ruleset allows merge commits, a release PR is
   merged as a merge commit, pinned to the commit CI passed:

   ```sh
   gh pr merge <PR> --merge --match-head-commit <the head SHA CI passed>
   ```

   `main` then contains the release branch's own history, so there is no
   back-merge of `main` into `release/X.Y` after each release (and no
   preflight of a back-merge commit, no double-counted baseline notes).
   Until the ruleset allows it, release PRs are squash-merged as before and
   `main` is merged back into `release/X.Y` before its next change.
5. **The push to `main`** runs nothing again when its tree is exactly the
   tree the PR's head passed every required check with (a merge commit of an
   up-to-date PR, or its squash): `plan` says so and the required checks
   pass by reference. Any other tree runs everything, and so does a weekly
   run on `main`.
6. **Nightly, then Promote**, as below. The nightly compiles each
   architecture on its own runner at the same time; a beta promotion reuses
   CI's accessibility scan of the same tree.

Measured (2026-11-02):

| Step | Before | After |
|---|---|---|
| Local preflight | about 55 min, every gate, serial | 3–5 min when a static check fails or no package's tests can change; 15–21 min when every package's tests run (a change to the build as a whole), at low priority beside other projects |
| A PR's CI round | 32–38 min (`verify` alone) | about 15 min end to end (the test build about 7.5, then the longest partition); the visual and a11y gates are done in 5 |
| Merge → beta | about 45 min (nightly 32, promote 12) | nightly and promote measured on their next runs |

## Each release

**Nightly** needs nothing from you: every day (03:00 UTC) that something
that ships has changed on `main` since the newest `vX.Y.Z` tag,
`nightly.yml` builds the next version. Its notes are generated from the
commit subjects (`scripts/nightly_notes.py`; only `feat`, `perf` and `fix`
commits, with task IDs and PR numbers removed). To build now, run
**Nightly** by hand (`workflow_dispatch`): it builds whatever changed, and
only the very commit of the last nightly needs `force`.

**What ships**, and so what makes a nightly, is one ordered list:
`RULES` in `scripts/release_ships.py`. A day whose changes are all outside
it (docs, release notes, tests, CI, gates) builds nothing, and the run's
summary says "No nightly today" and why. Twice before this a build identical
to the one before took the number the planned release needed (v0.1.5 changed
only a test, v0.1.7 only release notes), and that release had to be renamed.

| Ships (a change builds) | Why |
|---|---|
| `crates/**`, except the next row | The app: Rust code, build scripts, the Tauri configuration, capabilities, icons, `Info.plist`, entitlements |
| `ui/**`, except the next row | What `pnpm build` bundles, with its packages and lockfile |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml` | The version, the exact dependencies, the compiler |
| `.github/workflows/build-macos.yml`, `nightly.yml`, and the scripts they run (`release_version.py`, `release_ships.py`, `sign_macos.sh`, `install_tauri_cli.sh`, `edition_icon.swift`, `apple_retry.py`, `updater_sig_check.py`, `release_seams_check.sh`, `nightly_notes.py`, `downgrade_note.py`, `release_notes_check.sh`) | How it's built, signed and published, and the notes its feed carries |

| Doesn't ship | Why |
|---|---|
| `crates/*/tests/**`, `examples/**`, `benches/**`, in-source `tests.rs` | Only `cargo test`, `run --example` and `bench` build them |
| `ui/tests/**` (Playwright, visual baselines), `ui/src/**/*.test.ts(x)`, `ui/src/test/**`, the Playwright and Vitest configs | Tests |
| `**/*.md`, `docs/**` (including `docs/releases/`) | Documentation. A version's hand-written notes are picked up by its build |
| `.github/**` (other workflows, templates, Dependabot), the other `scripts/**`, `justfile`, `deny.toml`, `rustfmt.toml`, `fuzz/**`, `stats/**` | CI, gates and tools; the app carries `juggler-stats`' generated copy of the schema |

A file no rule names ships, so an unclassified file builds rather than
being skipped, and `just release-selftest` (in CI) fails until every
tracked file is classified. It also checks the list against the tree: no
shipped Rust file `include_str!`s, and no shipped UI file imports, a file
that doesn't ship; every script the two build workflows run ships; and every
in-source `tests.rs` is declared `#[cfg(test)]`. To see why a file counts:
`python3 scripts/release_ships.py explain <path>`.

Signing talks to Apple's timestamp, notary and ticket services, which can
fail for a few minutes. `scripts/apple_retry.py` wraps those steps: it
retries only Apple's known transient failures (up to four tries, 30 s,
60 s, 120 s apart) and fails at once on anything else. If a run still
fails on one of them, re-run its failed jobs from the run page.

**Promoting a version to beta:**

1. **Pick the build.** Any `vX.Y.Z` pre-release on GitHub that nightly
   testing has been happy with.
2. **Usage-statistics schema.** Run
   `FH_SITE=<website clone> scripts/stats_schema_diff.sh` at that commit. It
   must report identical, because the server rejects anything its copy
   doesn't list.
3. **Tool runtimes (D-40, ADR-0022).** Two checks, each a line in the
   release PR:
   - **Codex's protocol.** Generate the installed Codex's app-server
     schema offline and record its surface (the commands are at the top
     of `scripts/codex_app_server_surface.py`). A new Codex version gets
     its own `surface-<version>.json` fixture; the adapter's tests then
     say whether every method AI Juggler uses is still there. A version
     that lost one starts in the terminal by itself, with the reason.
   - **Anthropic's terms for "Run without a terminal" (Beta).** Re-read
     the pages ADR-0022 decision 4 names (Claude Code's legal and
     compliance page, the Agent SDK overview, the headless page and the
     Help Center article on `claude -p` and subscription limits) and
     confirm they still allow a user's own installed `claude -p` on their
     own subscription. If they no longer do, the Beta is switched off in
     that release (its gate fails), and the notes say so.
4. **Notes for users.** Write `docs/releases/X.Y.Z.md` on `main` (they
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
5. **The update on a real setup.** Updates are where real data and real
   signing meet, and two updates in a row failed there and nowhere else
   (0.1.0: a signature without its version; 0.1.3: a project name read as
   a key by the backup before the update). On the Mac with the real setup,
   copy AI Juggler's database (it only reads the original) and run the
   update's own backup step on the copy:
   ```sh
   C=$(mktemp -d) && sqlite3 "file:$HOME/Library/Application Support/AI Juggler/juggler.sqlite?mode=ro" ".backup '$C/juggler.sqlite'"
   cargo test -p juggler-app --test qa_real_readonly --no-run
   AIJ_QA_APP_DATA_COPY=$C <the test binary> --ignored --nocapture update_preflight
   rm -rf "$C"
   ```
   It must pass. The signature check (`scripts/updater_sig_check.py`, in the
   sign job) covers the other half.
6. **Run Promote** (`workflow_dispatch`): version `X.Y.Z`, channel `beta`.
   It checks the build, the notes and the D-24 requirements
   (`scripts/promotion_rules.py`: 3+ days since the build was published, no
   open `blocker`), and the accessibility gate (`just a11y-gate`, AJ-E804)
   on the exact commit: a full scan CI already passed on that tree (the
   commit, or the head of the pull request merged as it, with an identical
   tree; every shard passed) counts, otherwise it runs there (ADR-0025).
   Then it waits for your approval in the run. Approving points the beta feed at the build, puts the notes on
   the release and in its feed, and tags it `beta/vX.Y.Z`.
7. **Check it.** Install the `.dmg` on a clean Mac and open it: Gatekeeper
   must not complain. Settings › About & updates shows the version.

**Promoting a version to stable** works the same way, with channel
`stable`, and always runs the accessibility gate again, whatever CI ran. It
also needs:

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

## Checking where a build came from (AJ-E2007)

Every nightly publishes, beside the DMG and the updater archive:

- **Build provenance.** A signed SLSA provenance attestation for the DMG,
  the `.app.tar.gz` and each SBOM. It names the workflow, the commit and
  the run that built the file. Anyone can check a download:

  ```bash
  gh attestation verify AI-Juggler-0.4.1-universal.dmg --repo figurehead-ai/AI-juggler
  ```

  A file that was changed after the build, or built anywhere else, fails.
- **SBOMs (CycloneDX 1.5).** `AI-Juggler-<version>-juggler-app.cdx.json`
  and `…-juggler-agentbridge.cdx.json` for the Rust side, as that build
  resolved them, and `…-ui.cdx.json` for the shipped JavaScript
  (`scripts/js_sbom.py`, from pnpm's own list).
- **The dependency list inside the executables.** The app and the agent
  bridge are built with `cargo auditable`, so `cargo audit bin
  "AI Juggler.app/Contents/MacOS/juggler-app"` checks a shipped copy
  against the advisory database with nothing else to hand.

What goes in is checked before it's built:

- `cargo deny check`, with `[bans.build]`: no dependency may ship an
  executable or a script without being named in `deny.toml` with why.
- `cargo vet --locked`: every crate is audited (ours or imported from
  Mozilla, Google, the Bytecode Alliance, ISRG, Embark and Zcash) or
  exempted. `scripts/vet_exemptions.py` keeps the exemption list
  shrinking; a new dependency needs an audit (`cargo vet certify`) or an
  imported one.
- pnpm takes a version only after a day (`minimumReleaseAge`), refuses
  one whose trust evidence weakened (`trustPolicy: no-downgrade`) and any
  git or tarball dependency deep in the tree (`blockExoticSubdeps`), at
  the pinned `packageManager` version. `pnpm audit --prod` runs on every
  pull request.
- Dependabot proposes a version only after it's a week old (a major,
  two).
- `supply-chain.yml` runs zizmor on the workflows and the OpenSSF
  Scorecard on pull requests that touch them and weekly. Every checkout
  sets `persist-credentials: false`, and every workflow starts from
  read-only or empty permissions.

## If a key leaks (AJ-E2009)

Act first, explain after. Each step names who does it. "Maintainer" means
a person with the Apple account and the repository's admin rights.

### The updater key (Tauri's minisign key)

Every install trusts two public keys: the current one
(`AIJ_UPDATER_PUBKEY`) and a standby one (`AIJ_UPDATER_NEXT_PUBKEYS`),
both repository variables baked in at build time. The standby key's
private half never touches CI: it stays offline in the maintainer's
password manager. A package that fails the current key is checked
against the standby key (`updates::trusted_keys`;
`tests/updates_plugin.rs` proves it installs and that an unknown key
still never does). So a rotation needs no new build first.

**One-time setup (maintainer).** Run `cargo tauri signer generate -w
standby.key` on a trusted Mac. Store `standby.key` and its password
offline. Set the repository variable `AIJ_UPDATER_NEXT_PUBKEYS` to the
printed public key. Every build from then on trusts it.

**When the current private key, or its password, may have leaked:**

1. **Stop publishing with it (maintainer, minutes).** Delete the
   `TAURI_SIGNING_PRIVATE_KEY` and `…_PASSWORD` secrets from the `release`
   environment. Pause the nightly schedule (Actions › nightly › Disable).
2. **Pull anything it signed since the leak (maintainer).** Point each
   channel back at the last release you know is good (`promote.yml` with
   that version). Delete any release that may be tampered with.
3. **Promote the standby key (maintainer).** Put the standby key's
   private half and password in the `release` secrets. Set
   `AIJ_UPDATER_PUBKEY` to its public key. Generate a new standby key as
   above and set `AIJ_UPDATER_NEXT_PUBKEYS` to it. The leaked key is now in
   no build's list.
4. **Ship a release signed with the new key.** Every install accepts it,
   because the new key was their standby. From that release on, installs
   no longer trust the leaked key.
5. **Tell people** (below). Until users update, an install still trusts
   the leaked key. Abusing it also needs the release feed (this
   repository's releases over HTTPS), so check the repository's audit log
   for who could write releases.

A planned rotation (every two years, or when someone with access leaves)
is the same without steps 1–2.

### The Developer ID certificate or the notarization credentials

1. **Revoke (maintainer).** Revoke the certificate at
   developer.apple.com › Certificates (Apple can then revoke the
   notarization of what it signed). For the App Store Connect API key:
   revoke it at appstoreconnect.apple.com › Users and Access ›
   Integrations. For an app-specific password: revoke it at
   account.apple.com.
2. **Replace.** Create a new Developer ID Application certificate (and API
   key) and update `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
   `APPLE_SIGNING_IDENTITY`, `APPLE_API_KEY*` in the `release` environment.
3. **Re-sign the current release.** Run the nightly by hand. Promote the
   new build to each channel that served one signed with the old
   certificate.
4. Ask Apple Product Security (product-security@apple.com) to block what
   the leaked certificate signed after the leak, and tell people.

### When each secret is rotated

| Secret | Where | Rotated | Next due |
|---|---|---|---|
| Updater key (minisign) | `release` secrets; standby offline | every 2 years, or on any doubt | 2028-10 |
| Developer ID Application certificate | `release` secrets | at expiry (5 years), or on any doubt | at its expiry |
| App Store Connect API key | `release` secrets | yearly | 2027-10 |
| Apple ID app-specific password (if used instead) | `release` secrets | yearly | 2027-10 |
| Repository admins and the `release` environment's reviewers | GitHub settings | reviewed each quarter | 2027-01 |

The maintainer checks this table on each first release of a quarter and
updates "Next due".

### Telling people

1. **Within 24 hours:** a draft GitHub security advisory (Security ›
   Advisories) with what is known, which versions are affected, and what
   users should do. It gets a CVE when one applies. Publish it once
   there's a fixed release, or sooner if users must act now.
2. **The fixed release:** its notes say what happened and what to do,
   and they are what the in-app update prompt shows. Promote it to every
   channel.
3. **The website and the README:** a short notice linking the advisory,
   kept until most installs have updated.
4. **Whoever reported it:** thanks and credit, under SECURITY.md's
   disclosure terms.

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
2. **Fix forward** with a new version. Version numbers only go up, and the
   updater refuses a downgrade (`requireSignedVersion`).
3. **For users who already installed it**, the release notes of the bad
   version (Known issues, edited on the release) and of the fix say how to go
   back, as below.

### How a user goes back to the previous version

Two things make it possible:

- **A copy before every database upgrade** (from the release after 0.0.8):
  before AI Juggler upgrades its database it keeps a copy beside it,
  `juggler.sqlite.before-v<N>` (N: the schema it had), the two newest only.
  If the copy can't be written, nothing is upgraded.
- **A backup before every update** installed from inside the app:
  `backups/before-update-<from>-to-<to>-<time>.ajbackup` in the data folder
  (`~/Library/Application Support/AI Juggler`; the Nightly app uses `AI
  Juggler Nightly`).

Then:

- **If the previous version opens the data** (no database change between
  the two): install its `.dmg` from GitHub Releases over the newer one.
- **If it doesn't** (the release notes carry a downgrade note):
  - **In the newer version**: Settings › About & updates › **Go back to an
    earlier version** › **Prepare to go back to \<version\>**. It says what
    is lost (what changed since the update), puts the copy from before the
    upgrade in place, keeps the data as it is now beside it
    (`juggler.sqlite.from-v<N>-<time>`), and quits. Install the earlier
    version and open it. It waits while a switch, a bring-in or a session
    would be cut short.
  - **In the older version**, if it has the newer-data alert (built after
    this change): when it opens on newer data it says "This data is from a
    newer AI Juggler" and offers **Use the data from \<date\>** (the copy) or
    **Use the backup from \<date\>**. The newer data is moved aside
    (`juggler.sqlite.from-newer-<time>`, never deleted) and the app opens on
    the copy or the restored backup.
  - **Going back to 0.0.7** (neither exists there; 0.0.7 quits at once on
    newer data, without a message):
    1. Quit AI Juggler.
    2. In `~/Library/Application Support/AI Juggler`, rename `juggler.sqlite`
       to `juggler.sqlite.from-0.0.8` (and `juggler.sqlite-wal` /
       `juggler.sqlite-shm` the same way, if they are there).
    3. Install 0.0.7 from its `.dmg` and open it. If the welcome screen
       shows, close it.
    4. If there is a `before-update-0.0.7-to-0.0.8-….ajbackup` in the
       `backups` folder (it was updated from inside the app): Settings ›
       Data: backup & retention › **Restore from a backup…**, pick it (⌘⇧G
       and paste the folder path), then **Restore** and restart. Without
       one, 0.0.7 starts empty: bring the Claude setups in again (they
       were never changed).

   To return to the newer version later, quit, put the renamed
   `juggler.sqlite` back, and install the newer version again.

### Rehearse it

Once per release channel, and whenever a release changes the database:
build or install the previous version and the new one as AI Juggler Dev with
a throwaway `HOME`, let the previous one write data and its pre-update
backup, open the data with the new one, then go back as above. Record it in
`docs/qa/evidence/AJ-E1010/` (the first run:
`rollback-rehearsal.md`). On a signed build, do it with the real feed:
update to the second version, point the channel back at the first, install
the first from its `.dmg`, and go back as above.

## Database changes between versions

Migrations only go forward, and an older build refuses data a newer one
saved (it never risks it). So:

- **Every release whose database schema is newer than the version users
  come from ships a downgrade note** in `docs/releases/X.Y.Z.md`: a bullet
  under `## Known issues` that starts with "Going back to" and says how.
  `scripts/release_notes_check.sh` enforces it (`scripts/downgrade_note.py`
  counts the migrations at the release and at the version users come
  from): nightly builds compare with the previous nightly, and their
  generated notes add the line by themselves; **Promote** compares with the
  version the channel serves now. A release can't skip it.
- Storage keeps a copy of the database before every upgrade
  (`juggler.sqlite.before-v<N>`, the two newest), which is what "Go back to
  an earlier version" puts back.
- When a release changes the schema, add it to the table below and to
  `SCHEMA_RELEASES` in `crates/juggler-app/src/newer_data.rs` (it names the
  version in "Prepare to go back to …" and "Install … from the releases
  page", so it's the newest release on that schema: a patch release on
  the newest schema moves that line to itself).

| Version | Schema | Opens the previous version's data | The previous version opens its data |
|---|---|---|---|
| 0.0.7 | 27 | yes | — |
| 0.0.8 | 28 (accounts used in place) | yes | **no** — downgrade note in 0.0.8's notes |
| 0.0.9 | 29 (session footprints) | yes | **no** — downgrade note in 0.0.9's notes; 0.0.9 keeps a `before-v28` copy, so "Go back to an earlier version" can return to 0.0.8 |
| 0.0.13 | 30 (account rename, remove and default; account audit) | yes | **no** — downgrade note in 0.0.13's notes; 0.0.13 keeps a `before-v29` copy, so "Go back to an earlier version" can return to 0.0.12 |
| 0.1.0 | 32 (31: session and project names, pins, forgotten projects, AJ-E1502; 32: other managers' worktree homes, AJ-E1519) | yes | **no** — downgrade note in 0.1.0's notes; 0.1.0 keeps a `before-v30` copy, so "Go back to an earlier version" can return to 0.0.17 |
| 0.2.x | 37 (33: pools, AJ-E1702; 34: messages between agents, E18; 35: other apps' processes, D-41; 36: tasks shared by pools, E19; 37: the notification model's history and mutes, AJ-E2501) | yes | **no** — 0.2's notes need the downgrade note ("Going back to 0.1.0 or earlier…"); 0.2 keeps a `before-v32` copy, so "Go back to an earlier version" can return to 0.1.0 |
| 0.3.0 | 38 (the pool canvas: where you put each member on a pool's canvas, and an index for a pool's task events) | yes | **no** — 0.3's notes need the downgrade note ("Going back to 0.2.x or earlier…"); 0.3 keeps a `before-v37` copy, so "Go back to an earlier version" can return to 0.2 |
| 0.3.1–0.3.9 | 39 (launch settings removed, not deleted, D-60) | yes | **no** — 0.3.1's notes need the downgrade note; it keeps a `before-v38` copy, so "Go back to an earlier version" can return to 0.3.0 |
| 0.4.x | 46 (40–42: the Account and the App stored, with their colours; 43–44: the relation counts; 45: the pools model, D-62, with 0.3's pools moved over; 46: the Floor's areas) | yes | **no** — 0.4's notes carry the downgrade note ("Going back to 0.3 from 0.4"); 0.4 keeps a `before-v39` copy, so "Go back to an earlier version" can return to 0.3.9 |

Tests: `cargo test -p juggler-core --test pre_migration_copy` (the copy
holds exactly the rows the database had, two are kept, a fresh database
makes none, no copy means no upgrade) and `cargo test -p juggler-app
--test newer_data` (a newer schema is refused and the copy or backup
offered and restored; the newer build prepares going back; a failed
attempt leaves the data as it was).
