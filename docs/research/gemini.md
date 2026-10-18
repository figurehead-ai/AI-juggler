# Research notes: Google Gemini CLI (and its Antigravity successor)

> Compiled 2026-10-16 as the AJ-E605 baseline under the billing-source
> protocol (AJ-E601, `billing-source-protocol.md`). Legend: **[V]** =
> verified fact with dated source · **[I]** = inference · **[U]** =
> unknown / could not verify. **No Gemini install exists on the
> maintainer machine** (`which gemini` empty, no `~/.gemini`) — every
> live-capture item below is therefore marked `unverified — no
> install`, which the AJ-E605 budget defines as a PASSING outcome for
> this packet. All grades here are **doc-based** (protocol §4 accepts
> dated official docs when no capture exists).
>
> **The headline landscape fact (verified):** Google sunset **Gemini
> CLI for individual users on 2026-06-18** — free, Google AI Pro and
> AI Ultra logins stopped being served — and moved the product to the
> closed-source, cloud-only **Antigravity CLI**
> (announcement 2026-05-19, https://developers.googleblog.com/an-important-update-transitioning-gemini-cli-to-antigravity-cli;
> maintainers' discussion https://github.com/google-gemini/gemini-cli/discussions/28017;
> banner on https://geminicli.com/docs/resources/quota-and-pricing,
> checked 2026-10-16). The open-source CLI still runs for **API-key
> and Vertex AI** paths. Any Juggler "Gemini account" in 2026-10 is
> therefore either (a) the legacy CLI on an API key, or (b) the
> Antigravity CLI on a subscription — §7 covers the successor.

## 1. Distribution & versioning

- **[V] Install:** `npm install -g @google/gemini-cli` (Apache-2.0,
  https://github.com/google-gemini/gemini-cli). Current published
  version **0.64.0** (registry.npmjs.org/@google/gemini-cli/latest,
  checked 2026-10-16). Three release channels: nightly / preview /
  stable (https://github.com/google-gemini/gemini-cli/blob/main/docs/releases.md).
- **[V] Official docs hubs:** https://google-gemini.github.io/gemini-cli/docs
  and https://geminicli.com/docs (the latter carries the sunset
  banner); Google Cloud variant at
  https://docs.cloud.google.com/gemini/docs/codeassist/gemini-cli.
- **[V] Sunset timeline:** announced 2026-05-19 (developers blog,
  above); individual-user requests ceased **2026-06-18** (GitHub
  discussion #28017); legacy wind-down completed ~2026-07-17.
- **[I]** The npm package and GitHub repo remain live for API-key /
  Vertex users; nothing observed suggests the package is pulled.

## 2. Configuration & local state

- **[V] Config root:** `~/.gemini/` — `settings.json` (user),
  `.gemini/settings.json` (project), plus system-level paths
  (`/etc/gemini-cli/settings.json`, macOS
  `/Library/Application Support/GeminiCli/settings.json`);
  `GEMINI_CLI_HOME` relocates the user root
  (https://geminicli.com/docs/reference/configuration, last updated
  2026-10-07).
- **[V] Context files:** hierarchical `GEMINI.md` — global
  `~/.gemini/GEMINI.md`, project root, ancestors, subdirectories;
  discovery capped at 200 dirs (`context.discoveryMaxDirs`).
- **[V] Shell history:** project-scoped at
  `~/.gemini/tmp/<project_hash>/shell_history` (same doc).
- **[V/U] Session/chat files:** chat sessions are checkpointed under
  `~/.gemini/tmp/<hash>/` and retained per `sessionRetention`
  (default 30 d); `general.checkpointing.enabled` (default **false**)
  gates recovery checkpoints. Exact chat-history file names/shape are
  community-documented, not spec-documented — `unverified — no
  install` locally (dev.to "Automatic Session Saving"; config doc).
- **[V] Telemetry:** OTLP telemetry with a **local** target
  (`telemetry.target: "local"`, `outfile`, `logPrompts`) — opt-in,
  carries tool names/durations/model, not prompt content
  (configuration doc, 2026-10-07). Whether events carry token counts
  is not documented — **[U]**; a sanctioned local usage-events feed
  only if a capture proves the fields (protocol §2).

## 3. Auth & plans (post-sunset)

- **[V]** Individual Google-login tiers (free Code Assist for
  individuals / AI Pro / AI Ultra) **no longer serve the legacy CLI**
  (sunset, §1). "Tiers not listed, including Google AI Plus, are not
  supported" (quota-and-pricing doc).
- **[V] Remaining auth paths** (quota-and-pricing doc, checked
  2026-10-16):
  | Auth | Tier | Daily request cap |
  |---|---|---|
  | Gemini API key | Free tier | 250 (Flash-only) |
  | Gemini API key | Pay-as-you-go | per-token billing, no fixed cap |
  | Vertex AI | Express mode | varies (90 d before billing required) |
  | Vertex AI | Pay-as-you-go | dynamic shared quota / provisioned throughput |
  | Workspace | Code Assist Standard | 1,500 |
  | Workspace | Code Assist Enterprise | 2,000 |
  (Historical individual caps, pre-sunset: free 1,000 / Pro 1,500 /
  Ultra 2,000 — same doc; retained for interpreting old captures.)
- **[V] Limit dimensions (API path):** RPM / input TPM / RPD, "per
  project, not per API key"; **RPD resets at midnight Pacific**;
  spend-based rolling 10-minute caps by usage tier ($10/$50/$200)
  returning `429 RESOURCE_EXHAUSTED`
  (https://ai.google.dev/gemini-api/docs/rate-limits, updated
  2026-10-15). This midnight-PT daily reset is a **documented reset
  rule** — the window clock (AJ-E606) can carry it as a
  documented-sanctioned reset boundary for API-key accounts; the
  subscription clock stays separate regardless (D-12).

## 4. Usage & quota readouts

- **[V] `/stats` slash command:** "Show model-specific usage
  statistics, including token counts and quota information"; the
  quota doc adds it "provides a snapshot of your current session's
  token usage" plus limit info, and "a summary of model usage is also
  presented on exit at the end of a session"
  (https://geminicli.com/docs/reference/commands, 2026-08-17;
  https://geminicli.com/docs/resources/quota-and-pricing). Grade:
  **documented-sanctioned, session-scoped** — it is NOT an account
  pool feed. Live shape: `unverified — no install`.
- **[V] Context-window %:** the UI footer shows context usage
  (`ui.footer.hideContextPercentage`, default hides it) — context
  window, not plan quota (configuration doc, 2026-10-07).
- **[V] No 5h/weekly window for the legacy CLI:** the quota doc
  describes **per-user-per-day request caps only**; the 5-hour +
  weekly model belongs to the Gemini **app** (Google support
  16275805) and to **Antigravity** (§7), never to the legacy CLI.
  Do not port Claude/Codex window shapes onto it (pass2/09 U-4).
- **[V] Web dashboards (auth-required, not local):** AI Studio
  "View your active rate limits" (https://aistudio.google.com/rate-limit)
  and Cloud Console quotas; **[V/U] no documented API, response
  header, or local file exposes remaining quota/reset times**
  (rate-limits doc documents none; absence stated there).
- **[U]** Exhaustion behavior locally (`429` surfaces in stderr /
  exit codes): documented only for the API path generally;
  CLI-specific exit-code contract unverified — no install.

## 5. Renewal & billing dates (the D-12 question)

**Explicit answer: NO sanctioned subscription renewal/billing-date
source exists for Google consumer plans.** Negative-result search
log (≥4 surfaces, 2026-10-16, mirroring AJ-E602 Req.2):

1. `developers.google.com` API index — no Google One / consumer
   subscription query API; Gemini API billing docs cover prepaid
   credits only (https://ai.google.dev/gemini-api/docs/billing,
   2026-09-28).
2. Google Play Developer API `purchases.subscriptionsv2` returns
   `expiryTime` — **only for subscriptions of apps YOU publish**
   (https://developers.google.com/android-publisher/api-ref/rest/v3/purchases.subscriptionsv2);
   unusable for Google One, prohibited scope-creep.
3. End-user surfaces are web-only: `myaccount.google.com/subscriptions`
   and Play Store → Payments & subscriptions — manual-read, no API,
   no local file.
4. `one.google.com` / `gemini.google/subscriptions` marketing pages —
   plan feature tables, no renewal-date feed.
5. Gemini CLI config/state surface (`~/.gemini/**` documented keys) —
   no billing/renewal field documented.

- **[V] Prepaid API credits expire 12 months after purchase**
  (billing doc above) — an expiry date the user knows from the
  purchase receipt only; not queryable. Same D-12 verdict class as
  Claude/Codex: **user-entered date or unknown; never inferred.**

## 6. Confidence ceiling per field (what AJ-E607 may implement)

| Field | Ceiling | Fallback AJ-E607 must implement |
|---|---|---|
| Session token usage (per chat) | `documented-sanctioned` shape (`/stats`, exit summary) — live grade `unverified — no install` | none until an install captures the file/command shape |
| Account quota % | **none** (no feed; daily request caps are counts, and only for auth paths that still serve) | render unavailable-with-reason (D-06) |
| Absolute burn (API path) | Computed from billing after the fact — **[U]** no local event source verified | unknown |
| Reset rule (API-key path) | `documented-sanctioned`: RPD resets midnight Pacific | window clock only; never conflated with renewal |
| Renewal/expiry | `unknown` (§5) — user-entered per D-12 | manual date entry (AJ-E610), `Unknown` otherwise |
| Plan tier | user-selected at account setup (Juggler-side fact, not vendor-verified) | display as entered |

**Rule for E611 ranking:** a Gemini account's headroom is `unknown` —
never zero, never full — until a source verifies.

## 7. Successor: Antigravity CLI (a section, not a fork — handover note)

- **[V]** Antigravity CLI is the subscription-path successor
  (closed-source, cloud-only). Docs hub: https://antigravity.google/docs.
- **[V] Documented quota command:** `/usage` (alias `/quota`)
  "refreshes your model quota display"
  (https://antigravity.google/docs/cli/commands/usage) — a
  documented-sanctioned **readout candidate**, live grade unknown.
- **[V] Plans model:** rate limits scale by plan; Ultra quotas
  "refreshed every five hours" subject to weekly caps
  (https://antigravity.google/docs/plans; plan-change post
  https://antigravity.google/blog/changes-to-antigravity-plans,
  2026-05). Community reports a 5-hour refresh + weekly ceiling for
  Pro (https://discuss.ai.google.dev/t/navigating-antigravity-pro-quota-limits/130212) —
  window shape parity with Claude/Codex **[I]**, capture needed.
- **[U]** Community tool `skainguyen1412/antigravity-usage` reads
  quota from **local IDE state offline** — evidence that a local
  quota file exists (grade `local-observable`?) but the path/shape
  is community-discovered, not documented: prohibited until
  documented or captured-and-graded per protocol §6.
- **When Juggler adds an Antigravity adapter:** investigate here
  (add §8+), same protocol, new task ID; do not reuse Gemini grades.

## 8. Consequences recorded downstream

- **AJ-E607:** emit no Gemini source adapter beyond a stub that
  renders unavailable-with-reason; nothing in this packet verifies a
  local usage-events file for the legacy CLI.
- **AJ-E610:** Gemini accounts show `No verified quota source for
  this Provider yet` (D-06) + manual renewal entry (D-12).
- **AJ-E611:** ranking treats Gemini headroom as unknown.
- **Landscape docs:** `landscape-and-stack.md:12` (cc-switch Gemini
  support) predates the sunset — account switching for the legacy
  CLI is moot for individuals; revisit when Antigravity lands.
