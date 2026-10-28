# E12 — Optional Figurehead account (two paths: desktop / cloud)

> Source decision: [D-18](../review/decision-log.md) (2026-10-17) and
> [ADR-0014](../adr/0014-optional-figurehead-account.md). The server side
> is the Figurehead API contract (the endpoints, request and response
> shapes quoted in these tasks are the parts the app implements).
>
> Law for every task here: the **desktop path is complete on its own**
> and, with no account, the app makes **zero** calls to figurehead.ai;
> every cloud feature has its own toggle and says what it sends;
> content is encrypted on the Mac; vendor credentials never leave it;
> no state-changing action without a click; every cloud feature is free
> at launch and the app never hard-codes free vs paid.

## Feature availability (the two paths)

| Feature | Desktop (no account) | Cloud (linked account, toggle on) |
|---|---|---|
| Switching, usage clocks, cleanup, onboarding, the Inspector | ✅ always | never needs an account |
| Backups | ✅ local export and restore (AJ-E1009) | ✅ encrypted upload; restore on any linked Mac (AJ-E1204/E1205) |
| Usage statistics | ✅ local view; optional anonymous sharing (E11) | ✅ personal history across Macs on the dashboard (E11 personal sync) |
| Fleet | ✅ LAN / own-VPN pairing with a code (E7) | ✅ your linked Macs listed automatically; pinned after you confirm the fingerprint on both (AJ-E1206) |
| Alerts | ✅ in-app, notifications, tray | later (seam only) |
| Config / rules sync | — | later, end-to-end encrypted (seam only) |
| Transcript vault | ✅ local archive (AJ-E1007) | later, needs its own decision (seam only) |

Every cloud feature is free at launch; availability and limits come
from the account's entitlements, never from code.

### AJ-E1201 Decision D-18, ADR-0014, stale-rule updates | Status: DONE-EVIDENCED | Priority P0 | Effort S | Depends: none
**Goal**: Record the optional-account decision and remove every "no cloud, no account" rule it supersedes, before any code.
**User story**: As a contributor, I read one decision and one ADR that say exactly what the cloud path may and may not do.
**Requirements**:
1. The decision log SHALL carry D-18 "Optional Figurehead account: two paths (desktop / cloud)" as SETTLED, quoting the maintainer, with the five points of the decision.
2. ADR-0014 SHALL record the architecture and mark ADR-0004's "cloud relay / vendor backend — rejected outright" as partly superseded (discovery allowed, relay still out of scope).
3. README, PROPOSAL, SECURITY and SPEC §1 (privacy row), §8 (Keychain items, allowed endpoints), §11 (account-based discovery) and §12.1 (optional onboarding step) SHALL state the two-path rule; no document SHALL still promise "no cloud, no account".
4. This epic and its tasks SHALL be added to TASK-CATALOG.md and the task README.
**UI/UX reasoning**: Documentation is the source for in-product and website copy; Nielsen #2 (match the real world): "Works fully offline with no account. An optional Figurehead account adds cloud features you choose to turn on."
**Performance budget**: n/a (documents only).
**Quality gates**: `just verify` (evidence + terminology gates) green; `grep -rn "No cloud, no account" docs README.md SECURITY.md` returns nothing.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1201/manifest.md` listing each updated rule with its file; pass = every row PASS and the grep empty.
**Visual testing**: n/a (no UI change).
**Files to touch**: `docs/review/decision-log.md`, `docs/adr/0014-optional-figurehead-account.md`, `docs/adr/README.md`, `docs/adr/0004-fleet-transport-grpc-mtls.md`, `README.md`, `docs/PROPOSAL.md`, `SECURITY.md`, `docs/SPEC.md`, `docs/tasks/TASK-CATALOG.md`, `docs/tasks/README.md`.
**Research basis**: D-18 (maintainer instruction 2026-10-17); ADR-0004; SPEC §1/§8/§11/§12.1.
**Handover note**: The usage-statistics spec gets its own decision (D-19) and ADR (ADR-0015) — don't fold it into D-18. Never copy server internals into this repo; write "the Figurehead API contract".

### AJ-E1202 Account client: link (browser + code), Keychain token, 401/unlink, entitlements cache | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1201, AJ-E702
**Goal**: A small, audited HTTP client that links this Mac to a Figurehead account, keeps the device token only in the Keychain, and keeps the account's entitlements current.
**User story**: As a user with two Macs, I approve "Studio Mac" in my browser once and AI Juggler shows my account and plan — and if I unlink it on the website, the app notices and turns cloud off without losing anything.
**Requirements**:
1. With no account linked the client SHALL make no network call to figurehead.ai (pinned by a recording-transport guardrail).
2. Browser linking SHALL `POST /api/v1/devices/link-requests` with `name` (default: the Mac's name, editable, ≤ 64 chars), `platform` (e.g. `macOS 26 · arm64`), `appVersion` and `publicKey` (`ed25519:<b64>;x25519:<b64>` — the AJ-E702 Fleet identity key plus a new X25519 key), open `verifyUrl` in the default browser, show `userCode`, and poll `GET …/link-requests/{requestId}` with the `pollToken` every `interval` seconds; `approved` stores the token, `denied`/`expired` (410) say so with Retry, `429 slow_down` honours the new interval, `404` restarts.
3. Pairing-code linking SHALL `POST /api/v1/devices/claim` with the typed code (case and dashes ignored) and the same device fields; `400 invalid_or_expired_code` renders a plain reason.
4. The device token SHALL be stored only in the Keychain (service `ai.figurehead.juggler`, account = device id) and SHALL never appear in SQLite, settings, logs, packs, backups or crash reports (grep for the `fhd_` prefix pinned in tests).
5. Every call SHALL send `Authorization: Bearer <token>` (when linked), `User-Agent: AIJuggler/<version>` and `Content-Type: application/json` (including bodiless `DELETE`; backup uploads use `application/octet-stream`), with a 15 s timeout and bounded exponential backoff on network errors and `5xx`.
6. A `401` from any endpoint SHALL delete the token, turn every cloud toggle off, keep all local data, and show "This Mac was unlinked from Figurehead"; **Unlink this Mac** SHALL `DELETE /api/v1/devices/self` and then do the same cleanup even if the call fails or the Mac is offline.
7. `GET /api/v1/account` SHALL be called at launch (when linked), after linking, and every 6 h; the last answer SHALL be cached; features SHALL be shown and enabled only from `entitlements` (unknown keys ignored; `available: false` hidden; `enabled: false` + `tier: "paid"` → disabled with Upgrade opening `upgradeUrl`); a `404` SHALL fall back to Phase-1 defaults (backup, discovery, usage sync enabled); a `402 upgrade_required` SHALL stop that feature's uploads and show Upgrade, and SHALL never delete local data.
8. Renaming this Mac SHALL `PATCH /api/v1/devices/self` with the new name.
9. Debug builds SHALL honour `AI_JUGGLER_FIGUREHEAD_URL`; release builds SHALL always use `https://figurehead.ai`.
**UI/UX reasoning**: Nielsen #1 (visibility of status): linking shows the code, the browser step and a live "Waiting for approval…" state; Nielsen #9: every failure (denied, expired, offline, unlinked) is named in plain words with one recovery action.
**Performance budget**: Link request + first poll ≤ 2 s on a normal connection; polling never faster than `interval`; account refresh off the UI thread; zero calls when unlinked.
**Quality gates**: Unit tests against a local mock server for every status code above; guardrails "signed out is silent", "token only in Keychain", "401 cleanup"; clippy `-D warnings`; no `unsafe`.
**Evidence/validation proof**: `cargo test -p juggler-account` (all status-code cases) + the guardrail tests; one end-to-end link against a local Figurehead dev server recorded in `docs/qa/evidence/AJ-E1202/manifest.md`; one production link approved by the maintainer (recorded PARTIAL-BY-DESIGN until then).
**Visual testing**: Covered by AJ-E1203's captures (connecting, connected, unlinked, offline).
**Files to touch**: new `crates/juggler-account/`, `crates/juggler-app/src/commands.rs` (account commands), `crates/juggler-app/src/lib.rs`, `ui/src/lib/bridge.ts`.
**Research basis**: ADR-0014; the Figurehead API contract (device linking, account, devices/self).
**Handover note**: Trap: the poll token and the device token are different secrets — never log either. The token is returned once on approval; a lost response means a fresh link, never a retry of the same request.

### AJ-E1203 Settings → Account & Cloud, onboarding step, Cloud chips, consent screen | Status: DONE-EVIDENCED | Priority P0 | Effort L | Depends: AJ-E1202
**Goal**: One home for everything account-related, a skippable onboarding step, and a consent screen that states exactly what each cloud feature sends.
**User story**: As a privacy-conscious user, I can see every cloud feature, what it sends, and turn each one on or off — and if I never connect, the app never asks again.
**Requirements**:
1. A **Settings** screen SHALL exist (the app's first; it also carries the D-16 Appearance choice: System / Light / Dark, kept in sync with ⌘⇧L) with an **Account & Cloud** section as the single home for this epic.
2. Signed out, the section SHALL explain the two paths in one paragraph and offer **Connect account** (browser) and **Use a pairing code**; desktop features SHALL show no account prompts; features with a cloud half SHALL show a small **Cloud** chip that opens this section; there SHALL be no pop-ups, banners or repeat reminders.
3. Onboarding SHALL add one optional step after the policy choice: "Connect a Figurehead account (optional)" with **Connect account** and **Continue without an account** as equal buttons and one line on what Cloud adds; skipping is the default path and SHALL not count against any click budget it would break.
4. After linking, a consent screen "Turn on Cloud for this Mac" SHALL list the entitled features, each with a toggle, what it does, and exactly what it sends; defaults: Cloud backup ON, Fleet discovery ON, Sync usage stats OFF (disabled with "Arrives with usage statistics" until E11), every future feature OFF; **Turn on selected** confirms; choices are stored with a timestamp and a `cloud_consent_version` and the screen re-appears when that version bumps.
5. Connected, the section SHALL show the account email and plan, this Mac's editable name, per-feature toggles with last activity, **Open dashboard** (browser), **Unlink this Mac**, and a link to delete cloud data on the website; a paid-and-not-entitled feature SHALL render a disabled toggle with **Upgrade** (never at launch, but it must work).
6. Copy SHALL never imply the desktop path is lesser ("You can do everything without an account; Cloud adds…") and SHALL say what is sent and that it is encrypted.
**UI/UX reasoning**: Consent clarity (GDPR-style, Nielsen #3 user control): per-feature toggles with consequence text; Apple HIG settings structure; "never nag" — no unsolicited prompts.
**Performance budget**: Settings opens ≤ 100 ms (cached account); toggle writes ≤ 50 ms locally.
**Quality gates**: vitest for each state and the consent defaults; Playwright gallery captures; a11y axe 0 both themes; terminology gate.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1203/manifest.md`; pass = every Req has a test, gallery complete, a11y green.
**Visual testing**: Signed out, connecting (browser code shown), consent screen, connected, paid-locked (mocked), unlinked, offline — 1280×800 and 1024×680 × light/dark; onboarding step at 1280×800 and 390×844.
**Files to touch**: new `ui/src/screens/Settings.tsx`, `ui/src/App.tsx` (nav), `ui/src/screens/OnboardingWizard.tsx`, `ui/src/lib/bridge.ts`, `ui/tests/visual/*`.
**Research basis**: D-16 (Settings Appearance), D-18; ADR-0014; wireframe 26 (Settings).
**Handover note**: The onboarding step must not break AJ-E305's ≤ 5-click flow (a): it is skippable with one click and counted honestly.
*Req.3 evidence, amended 2026-10-27 (D-46):* the onboarding step left first run with the D-25 redesign (commit 91b640d6) and is back in 0.1.9 as first run's last step: `ui/src/screens/FigureheadStep.tsx` (reusing `useFigurehead`/`FigureheadFlow` from `ui/src/components/FigureheadCloudCard.tsx`, the Settings flow), wired in `ui/src/screens/FirstRun.tsx`; tests in `ui/src/screens/first-run.test.tsx` ("D-46 the optional Figurehead account": equal buttons, nothing pre-selected, skip finishes, Connect starts the same link, Add account never asks).

### AJ-E1204 Vault key, recovery code, encrypted backup format | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1009
**Goal**: A documented, versioned, end-to-end encrypted container for the AJ-E1009 backup set.
**User story**: As a user, I save one recovery code and know that Figurehead can never read my backups.
**Requirements**:
1. Turning on cloud backup the first time SHALL generate a 32-byte random vault key stored in the Keychain (service `ai.figurehead.juggler.vault`) and show a **recovery code** once (base32, groups of 4, Copy and Save), requiring "I've saved my recovery code" before the first upload, with the plain warning "Without this code, a new Mac can't open your backups. We can't recover it for you."
2. The container SHALL be: zstd-compressed tar of the AJ-E1009 backup set (which never includes the Fleet `fleet/` folder — identity keys and peer pins stay on the Mac), encrypted with XChaCha20-Poly1305 (RustCrypto `chacha20poly1305`), prefixed by the magic `FHB1` and a version byte, with `aad = "figurehead-backup-v1"`; the format SHALL be documented in `docs/` (an ADR if non-trivial).
3. Decryption SHALL accept the Keychain key or the recovery code and SHALL reject a wrong key, a wrong magic/version, or tampered ciphertext with a named reason.
**UI/UX reasoning**: Apple HIG for irreversible information: the recovery code is shown once with explicit confirmation; Nielsen #5 error prevention.
**Performance budget**: Encrypt/decrypt ≥ 100 MB/s; a 25 MB container in < 1 s.
**Quality gates**: Round-trip, tamper, wrong-key and known-answer tests; the plaintext-canary guardrail (no seeded string in the ciphertext).
**Evidence/validation proof**: `cargo test -p juggler-account vault::` + format doc; `docs/qa/evidence/AJ-E1204/manifest.md`.
**Visual testing**: Recovery-code sheet at 1280×800 both themes; a11y: code is selectable text with Copy.
**Files to touch**: `crates/juggler-account/src/vault.rs`, `docs/` format note, Settings UI.
**Research basis**: ADR-0014; AJ-E1009 backup set.
**Handover note**: Never derive the vault key from the device token or the Fleet identity key.

### AJ-E1205 Cloud backup upload, schedule, restore | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1202, AJ-E1204
**Goal**: Back up to Figurehead on demand or on a schedule, and restore on any linked Mac through AJ-E1009's verified restore path.
**User story**: As a user replacing my laptop, I link the new Mac, enter my recovery code, and get my setup back.
**Requirements**:
1. Upload SHALL `POST /api/v1/backups` (`application/octet-stream`, optional `X-Backup-Label` ≤ 120 chars, ≤ 25 MB or the entitlement's `maxBytes`), on demand and on an optional schedule (daily/weekly, default weekly); one upload at a time; never during an active switch.
2. Restore SHALL list `GET /api/v1/backups`, download `GET /api/v1/backups/{id}`, verify `X-Backup-Sha256`, decrypt (Keychain key or recovery code), verify the manifest, and hand off to the AJ-E1009 restore preview + confirm step.
3. With cloud backup toggled off, no `/backups` call SHALL be made (guardrail).
4. `413` and `402` SHALL stop uploads with a named reason; local data is never deleted.
**UI/UX reasoning**: Nielsen #1: "Last backup 2 h ago", progress during upload/restore; destructive-adjacent restore keeps AJ-E1009's preview.
**Performance budget**: Upload off the UI thread; restore verification < 30 s for 25 MB.
**Quality gates**: Mock-server tests for upload/list/download/sha mismatch; toggle guardrail.
**Evidence/validation proof**: two-Mac (or two data-dir) round trip with the recovery code against a local dev server, recorded in `docs/qa/evidence/AJ-E1205/manifest.md`.
**Visual testing**: Backup list, upload in progress, restore preview at 1280×800 both themes.
**Files to touch**: `crates/juggler-account/`, `crates/juggler-app/`, Settings UI.
**Research basis**: ADR-0014; the Figurehead API contract (backups); AJ-E1009.
**Handover note**: The server keeps at most a bounded number of backups; never rely on an old one being present.

### AJ-E1206 Fleet discovery via account | Status: DONE-EVIDENCED | Priority P1 | Effort M | Depends: AJ-E1202, AJ-E704
**Goal**: Your own linked Macs find each other without a pairing code.
**User story**: As a user with a laptop and a Mac mini on the same account, I open Fleet and both are already listed; I confirm the fingerprint on each and they are paired — no code to type.
**Requirements**:
1. With Fleet discovery on, Fleet SHALL list peers from `GET /api/v1/devices`; with it off, no `/devices` call SHALL be made.
2. A listed peer SHALL be pinned **only after the user confirms its fingerprint on both machines** (each shows the other's fingerprint from the directory's `publicKey`), never silently; such peers SHALL start at View and be labelled "via your Figurehead account"; the TLS handshake SHALL still prove possession of the pinned key (ADR-0013 architecture contract).
3. The connection SHALL stay machine-to-machine (LAN or the user's VPN) over mutual TLS (ADR-0004); there is no relay.
4. Unlinked, Fleet SHALL pair with the 6-digit code or QR exactly as E7 specifies.
**UI/UX reasoning**: Nielsen #6 recognition over recall: linked Macs appear by name with "via your Figurehead account".
**Performance budget**: Directory refresh ≤ 1 s; no polling faster than once a minute.
**Quality gates**: Mock directory test; **substitution test** (a directory entry carrying an attacker key is never pinned without the two-sided confirmation, and a confirmed pin whose TLS key differs is rejected); discovery-off zero-call test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1206/manifest.md` with the two-node loopback run.
**Visual testing**: Fleet list with account-discovered peers, 1280×800 both themes.
**Files to touch**: `crates/juggler-link/`, `crates/juggler-account/`, Fleet UI.
**Research basis**: ADR-0014; SPEC §11; E7.
**Handover note**: A key from the directory is a candidate, not trust — a compromised account can offer a key, so the two-sided fingerprint confirmation is the trust step, and the TLS handshake still proves possession.

### AJ-E1207 Guardrail tests and visual captures | Status: DONE-EVIDENCED | Priority P0 | Effort M | Depends: AJ-E1202, AJ-E1203, AJ-E1205
**Goal**: Prove the two-path law in tests.
**User story**: As a maintainer, I can point to a test for every promise the cloud path makes.
**Requirements**:
1. Signed out is silent: a scripted session with no account makes zero connections to figurehead.ai (recording transport).
2. Toggles mean it: linked with a feature off, that feature's endpoint is never called (backup, discovery, usage sync).
3. No plaintext leaves the Mac: seeded strings (an Account label, a project path, `$HOME`) never appear in uploaded bytes, which start with the format magic.
4. No credentials: a filesystem-access audit during backup shows no reads of Provider credential files.
5. Token handling: the token exists only in the Keychain (the `fhd_` prefix appears nowhere else).
6. `401` cleanup: a mocked `401` removes the token and turns every toggle off; local data unchanged.
7. Entitlements drive the UI: a mocked paid-and-disabled feature shows Upgrade and makes no calls; `402` stops uploads.
8. The desktop path is unchanged: the full existing suite passes with the account feature compiled in and never used.
**UI/UX reasoning**: n/a (test task) — the visual half is AJ-E1203's gallery.
**Performance budget**: guardrail suite ≤ 30 s.
**Quality gates**: all eight green in `just verify`.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E1207/manifest.md` mapping each guardrail to its test.
**Visual testing**: AJ-E1203's captures.
**Files to touch**: `crates/juggler-account/tests/`, `crates/juggler-app/tests/guardrails.rs`.
**Research basis**: ADR-0014.
**Handover note**: A guardrail that needs the network to pass is wrong — every one runs against a recording transport or a local mock.
