# Evidence manifest: AJ-E3822

**Task:** `docs/tasks/E38-chat.md`, AJ-E3822: helpers never share an identifier across identities, and every one names itself (the '"(null)" differs from previously opened versions' prompt).

**Branch:** `release/0.5.1`. Built offline on this Mac with the helper scripts (no app launched). Paths are placeholders (`/Users/you`).

## By requirement

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 every helper's identifier follows the build's identity; the run-time side follows it too | PASS | Build side: `scripts/helper_identity.sh` (AIJ_EDITION main or nightly gives `ai.figurehead.juggler.<suffix>`, dev or unset `ai.figurehead.juggler.dev.<suffix>`; a bundled helper's check reads the .app's own identifier). Release workflow: `AIJ_EDITION: ${{ matrix.edition }}` on the compile job. Run time: `Edition::helper_identifier` (edition.rs), read by `engines::apple::identifier()` (the code requirement, so the sandbox container) and `engines::local::identifier()` (the code requirement and Metal's cache folder). Test: `helpers_follow_the_edition_and_releases_keep_theirs` (edition.rs). Builds: the before and after tables below | (this commit) | 2026-11-23 |
| R2 every embedded plist names itself | PASS | `CFBundleName` and `CFBundleDisplayName` "AI Juggler" in both helper templates; each helper's check (`helper_plist_check`) reads the embedded plist back and fails without them (a build from the old plist fails: "CFBundleName is 'juggler-apple-ai', not 'AI Juggler'; CFBundleDisplayName is None") | (this commit) | 2026-11-23 |
| R3 a lint keeps both true | PASS | `scripts/helper_identity_lint.py` (+ `--selftest`) in `just verify-static` and CI's static job. Against the files before this change it reports the hard-coded identifiers (build_apple_ai_helper.sh:33, build_local_ai_helper.sh:47, apple.rs:40, juggler-local-ai-protocol lib.rs:46, the Apple plist) and both plists' missing names | (this commit) | 2026-11-23 |
| R4 a release-notes line | PASS | `docs/releases/0.5.1.md`, Known issues; `scripts/release_notes_check.sh 0.5.1 <v0.5.0's commit>` OK | (this commit) | 2026-11-23 |

## A Dev build's helpers, before

The Dev .app built from release/0.5 on 2026-11-22 (`scripts/build_dev_app.sh`), `codesign -dv --verbose=4` and `launchctl plist __TEXT,__info_plist`:

| Helper | Identifier | Signature | Embedded plist |
|---|---|---|---|
| juggler-apple-ai | `ai.figurehead.juggler.apple-ai` (the same as Nightly's, which the team signs) | adhoc, runtime | CFBundleIdentifier `ai.figurehead.juggler.apple-ai`, CFBundleName `juggler-apple-ai`, no CFBundleDisplayName |
| juggler-local-ai | `ai.figurehead.juggler.local-ai` (the same as Nightly's) | adhoc, runtime | CFBundleIdentifier `ai.figurehead.juggler.local-ai`, CFBundleName `juggler-local-ai`, no CFBundleDisplayName |
| juggler-agent | `juggler_agent-c4f68fe78f0c1420` (the linker's); the lab copy, re-signed with `--deep`, took `juggler-agent`, the release's | adhoc, linker-signed | none |

The installed AI Juggler Nightly, for comparison: `ai.figurehead.juggler.apple-ai`, `ai.figurehead.juggler.local-ai` and `juggler-agent`, TeamIdentifier set (read only).

## A Dev build's helpers, after

Built with the Dev recipe's own steps, AIJ_EDITION unset (`scripts/build_apple_ai_helper.sh release /Users/you/e3822/dev`, `scripts/build_local_ai_helper.sh release /Users/you/e3822/dev`, and `sign_agent` from `scripts/build_dev_app.sh` on the release `juggler-agent`):

| Helper | Identifier | Signature | Embedded plist |
|---|---|---|---|
| juggler-apple-ai | `ai.figurehead.juggler.dev.apple-ai` | adhoc, runtime (flags=0x10002) | CFBundleIdentifier `ai.figurehead.juggler.dev.apple-ai`, CFBundleName "AI Juggler", CFBundleDisplayName "AI Juggler" |
| juggler-local-ai | `ai.figurehead.juggler.dev.local-ai` | adhoc, runtime (flags=0x10002) | CFBundleIdentifier `ai.figurehead.juggler.dev.local-ai`, CFBundleName "AI Juggler", CFBundleDisplayName "AI Juggler" |
| juggler-agent | `ai.figurehead.juggler.dev.agent` | adhoc | none (not sandboxed: no container) |

The same scripts with AIJ_EDITION=nightly (as the release workflow runs them) give `ai.figurehead.juggler.apple-ai` and `ai.figurehead.juggler.local-ai`, unchanged, with the same two names. Each script's check with an .app: a Dev helper against an AI Juggler app fails "isn't signed as ai.figurehead.juggler.apple-ai", and a release helper against a Dev app fails "isn't signed as ai.figurehead.juggler.dev.apple-ai".

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] No captures here
- [x] No real names or home paths (placeholders only)

**Checked:** by the building session (Opus), 2026-11-23.
