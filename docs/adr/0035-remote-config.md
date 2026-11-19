# ADR-0035: One signed remote configuration, in layers, read through typed modules

**Status:** Proposed (2026-11-18, coordinating session, Opus). Building starts once the maintainer has seen the summary.
**Amendment:** §11, the `models` section (AJ-E3810, D-131): Proposed 2026-11-18. No schema or code for it until it is accepted.
**Date:** 2026-11-18
**Deciders:** Opus (design); the maintainer (D-128, D-129)
**Serves:** D-128 (version support without a rebuild), D-129 (a general remote configuration: flags, controls, messages, and account-level capabilities); [ADR-0014](./0014-optional-figurehead-account.md) (the optional account); [ADR-0015](./0015-usage-statistics.md) (opt-in stats, closed vocabulary); [ADR-0019](./0019-extension-architecture.md) (features as modules); [ADR-0023](./0023-local-cache.md) (the registered cache); D-52 (OS seams); D-127 (never show what can't be used)

## Context

**What the maintainer asked for (2026-11-18):**
- Version support should be updatable "from a separate remote file so we don't have to recompile and deploy whole app".
- It should then be widened into "a general config file", able to "disable or enable certain things or perform other control or maybe messages for the app".
- "for logged in users we should have additional API level capabilities".
- "architecture this carefully so it is future proof as we keep expanding our app".

**What exists today:**
- **The app works with no account and no network.** The account is optional (ADR-0014) and silent until linked; its device token lives in the Keychain.
- **Stats** are opt-in, with a closed vocabulary (ADR-0015).
- **Updates** come through the release channels (nightly, beta, stable) with signed builds.
- **Version knowledge is compiled in.** Adapters have version gates (`PROVEN_*`), and the fidelity register ships in the binary. D-128 replaces most gates with run-time shape checks, but some knowledge will always change faster than releases do.

**The risks a remote config adds,** each answered below:
1. **Tampering:** a forged or modified file.
2. **Rollback or replay:** an old, validly signed file sent again, to undo a kill switch.
3. **Privacy leaks:** fetching it must not identify the user.
4. **Remote code by stealth:** config growing into behaviour.
5. **Outages:** the app must never depend on the file being there.
6. **Schema drift:** old apps meeting new files, and new apps meeting old files.

## Decision

### 1. One envelope, many modules

The app reads one document per channel. It's a signed envelope around named **sections**, and each section belongs to exactly one feature **module** that registers a typed schema for it, the way features register with the app (ADR-0019). The config crate knows envelopes, signatures, layering and caching. It knows nothing about any feature.

```text
RemoteConfig envelope (JSON, canonical form for signing)
├─ format: 1                     the envelope's own version
├─ key_id: "rc-2026-a"           which pinned public key signed it
├─ channel: "stable"             nightly | beta | stable
├─ serial: 418                   strictly increasing per channel
├─ issued_at / expires_at        UTC; expired files are ignored
├─ min_app / max_app             app versions this file applies to (optional)
└─ sections: {                   each one is owned by a registered module
     "compat":   {...},          tool versions, shape-check versions, route state (D-128)
     "flags":    {...},          feature switches and rollouts
     "notices":  [...],          in-app messages
     "params":   {...},          tunables: intervals, limits, budgets
     "endpoints":{...},          service URLs, for the app's own services only
     "models":   {...},          the tested AI models the app may offer (§11)
     "<future>": {...}           unknown sections are kept and ignored
   }
signature: ed25519 over the canonical bytes of everything above
```

- **Modules register `Section<T>`** with a name, a schema version, a `serde` type, a validator and a merge rule. A section that fails its validator is dropped on its own, and the rest of the file still applies.
- **Unknown sections and unknown fields are ignored, never errors.** That's forward compatibility: an old app meets a new file. A missing section falls back to compiled defaults, which is backward compatibility: a new app meets an old file.
- **Each section carries its own `schema` number.** A module accepts the versions it knows and ignores newer ones until the app updates. A breaking change gets a new section name (`flags2`), never a reinterpretation.

### 2. What the file may and may not do

The rule that keeps this safe, and keeps it future-proof: **the file only selects among behaviour already compiled into the app; it never adds behaviour.**

- **Flags** are declared in code (`flag!(name, default, remote: OffOnly | OnOff | Rollout)`). Most flags are `OffOnly`: remote can switch a feature off (a kill switch), not on. A flag that's safe to turn on remotely says so in code, and a reviewer sees that. Security and privacy settings are never remote-controllable.
- **Params** are declared with typed bounds in code (`param!(sync_interval_s, 60, 30..=3600)`). Values outside the bounds are clamped and logged.
- **Notices** are data, not markup:
  - severity (info, needs-you, broken, mapped to D-117's levels and the D-126 Alert);
  - plain title and body text, with no HTML;
  - at most one link, to an allow-listed host (our website or docs);
  - targeting (below);
  - an expiry;
  - whether it can be dismissed;
  - and an id, so a dismissal is remembered.

  Notices never trigger actions. A "do something" notice can only open a screen that's already in the app, chosen from an enum.
- **Endpoints** may only point at our own service hosts, from a compiled allow-list. They can't redirect the account, the update or the stats traffic anywhere else.
- **Compat** carries D-128's support list: per tool and feature, the version ranges, the shape-check versions and the route state. Like a flag, it can narrow or widen only within what the code's shape checks support.

### 3. Trust: signing, keys, rollback

- **ed25519 signatures** (`verify_strict`) over a context string and the canonical bytes (sorted keys, no insignificant whitespace, integers only). The file carries the payload as opaque base64 beside its signature, and verification happens before any parse of it.
- **Pinned keys with key ids and roles.** The app ships 2–3 public keys: the current one and the next one, which sign configuration, and an emergency one, which signs only revocation lists. A key is retired by shipping an app without it; **for a stolen configuration key, that release is the real defence.**
- **Revocation lists** (amended 2026-11-18 after the step-1 security review):
  - signed by the emergency key only; a file may carry several, one per emergency-key generation, and a list this app can't verify is skipped, never fatal;
  - they name keys by **fingerprint** (SHA-256 of the public key), and an app keeps only fingerprints of keys it pins, so a list can't pre-revoke a later release's keys;
  - they are in-band, so an attacker who holds a stolen key and controls the host can withhold them. Each release therefore also compiles in the revoked fingerprints known at build time, and the publisher keeps the list in every file until no supported app pins the revoked key.
- **Where the private keys live** (amended 2026-11-18, D-129's publishing details):
  - `current` is a GitHub environment secret, never in the repo and never stored on a laptop: it is generated on the maintainer's Mac and piped straight into the secret, so the seed passes through memory only. It sits in `config-publish` (a required reviewer, `main` only, for new content) and in `config-resign` (no reviewer, `main` only, for the weekly re-sign, which can only re-sign the live content unchanged).
  - `next` and `emergency` follow the updater's standby model: their private halves stay offline in the maintainer's password manager, never as GitHub secrets, and only their public keys are compiled in. Before a rotation (or a revocation) their only job is availability.
- **Rollback protection:** the app stores the highest `serial` it has accepted per channel and key, and rejects a lower one (the same serial only with identical content). It also rejects files past `expires_at`, about 14 days by default and at most 31 days after `issued_at`, so a stale file can't be replayed forever. The publisher must re-sign periodically. A weekly CI job does this unattended, changing only the serial and the dates (the content must stay byte-identical in canonical form), and a daily job alerts if a live file is within 3 days of expiry.
- **Time:** expiry is judged against `max(now, the newest issued_at accepted)`, so setting the clock back revives nothing, and an `issued_at` more than 24 hours in the future is rejected. The day of skew is a trade-off: a slow or wrong clock isn't shut out, at the cost that a file may apply up to a day early and its window may end up to a day past the 31-day cap; that still takes a pinned key, and the cap is still measured from the trusted time.
- **Where the trust state lives:** the floors, the revoked fingerprints and the trusted time are security state, not cache. They live in their own checksummed file in the app's durable data folder (Application Support, never Caches), written atomically (temp file, fsync, rename, fsync the folder). **Clearing the cache never touches it.** A save only moves forward (floors by maximum, revocations by union), and refreshes are serialised. If the file is missing or corrupt (read in full and failing its checksum or parse), the app fails safe: it starts from the floor and revocations compiled into the release (the serial of the newest file known at build time), never from "no floor". If it can't be read *now* (permissions, I/O, no file handles), the refresh fails closed: nothing is fetched, saved or newly served, and the next refresh reads it again; a save never overwrites a file it couldn't read. Each read-modify-write holds an advisory lock beside the file. A build that pins keys must also compile in floors (a build-time check).
- **Channel binding:** a file for `beta` is rejected by a `stable` app.

### 4. Layers and precedence

From lowest to highest; a higher layer overrides a lower one, section by section, by each module's merge rule:

1. **Compiled defaults:** always present. The app is complete with nothing else.
2. **The public channel file:** the same for everyone on a channel. It's fetched anonymously from a static host (a CDN or GitHub release asset).
3. **The account file,** only when an account is linked: per-account and per-entitlement overrides. It's fetched from the account API with the device token, and signed by the server's own pinned key (§6).
4. **The user's own settings:** a user's explicit choices win over remote *preferences*, never over remote *safety*. Kill switches (`OffOnly` flags set off) and compat narrowing beat user settings. Notices respect the user's dismissals.

`effective = merge(defaults, public, account, user)` is one pure function. The Settings › Advanced "About this configuration" view shows, per value, which layer set it, so support and debugging never guess.

### 5. Targeting, evaluated on the Mac

Targeting rules live in the file and are evaluated locally, so the anonymous fetch sends nothing about the user. Rules match on:
- app version range;
- channel;
- OS and OS version (D-52);
- installed tool versions;
- and a **rollout bucket**: a stable hash of a random per-install id, generated locally, never sent, and resettable in Settings.

A 10 % rollout means "buckets 0–9". No server ever learns who is in which bucket.

### 6. Logged-in users: the account layer and APIs

With an account linked, the app may also call the account API (ADR-0014's client) for:
- **The account file** (layer 3): per-account flags (beta programs), entitlements (a paid tier's features, such as option D's cloud summaries later), and targeted notices (billing, account security).
- **Typed capability endpoints,** added per feature behind the same transport seam, each with its own ADR when it arrives: sync, Fleet, cloud summaries, support diagnostics on request.
- **Signed responses:** the server signs the account file with its own pinned key, so a hijacked TLS session or a compromised CDN can't grant entitlements. The client also checks the account id inside the file against the linked account.

Unlinked users get layers 1, 2 and 4 only, and lose no core feature (D-18).

### 7. Fetching, caching, failure

- **When:** at launch (after first paint, per the lazy-load rule) and then at most once a day (D-128), failed attempts included. The background fetch skips Low Power Mode and metered connections where the OS says so.
- **How:**
  - one GET to a fixed URL per channel; `If-None-Match` only if a policy turns it on (off by default: a host could hand each install a unique entity tag and so recognise it);
  - no cookies;
  - a generic user agent (`AIJuggler/<version>`), with no install id, no account and no query string;
  - a timeout of 10 s;
  - a size cap of 256 KB.
- **Cache:** the last *accepted* file is kept with the trust state (§3) and re-verified on every load. A new file is used only after full verification and after the state recording it has been saved; if that save fails, nothing new is used. On any failure (network, signature, serial, expiry, size, parse), the app keeps the last good file, or falls back to compiled defaults if it's past its expiry. Clearing the registered cache (ADR-0023) never touches the trust state.
- **Old apps, new files:** a value an app can't read (a wider bound, a new option, a new field) drops only that value; a rule gated to newer app versions isn't read at all; kills and compat narrowing always apply, even from a section with a newer schema.
- **Off switch:** Settings › Privacy offers "Check for configuration updates". It's on by default, because it carries kill switches and compat; it's anonymous, and the page says what it fetches. Turning it off uses compiled defaults plus the account layer, if linked.
- **Applying:** sections apply live through a change event (no restart), in a module-defined order. A change that can't apply live is marked `on next launch`.

### 8. Publishing and governance

- **The source of truth** is a directory in the repo (`config/remote/<channel>.json`), unsigned. A change is a PR.
- **CI validates** every section against the modules' schemas, as exported by the app's build. It also dry-runs the merge against the last three released app versions, and blocks a change that would switch a flag on that's declared `OffOnly`.
- **Promote-style workflow:** a `Publish config` workflow, behind the same environment approval as releases, bumps the serial, signs, uploads to the static host and keeps the last 10 signed files for audit. Re-signing before expiry is automatic.
- **Emergency kill switch:** the same workflow with a `--kill <flag>` input. It still needs approval.
- **Observability:** with stats opted in (ADR-0015), the weekly report adds only the applied `serial` per channel, a number, so we know the file is reaching people.

### 9. Code layout

- **`crates/juggler-remote-config`** (new; pure Rust; no Tauri, no UI, so it works on every OS):
  - the envelope;
  - canonical bytes;
  - verification with pinned keys;
  - serial and expiry state;
  - the `Section` registry;
  - the layer merge;
  - targeting and bucketing;
  - the fetch through an injected transport (recordable, like the account client).

  It must not know: any feature, tool or screen.
- **Feature modules** register their sections in their own crates or features: compat in juggler-adapter, flags in each feature, notices in a small `features/notices` with its UI.
- **`juggler-account`** gains `account_config()` (layer 3) and the server-key check.
- **UI:**
  - one `useRemote(section)` hook over a Tauri command, returning typed values;
  - notices render through the D-126 Alert component, or a quiet banner on Home;
  - the "About this configuration" view.

### 10. Testing

- **Unit:**
  - canonical bytes;
  - a bad signature, an unknown key, a revoked key, a lower serial, an expired file, the wrong channel, an oversize file;
  - unknown sections and fields ignored;
  - per-section validator isolation;
  - the precedence table;
  - the bucket's stability.
- **Property tests:** merge is idempotent, and the defaults layer alone equals today's behaviour.
- **A golden file per channel** in CI, plus the dry-run against old app versions.
- **A recording transport** proves the fetch sends no identifying header or query.
- **A fault-injection run:** the host down, a truncated file, a slow host. The app starts and works identically.

## Consequences

**Good:**
- Version support, kill switches, rollouts, notices and tunables change without a release.
- New features add a section and are done.
- Old apps keep working with new files.
- Account features have a clear, signed path.

**Costs:**
- A signing key to guard.
- A re-sign job that must run.
- One more privacy statement on the Settings page.
- Reviewers must check each flag's `remote:` mode.

**Not chosen:**
- **A third-party flag service:** it adds a vendor, sends identifiers and doesn't fit the privacy promise.
- **Remote scripts or rules engines:** that's remote code.
- **Unsigned files over TLS only:** that trusts the host and the CDN.
- **Per-user server-side targeting for anonymous users:** it needs an identifier.

## Order of work

1. **This crate and the compat section (D-128),** with the publish workflow, in 0.5.
2. **Flags and notices,** with their UI, in 0.5.
3. **Params and endpoints,** as features need them.
4. **The account layer and the server key,** with the next account feature (cloud summaries or sync), ADR per capability.

## Amendment: the `models` section (AJ-E3810, D-131)

**Status:** Proposed (2026-11-18, Opus). Written before any schema or code, as AJ-E3810 asks.
**Serves:** D-131 (the Models area and its signed list), D-130 (the acceptance bar and opt-in Beta), D-121 §2 (disk rules, as amended), D-127 (never show what can't be used), D-125 (pinned revision and SHA-256 per file); AJ-E3811, AJ-E3812, AJ-E3813 and AJ-E3803 read this section.
**Supersedes:** [ADR-0033](./0033-feed-summary-engines.md) §5's compiled `ModelPin` slot ("no manifest and no signing key"). The pin moves into this signed section, so a tested model can be added or withdrawn without a release. Everything else in ADR-0033 §5 (the click, resume, the transfer conditions, the cache entry) stays, read through this section.

### 11. The `models` section

**What it is.** A list section (§2, `list.rs`) owned by the AI models module (`crates/juggler-app/src/features/ai/models/`). Each record describes one model the app may offer: who makes it, which engine runs it, where its one file comes from, what it costs in disk and memory, and how it did on our tests, job by job. It describes models. It never carries code, an engine, a path on this Mac, or a URL off the allow-list.

**The rule that keeps it safe:** data, not code, adds a model. The engines, the hosts, the hardware facts and the jobs are compiled. The section only picks among them. There is no branch on a model id anywhere in the app: the recommendation (AJ-E3811), the screen (AJ-E3812), the download (AJ-E3813) and the router (AJ-E3802) read the typed record and nothing else.

#### 11.1 The entry (R1)

Every record has the list's own `id` (`[a-z0-9_-]`, at most 64, unique) and may have a `when` (§5's targeting, so a model can reach nightly first, or only macOS 26 and later). Its fields:

| Field | Kind | Required | What it says |
|---|---|---|---|
| `name` | text, ≤ 40 | yes | The display name: "Qwen3 1.7B". |
| `engine` | choice: `gguf_local`, `apple_builtin` | yes | Which compiled engine runs it. Any other value drops the **whole** entry (11.3). |
| `url` | link, ≤ 512 bytes | `gguf_local` only | The pinned download, on an allow-listed host and path (11.2). |
| `mirror_url` | link, ≤ 512 bytes | no | A second source for the same bytes, tried only if `url` fails. Same hash, same size. |
| `sha256` | SHA-256, 64 lower-case hex | `gguf_local` only | The file's hash. |
| `size_bytes` | whole number, 1 B to 8 GiB | `gguf_local` only | The file's exact size. |
| `licence_name` | text, ≤ 40 | `gguf_local` only | "Apache-2.0". |
| `licence_url` | link, ≤ 512 bytes | `gguf_local` only | The licence text, on `huggingface.co` at the same pinned revision, or on our website. |
| `min_chip` | choice: `m1` … `m5` | yes | The oldest Apple silicon generation it runs on. A chip newer than the app knows counts as newer than all. Intel Macs never qualify. |
| `min_memory_gb` | whole number, 8 to 256 | yes | The least memory it needs. |
| `min_free_gb` | whole number, 0 to 512 | no | Extra free disk to require. It can only tighten: the margin is the larger of this and the compiled rule, size plus 10 GB (D-121 §2.5). |
| `speed` | choice: `quick`, `moderate`, `slow` | no | How fast it ran on the harness's reference Mac, before the user's own Mac has run it. |
| `temperature_milli`, `top_p_milli`, `top_k`, `min_p_milli`, `presence_milli` | whole numbers within compiled bounds | no | The sampling the harness tested it with (round 3's chosen arm). Missing means the engine's default. They are tunables, like §2's params. |
| `context_tokens` | whole number, 1024 to 32768 | no | The context it was tested with. |
| `datamark` | flag | no | Whether the facts guard marks quoted data for this model (harness setting). |
| `feed`, `review`, `attention`, `chat`, `digest`, `watches` | choice: `ships`, `beta`, `not_yet`, `not_tested` | no | The verdict per job. Missing, or a state this app doesn't know, reads as `not_tested`. |
| `feed_note` … `watches_note` | text, ≤ 120 | no | One plain line per job: why the verdict is what it is. |
| `withdrawn` | flag | no | No longer offered (11.4). |

- **The model's chat template** is always the GGUF file's own. The engine passes `enable_thinking = false` to every template; a template without that switch ignores it. No setting per model is needed, so none exists.
- **An `apple_builtin` entry** may carry no `url`, `mirror_url`, `sha256` or `size_bytes`: an entry that does is dropped whole, so the section can never make Apple's engine fetch anything. At most one `apple_builtin` entry is read; a second one is dropped.
- **A `gguf_local` entry** without all of `url`, `sha256`, `size_bytes`, `licence_name` and `licence_url` is dropped whole.
- **The user sees the verdicts in words** from the words file: `ships` "Works well", `beta` "Beta", `not_yet` "Not yet", `not_tested` "Not tested", followed by the entry's plain line.
- **What a verdict does.** The router's Automatic choice (AJ-E3802) sends a job to a model only when its verdict is `ships`, or `beta` and the user has turned that job's Beta on. The per-job override offers the same models only (D-127). A verdict can never turn a job on: each job has its own flag in `flags`, declared in code (D-130), and that decides whether the job exists at all.

**Two new field kinds for every list section,** since a model needs them and nothing in them is model-specific (`list.rs`, ADR §2):
- **a whole number** with compiled `min` and `max` (canonical JSON already carries integers only);
- **a SHA-256:** exactly 64 lower-case hex characters.

Cross-field rules (the engine's required fields, the URL's path, the hash inside our own path) are the models module's record check. A record failing it is dropped whole and reported, like any record a list can't read. The publisher runs the same check: a test in the app crate reads `config/remote/<channel>.json` and holds every `models` entry to it, besides the exported schema.

#### 11.2 The hosts (R2)

The hosts, the path shapes, the official organisations and the redirect hosts are compiled into the models module. The section can't widen any of them.

| Source | The pinned URL | Redirects |
|---|---|---|
| **Hugging Face, official organisations only** | `https://huggingface.co/<org>/<repo>/resolve/<40-hex commit>/<file>.gguf`, with `<org>` exactly one of `Qwen`, `ibm-granite`, `nvidia`, `google`. A branch or tag name in place of the commit is refused. | Followed only to Hugging Face's published file hosts: `cdn-lfs-us-1.hf.co`, `cdn-lfs-eu-1.hf.co`, `us.aws.cdn.hf.co`, `us.gcp.cdn.hf.co`, `transfer.xethub.hf.co`, `transfer.xethub-eu.hf.co`, `cas-server.xethub.hf.co`, `cas-server.xethub-eu.hf.co`. The list is taken from Hugging Face's firewall page when the downloader is built and re-checked at each release; a recorded HEAD response at that step settles the exact redirect target. |
| **Our own files** (needs the maintainer's setup: open question 3) | `https://models.figurehead.ai/v1/<sha256>/<file>.gguf`, where `<sha256>` must equal the entry's own hash. | None. Any redirect is a failure. |

- **Why only official organisations:** with a stolen configuration key, the attacker still can't point the app at a file they uploaded to their own account. They'd need write access to an official repository or to our bucket as well.
- **Our own files are for:**
  - **Qwen3-4B-Instruct-2507,** which has no official GGUF. Per D-131 we convert it ourselves from the official safetensors, and never ship the third-party test file. That conversion fetches the official weights, so it waits for the maintainer's yes in chat.
  - **Mirrors** of official files, used as `mirror_url` when Hugging Face moves its file hosts or a network blocks them.
  
  The path is addressed by the hash, so a file there can't change under its name. Each file has its `LICENSE` (and any NOTICE) beside it, as Apache-2.0 asks of a redistributor.
- **A new organisation or host is a release,** on purpose: it widens what a stolen key could reach.
- **Licence links** may go to `huggingface.co` (at the same pinned commit), `figurehead.ai` or `www.figurehead.ai` (our acknowledgements page, for a licence the model's repository doesn't carry as a file).
- **Only repositories that need no account.** The app never sends a token, so a gated repository would fail. The harness step that records a verdict (11.5) notes that the file was fetched with no account, and the ledger keeps that note.

#### 11.3 Engines (R3)

`engine` is a compiled choice with two options. An entry naming any other engine, or a file format other than GGUF (the path must end in `.gguf`), is dropped whole, not shown with its other fields. An engine arrives only in an app release, with its own ADR (as ADR-0033 did for these two). A newer app that knows a third engine reads entries an older app drops, and the older app simply doesn't list them.

#### 11.4 Withdrawn models and the kill switches (R4, R2)

**Each kind of stop, from softest to hardest:**
- **Stop offering a model:** set `withdrawn: true`, or remove its entry.
  - It leaves Recommended and Other tested models at once.
  - If it's installed, it stays in Installed, still working, with the note from the words file: "We no longer list this model. It keeps working until you remove it."
  - Nothing is deleted for the user (D-131 §3).
- **Stop one job using a model:** lower that job's verdict (`beta` to `not_yet`). This is narrowing and applies at once; the job falls back to another model or to its plain path.
- **Stop a job everywhere:** the job's own Beta flag in `flags`, declared `OffOnly` (D-130).
- **Stop running every downloaded model:** a `local_models_on` flag, `OffOnly`, declared by the local engine's module. This is the security stop, for a flaw in the bundled model runner. The files stay until the user removes them; jobs fall back to Apple's model or the plain path. It isn't a field of this section, so no per-model "stop" exists in the data.

**The installed copy.** When a model is installed, the app keeps a copy of its typed entry beside it in the registered cache entry (AJ-E3803, D-58). That copy is what lets it work offline and with no section. It is refreshed from any later entry with the same `id` **and the same `sha256`** (so verdicts can narrow or widen), and never from one with a different file.

**What an absent section means:**

| What the app has | Offered (Recommended, Other) | Installed models |
|---|---|---|
| No accepted file yet, the check turned off, or the file past its expiry | The built-in list (11.6) | Work, from their installed copy, with no note |
| A file whose `models` section is missing, unreadable, or of a newer schema | The built-in list | Work, with no note: the app can't tell |
| A file whose section reads, and lists the model | The section's entries, plus the built-in Apple entry unless the section has its own | Work, with the entry's current verdicts |
| A file whose section reads, and the model is absent or `withdrawn` | As above, without it | Work, with the withdrawn note |

A section of a newer schema gives no records (§2's list rule, "nothing in a list is a kill switch"). A breaking change to this section is therefore a new name (`models2`), published beside `models` until no supported app reads only the old one.

#### 11.5 Only tested models, and who records a verdict (R5)

A model is listed only after it passes D-130's bar for at least one job, on the harness, on **the exact file** its entry pins (its SHA-256). Attention doesn't count on its own: it is decided by code from the facts, so it says nothing about the model's writing.

**How a verdict is recorded:**
1. **The harness run** on that file writes its results under `docs/qa/evidence/<task>/` (a Sonnet session may drive it).
2. **Opus confirms each per-job verdict** against D-130: no fabrication reaching the user (the ≤ 1 % bar, 0 on a set of 50), omissions within about 10 %, and blind preference of at least 60 % of decided pairs. It then writes one line per model to `config/remote/models-ledger.json`. The ledger lives in the repo only, never in the shipped file. Each line holds:
   - the id, the hash, the size and the URLs;
   - per job: the verdict, the plain line, the evidence path, "confirmed by Opus", the date and the commit.
3. **The publisher's check** refuses a `models` entry unless:
   - the ledger has its id with the same hash, size and URLs;
   - every verdict is the ledger's or lower (a published file may narrow without a new test, never raise);
   - at least one job other than attention is `beta` or `ships`;
   - each cited evidence path exists.
4. **The maintainer's approval** in `config-publish` releases it, as for every other section.

**The ledger is append-only per id.** An id never changes its file: a new quantisation, revision or conversion is a new id. The app enforces the same thing for what it has installed (11.7).

**Qwen3-4B-Instruct-2507:** its round-3 verdicts were measured on the third-party file. Our own conversion has a different hash, so its entry is published only after a verification run of round 3's frozen sets on our file passes the same bar. That is a check of the file, not a new evaluation round (D-131's wrap-up).

#### 11.6 The built-in list (R6)

The app compiles in one entry, Apple's on-device model, with the verdicts known at build time. It is written in the same JSON shape and read by the same reader (a test proves it), so there is no second code path. With no network and no file, Settings › AI › Models still works:
- **On a Mac with Apple Intelligence on,** Apple's model is recommended, with no download.
- **Otherwise** the screen says, in one plain line, that no model fits this Mac yet, and every job keeps its plain path (AJ-E3811 R3).

No downloadable model is compiled in: its URL, hash and verdicts belong to the signed list, and a compiled one would be the stale pin this amendment removes.

#### 11.7 Rollback, downgrade and size bounds

- **Rollback** is the envelope's own: a lower serial, an expired file, or the wrong channel is refused before the section is read (§3). Fail closed: on any failure the last accepted file stays in force (§7).
- **A changed file under an old id** (a different `sha256`, `size_bytes` or source) is refused by the publisher (11.5). On the Mac, such an entry for an installed id is ignored and reported. The installed copy keeps working, and nothing is fetched or replaced by itself; a new file is a new id and needs the user's click like any download.
- **An older app** meets the section with its own schema: unknown fields are ignored, unknown verdict states read as `not_tested`, and unknown engines drop the entry. Each of these errs towards offering less.
- **Layers:** the section is read from the public channel file only (layer 2). The account layer may not carry it until an ADR for a capability says so; a user's own per-job choices (layer 4) pick only among what the section allows.
- **Size bounds:**
  - at most 16 entries;
  - names and licence names at most 40 characters, per-job lines at most 120;
  - links at most 512 bytes;
  - a file at most 8 GiB;
  - every number within its compiled bounds.

  Together they keep the section well inside the file's 256 KB cap.
- **When it's read:** lazily, when Settings › AI › Models opens or a job asks the router. Never on a timer (AJ-E2023's rule); the file itself is fetched on §7's schedule.

#### 11.8 Security review points

- **URL parsing.** One parser serves the publisher and the app, and its rules are compiled:
  - `https://` only;
  - no userinfo (`@`), no port, no IP literal, no percent-encoding, no backslash, no query and no fragment in the pinned URL;
  - the host matched exactly against the source's list, in lower case;
  - path segments are `[A-Za-z0-9._-]` and never `.` or `..`;
  - the commit is 40 hex characters, and the file ends in `.gguf`.

  The same rules hold for the licence links.
- **Redirects.**
  - At most 3.
  - Each hop is checked like the pinned URL against the source's redirect hosts, and a hop off the list ends the download.
  - The query Hugging Face adds to its own signed redirect is the only query ever sent.
  - A resume starts again from the pinned URL, because the signed redirect expires.
- **Hash before load.**
  - The file is hashed as it arrives and checked in full after the last byte, before it is renamed into place.
  - A mismatch deletes it and tells the user. A file that fails is never loaded.
  - A verified file gets ADR-0033's stamp, and any change re-verifies before the next load.
  - The file name on disk is `<sha256>.gguf` in the cache folder, never a name from the URL, so no text from the section ever becomes a path.
  - The file is data for the out-of-process helper (ADR-0033 §4.2). It is never run.
- **A size mismatch is a reject:**
  - a `Content-Length` or `Content-Range` total that differs from `size_bytes` stops the download before any byte is written;
  - a byte past `size_bytes` aborts it;
  - a final size that differs deletes the file.

  The free-disk check uses `size_bytes`, so a lying host can't fill the disk past the margin.
- **The text is data only.** `name`, the licence name and the per-job lines are §2 plain text: no control, direction or markup characters. On top of that:
  - they are rendered as text nodes, never as HTML or Markdown;
  - they are length-capped as above;
  - they hold no `://`, so they show no links;
  - the publisher runs the plain-words lint on them (no internal terms, calm wording).
- **What a stolen configuration key could do here:** list a different file from an official repository or our bucket, mark verdicts up, or withdraw models. The app still asks for the user's click, checks the hash, the size and the disk margin, and keeps each job behind its own compiled flag. Code supplies the facts in every AI line (ADR-0033 §6). The defence for the key itself is §3's.
- **Privacy.**
  - A model is fetched only after the user's click (AJ-E3813 R1).
  - The request is a plain GET, with §7's generic user agent and no cookies, no token, no referrer, no install id and no query of ours.
  - The file's host learns the network address, the file and the time, as with any download, and nothing more. Our own host keeps no request logs (Logpush off, as for the config host).
  - The Privacy page says this in one sentence.

#### 11.9 Worked examples

The Qwen3 entry uses the pinned manifest (`docs/qa/evidence/AJ-E3709/models-manifest.md`); the verdicts and lines come from round 3's final table (`docs/qa/evidence/AJ-E3709/v5-final.md`). Sampling is left out, so the engine's defaults apply; the build step copies round 3's chosen arm from `v5-design.md` §7 into the ledger and the entry.

```json
"models": {
  "schema": 1,
  "items": [
    {
      "id": "apple-on-device",
      "when": { "os": ["macos"], "os_version": { "min": "26.0.0" } },
      "name": "Apple's on-device model",
      "engine": "apple_builtin",
      "min_chip": "m1",
      "min_memory_gb": 8,
      "feed": "not_yet",
      "feed_note": "The plain line reads better for now.",
      "review": "beta",
      "review_note": "Preferred over the plain review in 81 % of blind checks.",
      "attention": "ships",
      "attention_note": "Built from the facts; no misses in testing.",
      "chat": "beta",
      "chat_note": "Preferred over the plain answer in 75 % of blind checks.",
      "digest": "not_yet",
      "digest_note": "The plain roundup reads better for now.",
      "watches": "not_yet",
      "watches_note": "Drafts matched the request 70 % of the time; we want 95 %."
    },
    {
      "id": "qwen3-1-7b-q8",
      "name": "Qwen3 1.7B",
      "engine": "gguf_local",
      "url": "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/90862c4b9d2787eaed51d12237eafdfe7c5f6077/Qwen3-1.7B-Q8_0.gguf",
      "sha256": "061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a",
      "size_bytes": 1834426016,
      "licence_name": "Apache-2.0",
      "licence_url": "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/blob/90862c4b9d2787eaed51d12237eafdfe7c5f6077/LICENSE",
      "min_chip": "m1",
      "min_memory_gb": 16,
      "speed": "quick",
      "feed": "not_yet",
      "feed_note": "The plain line reads better for now.",
      "review": "beta",
      "review_note": "Preferred over the plain review in 89 % of blind checks.",
      "attention": "ships",
      "attention_note": "Built from the facts; no misses in testing.",
      "chat": "beta",
      "chat_note": "Answers in about half a second; preferred in 76 % of blind checks.",
      "digest": "not_yet",
      "digest_note": "The plain roundup reads better for now.",
      "watches": "not_yet",
      "watches_note": "Drafts matched the request 81 % of the time; we want 95 %."
    }
  ]
}
```

- **What the app does with these:**
  - **On a Mac with Apple Intelligence on:** Apple's model is recommended ("Needs no download"). Qwen3 1.7B is listed under Other tested models.
  - **On a 16 GB Mac without it:** Qwen3 1.7B is recommended (1.83 GB), offered only with at least 1.83 GB plus 10 GB free.
  - **On an 8 GB Mac without it:** nothing is recommended. The Qwen3 card isn't shown (D-127), and every job keeps its plain path.
- **The first entry is also the built-in list's** (11.6).

#### 11.10 Tests

- **Schema:**
  - a valid entry of each engine;
  - an unknown engine and a non-GGUF path, each dropping the whole entry;
  - an `apple_builtin` entry carrying a URL;
  - a host off the list, an organisation off the list, a branch name in place of a commit;
  - userinfo, a port, an IP literal, percent-encoding, `..`;
  - our own path whose hash differs from the entry's;
  - a missing hash, a short hash, an upper-case hash;
  - a size of 0 and one over 8 GiB;
  - a 41-character name and a 121-character line;
  - markup and direction characters in text;
  - an unknown field (ignored), an unknown verdict state (read as `not_tested`);
  - 17 entries;
  - a downgraded entry: the same id with another hash, refused by the publisher and ignored on the Mac.
- **Signing:** an unsigned `models` section, or one in a file that fails §3, is never read; the last accepted one stays.
- **Absent and withdrawn:** each row of 11.4's table, the installed copy surviving each, and `local_models_on` off.
- **The ledger check:** a raised verdict, a missing evidence path, an entry with only attention passing, and a new file under an old id are all refused.
- **The built-in list** reads through the same reader and gives a working screen with no network.
- **The download** (AJ-E3813, against a stub server, never a real model): a redirect off the list, a size mismatch in the header and in the body, a hash mismatch, and resume through the pinned URL.

#### 11.11 Order of work

1. **This amendment,** accepted.
2. **The two field kinds** in `list.rs`, then the typed section, its record check, the built-in list and `config/remote/schema.json` (AJ-E3810).
3. **The ledger and its publisher check,** filled from the manifest and round 3 for the five official files; Qwen3-4B after its conversion and verification run.
4. **The recommendation, the screen and the download** (AJ-E3811 to AJ-E3813) on the stubbed section. No model is downloaded while building.

## Open questions for the maintainer

1. **Where the public files are hosted:** our website's CDN, or GitHub release assets? Both are static; the website gives nicer URLs and caching control.
2. **The default for "Check for configuration updates":** on by default (proposed, because kill switches and compat must reach people), with the Privacy page explaining it.
3. **Where our own model files are hosted (§11.2), a setup step only you can do.** Proposed: a new Cloudflare R2 bucket, `figurehead-models`, with the custom domain `models.figurehead.ai`. It would be public read and have Logpush off, like the config bucket. Its write token would sit only in a new `models-publish` environment, with a required reviewer and `main` only. A separate bucket keeps the config publisher's token from writing model files, and the other way round. R2 has no egress fee, which matters for 2 to 3 GB files. It would hold our Qwen3-4B-Instruct-2507 conversion and any mirrors, each under its hash, with its licence beside it. Until it exists, entries use Hugging Face only, and Qwen3-4B isn't listed.
