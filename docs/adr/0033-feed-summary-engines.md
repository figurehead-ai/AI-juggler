# ADR-0033: Feed summary lines: one `SummaryEngine` seam, Apple's model automatic, a local model offered, helpers out of process, a gate every line passes

**Status:** Proposed (AJ-E3710; design awaiting the coordinating session's review before AJ-E3711 starts)
**Date:** 2026-11-16
**Deciders:** Opus (design); the coordinating session (hosting and integrity, 8 GB Macs, build order: recorded below as decided); the maintainer (D-120; the open questions at the end)
**Serves:** D-120 point 2 (B), on A (AJ-E3708); D-118 points 2 and 4; D-117 (levels come from facts); D-52 (OS seams); D-58 and ADR-0023 (the registered cache); ADR-0019 (features as modules, tools as data); ADR-0010 (the spawn ledger); ADR-0024 (turn ids)

## Context

- **The ruling (D-120 point 2, as first given; its tier 2 is amended below).** On-device AI turns an agent's own text into one clean line for the Home feed and never adds facts. It runs by itself when the Mac can do it comfortably; the user is told once and can turn it off in Settings; the app never says "please download a model". It degrades: (1) Apple's Foundation Models on macOS 26+ with Apple Intelligence on; (2) else a small open model fetched in the background, on suitable hardware, unmetered network, power, Low Data Mode respected, checksummed, resumable, disk-capped; (3) else A's excerpts. The user's own running Ollama or LM Studio may be used, never required. Nothing leaves the Mac.
- **The research** ([`feed-summaries-on-device.md`](../research/feed-summaries-on-device.md), measured on this Mac):
  - The Apple model answers in 0.5 to 1.5 s warm, 2.5 to 2.8 s cold, runs in system daemons, not in our process, and needs Swift: there is no Objective-C header.
  - It refused 1 of 8 harmless texts under the default guardrails. The permissive guardrails setting Apple documents for transforming the user's own text returned a good line.
  - 2 of 8 lines broke the 20-word cap. 0 of 8 added a fact.
  - An extractive check caught every invented path, name and function in 990 of 990 cases.
  - macOS 27's SDK also ships `PrivateCloudComputeLanguageModel`, which sends text to Apple's servers.
  - Our minimum macOS is 14.0 ([`tauri.conf.json`](../../crates/juggler-app/tauri.conf.json)). The workspace forbids `unsafe`. Swift linked into the app already forced [`ensure_swift_runtime.sh`](../../scripts/ensure_swift_runtime.sh) into existence.
- **The machine signals exist.** Power, Low Power Mode, thermal state, memory pressure and free disk are read by `juggler-supervisor/src/machine.rs` on the existing 2 s processes tick (`machine_health.rs`). Two signals are new: the network path's cost flags, and the Apple model's availability. Both are Swift-only APIs.
- **The idle budget is already missed.** AJ-E805 R3: 5.7 % of one core idle on the release app, against under 2 % ([manifest](../qa/evidence/AJ-E805/manifest.md)). A summariser must add nothing at idle, and its cost must be measurable on its own (off against on).
- **Decided by the coordinating session (2026-11-16), recorded here as decisions, not options:**
  1. **Hosting and integrity:** no new signing key.
     - The model file is pinned by its SHA-256, compiled into the notarized app, so the app's own code signature vouches for the hash.
     - It is fetched from an immutable source: a Hugging Face URL pinned to a revision commit.
     - It is verified before the first load and deleted on a mismatch.
     - GitHub release assets cap at 2 GiB a file, so a self-hosted mirror would mean splitting. It may come later, behind the same pinned hash.
     - The model's licence must allow redistribution: Apache-2.0 or MIT preferred, and the chosen candidates' licences are recorded.
  2. **8 GB Macs:** Apple's model and excerpts, no local model.
  3. **Build order in 0.5:** A (excerpts), then tier 1 (Apple) with the extractive check, then tier 2 (the local model).
  4. **Still the maintainer's:** the measurement download (Q1) and the battery measurement (Q5). See the open questions.
- **The maintainer's correction to D-120 (2026-11-16, recorded as D-120's "Amended 2026-11-16").** It overrides "fetched automatically": "if we are going to download 2GB model for our users, than it should always be an option or as a backup to maybe mac … we can't download 2GB models without user approval".
  - **Tier 1, Apple's model,** stays automatic: no download, told once, a Settings switch.
  - **Tier 2, the local open model,** is **offered, never fetched without the user's click:**
    - an offer card says what the user gets, the size and that it stays on the Mac, and one click gets it, under every safety rule above;
    - it is removable in Settings;
    - it is offered mainly on Macs without Apple Intelligence, and as an optional upgrade otherwise;
    - it is never offered on 8 GB Macs.
  - **Model choice** is a named slot here. A selection study of the top 30 open candidates, run by the coordinating session, fills it. Its licence must allow redistribution, Apache-2.0 or MIT preferred. Nothing is downloaded until that study picks one.
  - **Not in scope, noted for later:** for users who got the file, "some conversation with it too" (asking questions about their sessions on the Mac). §12 says how the seam allows it.

## Decision

### 1. One seam: `SummaryEngine`, one gate, one service

The feature is a module per ADR-0019: `crates/juggler-app/src/features/feed/summary/`. It has a service trait, its own cache registration, its own Settings section, and events on the app's typed bus. The feed never knows which engine wrote a line.

**Input.** A's excerpt (AJ-E3708) and AI Juggler's own facts, never a transcript read of its own:

```rust
pub struct TurnKey { pub conversation: ConversationId, pub turn: String /* ADR-0024 "{generation}:{offset}" */, pub revision: u32 }

pub struct LineRequest<'a> {
    pub key: &'a TurnKey,
    pub text: &'a str,          // A's excerpt source: the agent's own words, already redacted (D-14) and bounded
    pub facts: &'a SourceFacts, // AI Juggler's facts: state, exit, counts, files, project, branch: allowed words for the gate
    pub locale: &'a str,        // the Mac's language, BCP 47
    pub max_words: u8,          // 20
}

pub struct Draft { pub text: String, pub prompt_version: u16 }

pub trait SummaryEngine: Send + Sync {
    fn id(&self) -> EngineId;                                    // Apple | LocalModel | UserServer(spec id) | Extractive
    fn readiness(&self, caps: &Capabilities) -> Readiness;       // Ready | Preparing(why) | NotHere(why)
    fn write(&self, req: &LineRequest, stop: &Stop) -> Result<Draft, EngineError>;
    fn choose(&self, req: &LineRequest, sentences: &[&str], stop: &Stop) -> Result<usize, EngineError> {
        Err(EngineError::Unsupported)                            // optional: "choose, don't write"
    }
    fn health(&self) -> Health;                                  // last latencies, failures, crashes, resident bytes if known
    fn release(&self);                                           // quit the helper or drop the connection now
}
```

**Errors.** `EngineError` is one closed enum; no variant carries text:

| Variant | Meaning |
|---|---|
| `Unavailable(why)` | The engine is not ready here |
| `Refused` | The model's guardrail or refusal |
| `TooLong` | The text exceeds the model's context |
| `Language` | The locale is not supported |
| `Busy` | Rate-limited or another request is running |
| `TimedOut` | Past the per-call limit |
| `Stopped` | Cancelled |
| `Crashed` | The helper exited or broke the protocol |
| `Unsupported` | The engine has no `choose` |

**Blocking calls on one worker thread.** The trait is blocking and object-safe, called only from one lazily started worker thread, `feed-summaries`. The repo uses no `async_trait`, and one job at a time is the rule anyway (§5). The thread parks when the queue is empty: no wake-ups and no timer.

**Cancellation.** `Stop` is a shared flag plus a deadline:
- each engine checks it between protocol messages, and the helpers between tokens;
- a newer turn for the same conversation, the switch going off, Last messages going off, or a skip rule (§7) sets it;
- the engine sends the helper `cancel` and returns `Stopped`.

**Output.** One closed type, with provenance always present:

```rust
pub enum LineSource { Written { engine: EngineId }, Quoted /* the agent's own sentence, chosen or copied */ }
pub struct SummaryLine { pub key: TurnKey, pub text: String, pub source: LineSource, pub prompt_version: u16, pub gate_version: u16 }
pub enum Outcome { Line(SummaryLine), NoLine(NoLineWhy) }   // NoLine: the feed shows A's excerpt, as it does today
```

**Every engine's draft passes one gate (§6)** in the service, never inside an engine. A new engine cannot skip it, and a later cloud engine (D or E, not scheduled) plugs into the same trait and the same gate without a change to the feed. No hook for C, D or E is built now.

**`Extractive` is the always-ready engine.** Its "line" is A's clean opening, the excerpt the feed already shows. It is what every failure falls back to, so a model engine can only ever replace an excerpt with something better.

**"Show original" is always one click away.** The feed DTO gains `summary: Option<FeedSummaryInfo { text, source }>` beside A's excerpt, which stays in the item:
- `Written` shows a small "Summary" mark and "Show original", which swaps the excerpt in place, by keyboard too, as D-118 point 6 requires;
- `Quoted` shows as the agent's quote, with no Summary mark.

The mark's words, its accessible label ("Summary written on this Mac") and every status sentence live in `ui/src/lib/feed-words.ts`. The line itself is data, like a quote: plain text, one line, control characters removed, at most 160 characters, never markdown, never a link.

### 2. What gets a line, and what never does

- **Calm items only.** Only finished turns at L0 or L1 (D-117) get a written line.
- **The agent's own words for L2 and L3.** A turn that asks you something (L2), or failed or broke (L3), shows the agent's own question or failure sentence (A's excerpt, chosen by A's rules) and AI Juggler's facts. No model rewrites them. A1 and A2 of the research can then never lose the question or the failure, and the loud items, D-117's one loud mark per need, rest on facts alone.
- **The state comes from facts.** The item's state label and level ("Finished", "Failed", "Needs you") always come from `items.rs` and `attentionLevel()`, never from the line.
- **Where a line never goes:** notifications, the bell, the menu bar, the pinned zone, Fleet (a paired Mac never gets one), usage statistics, crash reports or logs. It is a feed item's text and nothing else.
- **Gated by Last messages.** Lines are made only while Settings › Desktop apps › Last messages is on, the same gate as A. Turned off, every queued job stops and every stored line is dropped at once, as ADR-0023 point 6 does for subagent text.

### 3. Tiers from one capability snapshot, event-driven

**The snapshot.** One pure struct, rebuilt only when an input changes:

```rust
pub struct Capabilities {
    pub os: OsInfo, pub chip: Chip, pub ram_bytes: u64,
    pub cache_volume: Option<Room>,          // free and total, from juggler_platform::room
    pub power: PowerState, pub low_power: bool, pub thermal: ThermalState, pub pressure: PressureLevel,
    pub network: Option<NetPath>,            // expensive, constrained, satisfied; None = unknown
    pub apple: AppleModel,                   // Available{context, locale_ok} | NotEligible | NotEnabled | NotReady | NotThisOs | HelperMissing | Unknown
    pub local_file: LocalFile,               // Absent | Wanted{have, total} (the user clicked Get it) | Verified{bytes} | Failed{why, retry_at}
    pub prefer: EnginePreference,            // Auto (Apple first) | LocalFile (set when the user gets the file on a Mac that has Apple's model)
    pub user_server: Option<UserServerSeen>, // a spec id and a loaded model name, when one is running and the user turned it on
    pub switch_on: bool, pub last_messages_on: bool,
}
pub fn plan(caps: &Capabilities) -> Plan  // pure: { engine, offer: Offer, transfer: TransferDecision, status: Status, notice: Option<NoticeKind> }
```

**Inputs, and what changes each one:**

| Input | Changed by | Cost |
|---|---|---|
| OS, chip, RAM | read once at start, after first paint | microseconds |
| Power, Low Power Mode, thermal, pressure, free disk | `machine_health` publishes `AppEvent::MachineChanged`, only when one of these fields changes, from the 2 s tick it already runs | no new timer |
| Apple model availability | the Apple helper's `capabilities` answer, asked once after first paint and again only on a turn end when the last answer was `NotReady` or older than a day, and on a macOS version change (seen at start) | no timer: the turn end is the event |
| Network path | the Apple helper's `NWPathMonitor`, streamed only while a download the user asked for is waiting or running | event-driven, by the OS |
| Local file | the user's Get it or Remove, and the transfer's own state changes | — |
| User server | ADR-0032's process table already lists running programs; a spec's process appearing or leaving is the event; then one loopback request for its loaded model | no port scan, no polling |
| The switches | `app_setting_set` | — |

**The plan, in order, first that holds wins:**
1. Switch off, or Last messages off: `Off`.
2. The user got the local file on a Mac that also has Apple's model (`prefer = LocalFile`), the file is `Verified` and the machine qualifies (§4.2): `LocalModel`. They chose it, so it wins.
3. Apple model `Available` and the locale supported: `Apple`.
4. The user turned on their own server (§4.3), it is running, and it has a model loaded: `UserServer`.
5. The local file is `Verified` and the machine qualifies: `LocalModel`.
6. Otherwise `Extractive`.

In the same pass, `offer` says whether and where the local file is offered (§5), and `transfer` whether a download the user asked for may run now. `NotReady` (Apple is preparing its own model) uses `Extractive` meanwhile, asks again at a later turn end, and is not a reason to offer the file in Home.

**At call time, one model attempt, then the gate's ladder, then the excerpt.** A turn is never sent through Apple, then the local model, then the user's server, one after another: that would multiply the cost of a bad turn.

**Health:**
- An engine that crashes 3 times within an hour is marked unhealthy until the next start or a capability change. The plan moves to the next tier, and the status line says so.
- `Refused` and gate failures are normal outcomes, never health failures.

### 4. Process model: nothing runs a model in AI Juggler's process

**One helper protocol, two helpers, one client.**
- Both helpers speak newline-delimited JSON over stdin and stdout, one request at a time, protocol version in `hello`.
- One Rust client, `summary/helper.rs`, spawns, times out, cancels and restarts both: no duplicated child-process code.
- Messages: `hello` · `capabilities` · `watch_network` (stream) · `write {id, text, max_words, prompt_version}` · `choose {id, sentences}` · `cancel {id}` · `quit`.
- A request is at most 32 KiB, and the helper answers within the deadline or is killed.
- Each spawn is recorded in the spawn ledger (ADR-0010) with the role `SummaryHelper`. Its cleanup tier knows it.
- Each helper exits on its own after its idle time, and at once when stdin closes, so the app's death never leaves one behind (the "no stray processes" rule).
- Each runs at utility quality of service, set by the Swift helper itself and by the system's `taskpolicy` for the Rust one. No `unsafe` in our code.

#### 4.1 The Apple helper (`juggler-apple-ai`, Swift)

- **The app stays Swift-free and launches on macOS 14.** A single-file Swift program built with `swiftc` by `scripts/build_apple_ai_helper.sh`, outside `cargo`. Deployment target macOS 14.0, Foundation Models weak-linked and used only under `if #available(macOS 26, *)`. On 14 and 15 it still answers `capabilities` (the network path) and reports `NotThisOs` for the model. arm64 only: Apple Intelligence needs Apple silicon, and on Intel the app never spawns it.
- **Shipped like `juggler-agent`.** An `externalBin` in `Contents/MacOS/`, signed on its own by `sign_macos.sh` (hardened runtime, App Sandbox with no network entitlement) before the app. The bundle check refuses a release without it. `cargo run` without it built reports `HelperMissing`, and the plan uses excerpts, so development never needs Swift.
- **The calls:**
  - `SystemLanguageModel.default` only, with separate `Instructions` holding the rules and the agent's text as the prompt, temperature 0, at most 60 output tokens.
  - On `guardrailViolation` or `refusal`, one retry with `permissiveContentTransformations` guardrails. Apple documents that setting for transforming the user's own text; it circumvents nothing.
  - On `exceededContextWindowSize`, one retry with the text cut to the excerpt's opening and closing, sized by `tokenCount(for:)` where it exists (macOS 26.4+), else 4 characters a token against the reported `contextSize`.
  - `choose` uses guided generation of a single integer.
- **Never Private Cloud Compute: a code check, not a sentence.**
  - A source test fails on `PrivateCloudCompute` anywhere in the helper's source.
  - The release workflow fails if the built binary's symbols name it (`nm`).
  - The source may import only `Foundation`, `FoundationModels` and `Network`. From `Network` it may use only `NWPathMonitor`, so no `NWConnection`, `NWListener`, `URLSession` or socket call.
- **Idle exit after 60 s.** Its memory is ours only as the helper's own (24 MB measured); the model lives in Apple's daemons.

#### 4.2 The local-model helper (`juggler-local-ai`, Rust and llama.cpp; tier 2, after Q1)

- **Its own crate and binary.** `crates/juggler-local-ai/` on `llama-cpp-2` with Metal. A new dependency: its licence (MIT or Apache-2.0) is checked and its addition needs the maintainer's yes with Q1.
- **Never in `default-members`.** `just verify` and the shared target never compile llama.cpp's C++ (shared-machine limits). It has its own CI job on GitHub's runners (ADR-0025) and is built by the release workflow.
- **Never in the app's binary.** A C++ crash, a leak or a parsing bug in the model file can't take the app down, and the model's 2 to 3 GB are released by exiting.
- **The file it loads is a known file.** It loads exactly one file, at the path the app passes, which the app verified against the pinned SHA-256 (§5). The parser only ever sees bytes we measured.
- **Calls:** temperature 0, a 4 K context, 60 output tokens. `choose` uses a grammar that allows only a digit.
- **Idle exit after 120 s.** It is never started under memory pressure Warning or worse, and is quit when pressure reaches Warning.
- **Signing:** like the Apple helper, App Sandbox with no network entitlement, read access to the model's folder only. If the sandbox blocks Metal or the read, the spike records it, and the fallback is the dependency check of §9 plus the socket test.
- **Which Macs qualify:** Apple silicon, at least 16 GB of RAM (decided: 8 GB Macs get Apple's model and excerpts only, and are never offered the file), macOS 14+. Windows and Linux use the same contract with CPU or Vulkan, later (§10).

#### 4.3 The user's own server (Ollama, LM Studio): optional, last to build

- **Off by default.** It appears as one folded line in Settings › Summaries only while one is seen running: "Use the AI you run yourself (Ollama or LM Studio) when it's open". Using it spends the user's own loaded model and GPU, so it is their choice. Nothing is required of anyone who doesn't have one.
- **Servers are data, not branches.** Each is a `LocalAiServerSpec` in `juggler-adapters`: id, process names, default port, API style (Ollama's own or OpenAI-compatible), the route that lists loaded models.
  - One generic `UserServer` engine reads the spec and calls through the adapters' existing loopback client (`local_http.rs`), which builds only `127.0.0.1:port`. Its test `the_loopback_client_reaches_only_127_0_0_1` covers this client too.
  - The default port only: a server moved elsewhere by an environment variable can't be seen, because we never read another process's environment (D-37), and so isn't used.
- **Only a model already loaded.** It is never started, never asked to load or pull, and `keep_alive` is never changed. An empty loaded list means the next tier.
- **To check before shipping it (UNVERIFIED in the research):** Ollama's `/api/ps` route; whether LM Studio's server loads a model on demand when asked for one; LM Studio's terms for work use.

### 5. The local file: offered, got on the user's click (tier 2)

**Never fetched without a click** (the maintainer's correction). Nothing in this section runs until the user chooses **Get it**. Until then the app makes no request for the file, not even a size check.

**Where it is offered** (`plan()`'s `offer`, a pure function). The offer is made only on a qualifying Mac: Apple silicon, at least 16 GB of RAM and macOS 14+. 8 GB Macs never see it.
- **Mainly on Macs without Apple's model:** Apple `NotEligible`, `NotThisOs`, or `NotEnabled` (Apple Intelligence turned off).
  - The offer card is shown once in Home, above the feed and below the pinned needs, as a quiet L1 note (D-117), after the user has seen excerpts in the feed for at least a day.
  - After **Not now**, Home never shows it again; Settings › Summaries keeps the offer.
- **As an optional upgrade where Apple's model works:** in Settings › Summaries only, never in Home.
- **Not offered:** while Apple is `NotReady` (it is preparing its own model), with the switch off, with Last messages off, or when the volume could not keep 10 GB and 10 % free after the file lands. In the last case the line says why: "Needs about 2.5 GB more free space".

**The offer card** (wording for the build; every sentence in the Settings words module and `feed-words.ts`): "**Clearer lines in your feed, written on this Mac.** A small AI file (about 2.5 GB) can turn each finished turn into one clear line. It stays on this Mac, and nothing you read here leaves it. AI Juggler gets it once, on Wi-Fi and power, and you can remove it in Settings › Summaries." **Get it (2.5 GB)** · **Not now**. The size is the pin's own `bytes`, never a guess.

**The model is a named slot.** One data file, `summary/pins.rs`, holds a `ModelPin` per supported model. There is no manifest and no signing key (decided):

```rust
pub struct ModelPin {
    pub id: &'static str,                 // MODEL_SLOT: filled by the coordinating session's selection study
    pub urls: &'static [&'static str],    // https://huggingface.co/<org>/<repo>/resolve/<40-hex revision commit>/<file>.gguf; a mirror later, same hash
    pub bytes: u64,
    pub sha256: [u8; 32],
    pub licence: &'static str,            // SPDX; redistribution allowed; a test refuses anything but an allowlist (Apache-2.0, MIT first)
    pub licence_url: &'static str,
}
```

**What the slot requires:**
- **A licence that allows redistribution,** Apache-2.0 or MIT preferred. The quantised file's own repository must carry the same licence: the vendor's own file first, otherwise a converter's repository whose licence is stated. The licence text and any NOTICE go into the app's acknowledgements.
- **It must pass the measurement plan** of the research and the gate of §6. The research's first candidates (Qwen3-4B-Instruct-2507 and SmolLM3-3B, Apache-2.0; Phi-4-mini-instruct, MIT) are inputs to the study, not a pick. Llama 3.2's and Gemma's terms are the kind the slot refuses.
- **A size the offer can state honestly:** about 2 to 2.5 GB at 4 bits. A file under GitHub's 2 GiB asset cap keeps a future self-hosted mirror unsplit, and that weighs in a candidate's favour on a tie.

Nothing is downloaded, in development or for users, until the study fills the slot and the maintainer approves the measurement download (Q1).

**Integrity.**
- The app's code signature and notarization vouch for the compiled-in hash, so the host is not trusted. A revision-commit URL can't change under us.
- The full SHA-256 (`sha2`, already a dependency) is checked after the last byte and before the file is renamed into place.
- A mismatch deletes the file, and the status line says "Couldn't get it this time. Try again". There is no automatic retry of a bad file.
- A verified file gets a stamp (size, inode, modified time, pin id). Any difference re-verifies before the next load.
- A file never runs; it is data for the helper.

**The transfer, once the user clicked.**
- `ureq`, already a dependency, from `summary/fetch.rs`, which joins `only_the_listed_crates_can_reach_the_network` and `the_app_reaches_only_its_own_addresses` with exactly the pin's hosts.
- HTTPS only. Redirects are followed only to hosts on a compiled list: `huggingface.co`, `hf.co` and Hugging Face's own file CDN names, checked at the spike. No cookies, no credentials, a plain user agent with no identifier.
- **Resume:**
  - It writes `<pin>.part` and a small `<pin>.part.json` (offset, the first response's `ETag` and size).
  - It resumes with `Range: bytes=N-` and `If-Range`, through the pin's URL each time, because the CDN's signed redirect expires.
  - It accepts only `206` with a `Content-Range` starting at N and totalling the pin's size; anything else starts again from 0. A half file is never loaded.
- **At most 4 MB/s**, in 8 MB chunks. Between chunks it checks the current plan, which events keep current. It pauses, keeping the part file, the moment a condition fails, and goes on by itself when they all hold again. The user's click covers finishing that one file, never another.

**Conditions while it runs** (`plan()`'s `transfer`). All must hold:
- on power, Low Power Mode off, thermal Nominal or Fair, memory pressure Normal;
- the network path satisfied, not expensive (hotspot, cellular) and not constrained (Low Data Mode). Unknown counts as no;
- at least 10 GB and 10 % of the volume free after the file lands.

A paused download says why on its status line ("Waiting for Wi-Fi and power"). **Cancel** deletes the part file.

**Retries.** A transfer error retries after 1 hour, 6 hours, then 1 day, only for the file the user asked for: one-shot timers armed only after a failure, never a polling loop. A network, power or wake event also retries.

**Disk cap and cleanup.**
- The file lives in the registered local cache (ADR-0023) as its own entry, "On-device summaries": `Trim::WholeOnly`, with its own Remove, so its size shows in Settings › Advanced › Local cache. Like the search index it is never trimmed for the 2 % budget, and Clear cache… leaves it: it is the user's own download, and getting it again costs 2.5 GB.
- One file and one part file at most, together at most 3 GB.
- Removed:
  - by **Remove** in Settings › Summaries;
  - on ADR-0023's near-full disk, and when free space falls below its low-disk line (5 GB or 5 %) during a transfer, with the part file deleted too. The status line then says "Removed to free space. Get it again";
  - when an app update changes the pin: the old file is removed. The new one is offered again, never fetched by itself.
- Turning the summaries switch off stops using the file but keeps it, and the status line offers **Remove (2.5 GB)**.

**What's shown.**
- The offer card, once (above).
- Settings › Summaries' status line (§8).
- Never a progress bar in the feed, never an error card, never a fetch the user didn't start.

### 6. Faithfulness: one gate for every engine

**A pure module, `summary/gate.rs`, with its fixtures.** It is versioned (`gate_version`), and a version change makes cached lines stale. In order:

1. **The input guard.** Sentences aimed at a summariser are taken out before any engine sees the text. These are imperative phrases addressed to "you", "the summary" or "above", such as "ignore", "say that" or "summarise as", from a short tested list. If one was found, the engine goes straight to `choose` (step 4) over the remaining sentences: research A6.
2. **The extractive check.** Every fact token in the draft must appear in the source text or in `SourceFacts`' named fields (project, branch, files, counts, app). Fact tokens are:
   - numbers, with units and the number words up to twenty, hundred and thousand mapped to digits on both sides;
   - paths and file names;
   - code-shaped words (snake_case, camelCase, `::`, backticks);
   - identifiers (versions, hashes, `#123`, `AJ-E…`, `D-…`);
   - URLs;
   - capitalised words mid-sentence.

   Matching is case-insensitive after NFKC.
3. **The outcome check.**
   - Each outcome or negation word in the draft (fail, error, pass, fixed, broken, blocked, waiting, done, not, no, never, can't, from one tested stem list) must appear in the source.
   - When the facts say something failed (a non-zero exit, a failed-test count, a failed state), the draft must carry a failure word.
   - A draft whose words are all in the source but whose verdict contradicts the facts ("all tests pass" when the facts count failures) fails here.
4. **The shape check.**
   - One sentence, at most 20 words and 160 characters.
   - The repo's redaction pass (D-14) must leave it unchanged. A secret-shaped token in a line means it is rejected, never shown redacted.
   - A draft that copies 90 % or more of one source sentence is relabelled `Quoted`: honest provenance, no "Summary" mark.

**The ladder.**
1. `write`, then the gate.
2. On failure, one `choose`, if the engine has it, over at most 8 candidate sentences of the source. The chosen sentence is a quotation by construction, labelled `Quoted`, and still goes through steps 2 to 4.
3. Then A's excerpt.

Every rung is silent to the user: a refusal, a timeout, a crash or a failed gate all look like the feed of today.

**Tests:**
- **Fixtures.** The 20 measurement texts of the research, stored as fixtures (this repository's own commit bodies, trailers removed) with their injected-fact variants, plus A1 to A6, hand-written.
- **A property test:** an invented path, name, function or count added to any true line is always rejected.
- **A fake engine** returns adversarial drafts (invented facts, a flipped verdict, a secret, an obeyed instruction, 40 words). None is ever shown, and each falls to the right rung.
- `a_line_never_changes_an_items_state_or_level`.
- `a_question_or_a_failure_is_never_rewritten`, for L2 and L3 (§2).
- **The research's 990-text corpus** stays a script in `docs/qa/evidence/AJ-E3711/`, not a unit test: CI's shallow clone hasn't the history. It measures the false-alarm rate per engine for the evidence manifest.

### 7. Budgets and skip rules

**Event-driven only.** A job is queued when:
- a turn ends (the existing turn-end events, AJ-E3705);
- or an item without a line enters the view, and its turn is still the conversation's newest.

There is never a timer or a "refresh", and no backlog sweep on start: after a night away, lines fill in as items are seen.

**The queue.**
- One job at a time, in priority order: turn ends in view, then items entering the view, then turn ends out of view.
- One pending job per conversation. A newer turn replaces the older job, and stops it if it is running.
- At most 32 queued, the oldest conversation's dropped first.
- At most 10 jobs a minute.

**Cached by turn.**
- The line is stored in the same registered-cache entry as A's excerpt for that turn (AJ-E3708: one entry per conversation, `Trim::Partly`). That entry's first line carries the excerpt's hash, engine, `prompt_version`, `gate_version` and the settings print, so any change makes the line stale and never shown (ADR-0023 point 3).
- A line is written once per turn revision.
- Lines are redacted text, so the entry follows A's: owner-only, and dropped when Last messages goes off.

**Skip, never delay.** When any of these holds, the job ends as `NoLine(Skipped)`, the excerpt stays, and the job is not retried unless the item enters the view later while its turn is still the newest:
- Low Power Mode;
- thermal Serious or Critical;
- memory pressure Warning or worse;
- on battery below 20 % (the Apple tier);
- on battery at all (the local tier, until Q5's measurement sets a rule).

**Per-call limits.** A hard deadline of 15 s per job. The helper is killed at the deadline plus 2 s.

**The budget the build must keep** (AJ-E3711's acceptance; measured on a release build with `docs/qa/evidence/AJ-E805`'s method):

| Measure | Target |
|---|---|
| Idle CPU of the app, feature on, no turn ending | no measurable change against feature off (within the run-to-run noise of the AJ-E805 method); helpers not running |
| App memory added | ≤ 5 MB (queue, client, snapshot); cached lines are on disk |
| Apple helper | ≤ 40 MB resident while alive; exits after 60 s idle |
| Local helper | ≤ 3.5 GB resident while loaded; exits after 120 s idle; never loaded under pressure Warning |
| Latency, line shown after a turn ends | Apple: ≤ 3 s p95 warm, ≤ 6 s cold; local: ≤ 4 s p95 warm on the slowest qualifying Mac (M1, 16 GB), ≤ 10 s with the load. Nothing waits on it: the excerpt shows first |
| CPU in Apple's daemons | ≈ 0.17 s a line (measured) |
| A 50-line burst | idle CPU back within the noise 2 minutes after it ends |
| Battery | the local tier stays on power only until Q5; Apple's tier is measured with Q5's method too |

**AJ-E805's finding is not ours to hide or to be blamed for.** Every measurement is feature off against feature on, on the same build. Tier 2 is not offered to users until AJ-E805's idle finding is fixed or that off-against-on figure is in the evidence.

### 8. Telling the user once, and the Settings switch

**On by default** where Apple's model works (D-120: automatic when the Mac can do it comfortably, with no download). The local file is never on by default: it is offered (§5).

**The notice.** Shown once, when Apple's tier first writes a line, above the feed and below the pinned needs, as a quiet L1 note (D-117: no fill, no dialog), with **Got it** and **Turn off**. Never during first run.

> "**Short summaries, written on this Mac.** Your feed now shows one clear line for each finished turn, written by Apple's built-in AI on this Mac. Nothing you read here leaves it, and Show original is on every line. You can turn this off in Settings › Summaries."

- **The local file's offer card (§5)** is the only other card, also shown at most once.
- **The local tier needs no notice of its own.** The user's own Get it is the moment they're told.
- **Tier 3 changes nothing on screen,** so it shows no notice.

**Settings › Summaries** (a new section, beside Desktop apps):
- One switch: **Write short summaries on this Mac**, with the line "One clear line for each finished turn, written on this Mac. Nothing leaves it."
- One status line under it, from `plan()`'s `Status`:

| Status | Words |
|---|---|
| Apple | "Using Apple's built-in AI." On a qualifying Mac, folded under it: "For clearer lines, get a small AI file (about 2.5 GB) that stays on this Mac." with **Get it** |
| Offered | "A small AI file (about 2.5 GB) can write these on this Mac. It stays here." with **Get it** |
| Getting it | "Getting the AI file: 640 MB of 2.5 GB." with **Cancel**; paused: "Waiting for Wi-Fi and power." |
| Local | "Using a small AI file on this Mac (2.5 GB)." with **Remove**; where Apple's model also works, "Use Apple's built-in AI instead" |
| User server | "Using Ollama, which you run yourself." |
| Can't here | "This Mac can't write them comfortably, so the feed shows the agent's own first lines." |
| Unhealthy | "Paused: the AI on this Mac stopped answering. The feed shows the agent's own first lines." |
| Off | "Off. The feed shows the agent's own first lines." With the file kept: "The small AI file still takes 2.5 GB." with **Remove** |
| Last messages off | the switch is disabled, with "Needs Last messages (Settings › Desktop apps): summaries are written from the agent's own words." |

- Folded under **More**: the user-server line (§4.3), shown only while one is running.
- Every sentence lives in the Settings words module and `feed-words.ts`, and passes the plain-words lint.
- A test fails if any summaries string tells the user they must download something. The offer is a choice, and its words say so.

### 9. Privacy and the trust guide: one test per claim

The build adds one section, "Summaries on this Mac", to [`docs/guide/trust.md`](../guide/trust.md). That page fails the build when a named test is missing, so each claim lands with its test:

| Claim (plain words) | Checked by (new unless noted) |
|---|---|
| Summaries read only an agent's last words, only while Last messages is on, on this Mac | `summaries_read_nothing_while_last_messages_are_off`, `a_summary_is_made_only_from_the_excerpt_and_the_facts` |
| They're written on this Mac: Apple's built-in AI on the device, never Apple's cloud model | `the_apple_helper_never_names_private_cloud_compute` (source); the release workflow's symbol check of the built helper |
| The helpers that write them can't reach the network | `the_summary_helpers_import_no_network_client` (Swift imports and symbols allowlist; the local helper's dependency tree has no HTTP or socket crate); `a_helper_under_a_no_network_sandbox_still_answers` |
| Using Ollama or LM Studio is your choice, and only on this Mac | `the_loopback_client_reaches_only_127_0_0_1` (existing), `a_user_server_is_asked_only_when_turned_on_and_running`, `only_a_loaded_model_is_used_and_nothing_is_pulled_or_loaded` |
| The small AI file is fetched only when you choose Get it, once, from Hugging Face, which sees your IP address, the file's name and the time | `nothing_is_fetched_without_the_users_click`, `the_app_reaches_only_its_own_addresses` (existing, extended by the pin's hosts), `the_fetch_follows_redirects_only_to_the_pinned_hosts`, `the_fetch_sends_no_cookie_or_identifier`, `a_transfer_waits_on_battery_hotspot_low_data_mode_or_low_disk`, `an_8_gb_mac_is_never_offered_the_file` |
| A file that doesn't match its pinned fingerprint is deleted and never used | `a_file_with_the_wrong_hash_is_deleted_and_never_loaded`, `a_part_file_is_never_loaded` |
| Lines are kept on this Mac only, owner-only, in the clearable cache; Last messages off drops them | `summary_lines_are_kept_owner_only_in_the_registered_cache`, `turning_last_messages_off_drops_summary_lines_at_once` |
| Remove deletes the AI file; Cancel deletes a part file | `remove_deletes_the_file_and_cancel_the_part`, `turning_summaries_off_stops_using_the_file_and_offers_remove` |
| A line never goes to a paired Mac, usage statistics, crash reports or logs | `no_summary_text_reaches_fleet_stats_crash_reports_or_logs` |
| A summary never changes what an item says happened | `a_line_never_changes_an_items_state_or_level` |

**STRIDE, briefly** (SECURITY-GUIDELINES C7):

| Threat | Answer |
|---|---|
| Spoofing: a swapped model file | the compiled-in hash |
| Tampering: a helper replaced on disk | the bundle's signature; helpers are spawned only from `Contents/MacOS/` |
| Repudiation | n/a: no user action is taken |
| Information disclosure | no network in the helpers; PCC banned by code; loopback only; no line outside the feed |
| Denial of service: a hung or crashing helper, a huge text | deadlines, kill, unhealthy after 3 crashes, 32 KiB requests, the queue bound |
| Elevation: prompt injection in the agent's text | the input guard and the gate; a line can't act, can't link, can't change state |

### 10. Windows and Linux (D-52)

The seams are OS-neutral now, and the macOS parts sit behind them:

| Seam | macOS (0.5) | Windows (0.6), Linux (0.7): later research |
|---|---|---|
| `platform::ai::system_model()` | the Apple helper | on-device system AI on some new PCs: UNVERIFIED, researched in the Windows release; Linux: none |
| `platform::net::path_cost()` | the Apple helper's `NWPathMonitor` | Windows' connection cost; Linux NetworkManager's metered flag: both UNVERIFIED |
| Power, thermal, pressure, room | `juggler-supervisor` and `juggler_platform::room` (existing) | the existing seams' per-OS readers |
| The local helper | Metal | CPU or Vulkan builds of the same crate and contract |

**Unknown fails closed.** An OS with no reading says `Unknown`, and `plan()` treats it as no: no offer, no transfer, `Extractive`. Windows and Linux compile with stubs that do exactly that, so they ship excerpts until their release adds a tier.

### 11. Build order in 0.5 (decided) and where each part lives

1. **A**, AJ-E3708: excerpts through the adapters, cached per turn. B builds on it and falls back to it.
2. **Tier 1**, AJ-E3711 part 1:
   - the seam (`engine.rs`), `Extractive`, the gate (`gate.rs`) and its fixtures, the queue and cache, `Capabilities` and `plan()` with the offer and transfer fields present but unused;
   - the Apple helper and its build script, signing and bundle check;
   - the notice (Apple variant), Settings › Summaries and the trust section's rows for what ships.
   - The trait-level sketch that AJ-E3710's quality gate asks for compiles on a spike branch first and is not merged.
3. **Tier 2**, AJ-E3711 part 2, only after the selection study fills the slot and Q1 is approved:
   - the measurement plan run (Apple and the study's finalists);
   - the pin, `juggler-local-ai` and its CI job, the offer card, the transfer and its conditions, the remaining status lines and trust rows.
   - It ships to users only once AJ-E805 is resolved or separated (§7).
4. **The user's own server**, optional, last, if 0.5's scope allows. It is small and off by default.
5. **Q5's battery measurement**, once tier 2 exists. It sets the local tier's battery rule (§7).

### 12. Not in scope: talking with the local model (noted for later)

The maintainer floated, for users who got the file, "some conversation with it too": asking questions about their sessions on the Mac. It is **not in 0.5** and needs its own ruling. The design leaves room for it without building any of it:
- **The helper protocol is versioned** (`hello`), so an `ask` message can be added to `juggler-local-ai` without touching `write` or `choose`.
- **It would be a separate seam, not `SummaryEngine`.** That seam is one line per turn under a strict gate, and a conversation is a different contract: free answers, sources to cite, a much larger context.
  - A later `AskEngine` trait would reuse the helper client, the pin, the capability snapshot and the budgets' skip rules.
  - It would read only what the feed's excerpts and AI Juggler's facts already hold, behind Last messages.
- **Whatever answers it gives** would carry the same provenance rule ("written on this Mac", the sources one click away) and never change an item's state.
- **Nothing for it is built now:** no `ask` message, no trait, no UI.

## Alternatives considered

- **Swift linked into the app (`swift-rs`, or an `objc2` shim).** It couples every `cargo build` to a 26+ SDK and the Swift runtime workaround, and risks a launch failure on macOS 14 and 15 unless every symbol is weak. Rejected: a separate helper keeps `cargo` Swift-free and the app launching on 14.
- **An XPC service for the Apple helper.** It has a cleaner lifecycle, but a heavier bundle layout, is harder to test and is macOS-only. A stdio helper is the same contract on every OS.
- **llama.cpp linked into the app.** A C++ crash or leak would take the app down, the model's memory would count as ours, and every build would compile C++. Rejected for the helper.
- **A signed manifest under the updater key.** The pinned public key is empty today, and a new key is a new secret to guard. Rejected by the coordinating session: the hash compiled into the notarized app gives the same guarantee with no key.
- **Hosting the file on GitHub releases now.** The 2 GiB cap means splitting a 4B file. Later, as a mirror behind the same hash.
- **Bundling the model in the app.** Every download would be 2.5 GB larger, including for Macs that use Apple's model. Rejected.
- **Trying every engine on a hard turn.** It multiplies the cost of exactly the turns that fail. One model attempt, then `choose`, then the excerpt.
- **Letting the model rewrite questions and failures.** The loudest items would rest on the least certain words. Rejected: L2 and L3 keep the agent's own words and AI Juggler's facts.
- **Polling for Apple Intelligence or the network.** Rejected: turn ends, machine-change events and `NWPathMonitor` cover every change that matters.
- **Requiring, or defaulting to, the user's Ollama.** Against "never required" and against spending the user's loaded model without a choice. Optional and off.
- **Fetching the local file automatically in the background** (D-120's first wording). Overruled by the maintainer: a 2 GB download needs the user's approval. Offered, with one click.
- **Offering the file in Home on Macs where Apple's model works.** That would be a nudge to spend 2.5 GB for a smaller gain. Offered in Settings only.
- **Offering on `NotReady`.** Apple is preparing its own model, so wait instead.
- **Community Rust wrappers for Foundation Models** (`fm-rs` and others). Their maturity is unverified, and they mean a new dependency. The 66 KB helper is ours to audit.

## Consequences

- **Easier:**
  - A new engine (a later cloud one, Windows' system AI) is one `SummaryEngine` impl, plus a `plan()` rule and a status sentence. The feed, the gate, the queue and the cache don't change.
  - The faithfulness rules are one module with one fixture set, for every engine.
  - The app's own process gains no model, no Swift and no C++.
- **Harder:**
  - Two more signed binaries in the bundle and in the release workflow, one with its own Swift build and one with a C++ build in its own CI job.
  - A pinned model means an app release to change the model, which is intended: the hash rides on the app's signature.
  - Hugging Face's host names join the app's address list. A firewall that blocks them leaves the user on excerpts, silently, which is acceptable.
- **Accepted:**
  - The gate rejects some true lines (the research's upper bound is 12 to 22 % on commit subjects); each costs only a fall back to the excerpt.
  - Telegraphic lines from 3 to 4B models are judged against the excerpt, not against a cloud model.
  - Getting the file tells Hugging Face an IP address, a file name and a time, which the Privacy page says.
  - Many Macs without Apple Intelligence will stay on excerpts until the user chooses the offer. That is the maintainer's intent.
- **To verify at the spike** (design facts, not the maintainer's questions):
  - Foundation Models and `NWPathMonitor` inside an App Sandbox helper with no network entitlement;
  - Metal and the model read in the local helper's sandbox;
  - Hugging Face's redirect hosts and `Range` on the redirected URL;
  - Ollama's and LM Studio's loaded-model routes, and LM Studio's terms.

## Open questions for the maintainer

- **Q1: the development download for the measurement plan.**
  - What: the selection study's finalist files (2 to 2.5 GB each) and the `llama-cpp-2` crate with its build tools, once, in a scratch folder, to run the research's 20-text plan and fill the model slot on evidence.
  - Status: the coordinating session is asking it now. Nothing is fetched until it's approved and the study has named the finalists.
  - What waits on it: tier 2. Tier 1 and A need no download.
- **Q5: the battery measurement.** It needs a MacBook on battery and a hands-on session, and waits until the local tier exists. Until then the local tier runs only on power.

Nothing else here needs a ruling. Hosting, 8 GB Macs, the build order and "offered, never fetched without a click" are decided (Context). The model itself comes from the coordinating session's study. The rest are design calls made above.
