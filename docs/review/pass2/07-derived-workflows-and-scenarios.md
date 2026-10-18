# Pass 2 §07 — Derived Workflows & Scenarios Beyond the Example

> Parent: [Maximum-Depth Report](../AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L1) ·
> Canonical example: [expected-worfklow.md](../../expected-worfklow.md#L1) ·
> Prior personas: [07-personas-and-journeys.md](../deep/07-personas-and-journeys.md#L1) ·
> Prior feature PRDs: [10-product-proposal-missing-features.md](../deep/10-product-proposal-missing-features.md#L1)
> Task: second pass — what prior research under-weighted, got wrong,
> or missed versus the maintainer's example. This file does not restate
> the prior report; each workflow cites it, then extends it.

## 0. Method and reading rules

The example workflow is one happy path: start on one tool, hit a quota wall, switch tools/accounts "as
many times as needed," with history, rules, and context intact
([expected-worfklow.md:5](../../expected-worfklow.md#L5),
[expected-worfklow.md:7](../../expected-worfklow.md#L7)). Prior research optimised around that noon wall:
forecast (10 §1), inbox (10 §2), honest handover (UX.5, full review F3), reset-return (SPEC §7.2,
[SPEC.md:288](../../SPEC.md#L288)). All eleven workflows below start where that model ends — at night, at
scale, across projects, across people, or when a whole Provider is down.

Marks in this file: `[repo]` = code/docs-read fact with `file:line`; `[prior §]` = cited prior-report
section; `[inferred]` = reasoned extension needing dogfood validation. Time-saved figures are
`[inferred]` estimates against the manual workaround named in each workflow, not measured values; §13
states how to falsify them.

Automation rule applied to every workflow, verbatim: the app *prepares*, the user *executes*; automation
is passive only — monitoring, indexing, notifications, suggestions, staging
([SPEC.md:26](../../SPEC.md#L26)); "the app never auto-rotates accounts"
([PROPOSAL.md:62](../../PROPOSAL.md#L62)); "preparation and staging can be automated; execution is always a
user click" ([PROPOSAL.md:63](../../PROPOSAL.md#L63)). Every workflow therefore splits **Auto (passive)**
from **Click-confirmed**. Anything that would auto-launch, auto-resume on a different Account, auto-kill,
or auto-send is listed as click-confirmed or as an anti-feature.

## 1. Terminology — adopt Provider / Account / Instance

The maintainer names the stack explicitly: "we should call them providers, accounts, and instances of the
tool" ([expected-worfklow.md:11](../../expected-worfklow.md#L11)). The codebase does not yet speak that
language. Mapping and conflicts:

| Maintainer term | Repo term today | Repo cite | Conflict |
|---|---|---|---|
| Provider | Harness / harness adapter | SPEC §4, [SPEC.md:91](../../SPEC.md#L91) | Rename in UI copy; keep `HarnessAdapter` in code |
| Account | Account (state dir + login) | [SPEC.md:93](../../SPEC.md#L93) | None — already correct |
| Instance (tool instance) | Instance (ADR-0007) vs Session | [adr/0007:46](../../adr/0007-instance-centric-information-architecture.md#L46) | UI conflates the two (F2.2) |
| Profile | Profile = Account + settings overlay | [SPEC.md:101](../../SPEC.md#L101) | No maintainer word; hide behind Instance launch |
| Session | SessionRecord (conversation) | [SPEC.md:108](../../SPEC.md#L108) | Keep as the conversation inside an Instance |

Three naming conflicts to rule on before any workflow ships:

1. **Provider vs Harness.** SPEC and all adapters say "harness"; the maintainer and the user-facing copy
say "Provider" ([expected-worfklow.md:13](../../expected-worfklow.md#L13): "which provider it is").
Recommendation: UI and docs say Provider; `HarnessAdapter` stays as the code-level trait name. Do not
rename crates for a copy change. 2. **Instance vs Session.** ADR-0007 already defines Instance as "one
running/tracked occurrence of a harness under an account" and Session as "the work conversation living
inside an instance" (adr/0007 decision 1). The built UI renders Instances as filtered Sessions (full
review F2.2, [review:171](../../review/full-competitive-ux-review.md#L171)). Every workflow below uses
Instance = the running tool process (with CPU/RAM/children) and Session = the resumable conversation. 3.
**Profile has no user word.** A Profile is launch configuration, not a thing users "juggle." Workflows
below say "launch an Instance on Account B" and treat Profile as an advanced detail inside the launch
sheet, never as a rail noun.

The example also demands per-Session indicators — Provider, Account, model, quota left
([expected-worfklow.md:13](../../expected-worfklow.md#L13)) — and, on selection, a right-hand panel with
usage, history, quality, performance, CPU/RAM, and spawned sub-processes
([expected-worfklow.md:14](../../expected-worfklow.md#L14),
[expected-worfklow.md:15](../../expected-worfklow.md#L15)). Workflows assume that panel exists; §12 lists
where prior work under-built it.

## 2. What the prior pass under-weighted, got wrong, or missed

Cited to the prior reports, extended here — not restated:

1. **Time-of-day blindness.** Prior journeys are daytime and single-sitting (07 §9 journey maps; 10 §28
three flows at 08:00, 13:00, Friday-afternoon). Overnight, offline, and end-of-week cadences have no
journey. → W1, W8, W10 below. 2. **The wall is treated as an event, not a schedule.** 10 §9 (reset queue)
is the strongest prior answer, but it queues one Session. Nobody modelled *parking many* Sessions and
triaging them at reset. → W2, W3. 3. **Handover is modelled as same-task, same-person, same-day.**
Pair-review (W5), cheap→strong model arbitrage (W6), and cross-person handoff (W9) reuse the T1–T4 tiers
([SPEC.md:269](../../SPEC.md#L269)) for different jobs the prior report never named. Prior handover trust
work (07 §7.2) covers fidelity labelling, not role-switching. 4. **One Project at a time.** 10 §15 groups
projects, but the consultant *day* — five clients, context-switch cost, billing boundaries — is a
workflow, not a grouping feature. → W4. 5. **Failure is per-Session; outage is per-Provider.** Crash
recovery is Session-scoped (SPEC §5.3, [SPEC.md:250](../../SPEC.md#L250)); zombie/orphan cleanup gets one
line ([SPEC.md:224](../../SPEC.md#L224)) against a full maintainer paragraph
([expected-worfklow.md:17](../../expected-worfklow.md#L17),
[expected-worfklow.md:18](../../expected-worfklow.md#L18)). A whole lab being down (W11) and incident
forensics (W7) are missing. 6. **"Team" was correctly scoped out, then over-applied.** Non-goal: "sharing
profiles or pools between people" ([PROPOSAL.md:45](../../PROPOSAL.md#L45)). Prior work lets that veto even a
one-way, user-exported pack to a colleague. Export/import packs already exist (SPEC §7.2 flow 5,
[SPEC.md:290](../../SPEC.md#L290)) — W9 keeps the guardrail and still ships the handoff.

## 3. Workflow catalogue at a glance

| # | Workflow | Persona anchor | Prior home | Status |
|---|---|---|---|---|
| W1 | Overnight / unattended runs | Devon (07 §2) | 10 §7 digest only | New |
| W2 | Quota-reset queue: park & resume | Maya (07 §1) | 10 §9 (single) | Extended |
| W3 | Morning triage of 15 Sessions | Devon (07 §2) | 10 §2 + §28.2 | Extended |
| W4 | Multi-project consultant day | Priya (07 §3) | 10 §15/§6 | Extended |
| W5 | Pair-switching for review | Devon + Maya | 10 §11 (gated) | New framing |
| W6 | Model arbitrage: cheap drafts, strong finishes | Alex (07 §6) | 10 §3 (account only) | New |
| W7 | Incident / debug workflow | Devon, Jonas (07 §4) | SPEC §5.3 partial | New |
| W8 | Offline / travel | Priya, Alex | `[NEG-EV]` none found | New |
| W9 | Team handoff: export pack to colleague | Priya, Sara (07 §5) | SPEC §7.2, gated by non-goal | Reframed |
| W10 | End-of-week cost review | Maya, Priya, Jonas | 10 §6 (feature, not ritual) | Extended |
| W11 | Provider outage day: failover whole Project | All | `[NEG-EV]` none found | New |

Each workflow follows the same shape: Trigger → Steps in Juggler (exact clicks, in maintainer language) →
Auto vs Click-confirmed → Failure branch → Delight moment → Features required (proposal IDs) → Journey
emotion + time saved `[inferred]`.

## W1. Overnight / unattended runs `[inferred]`

**Trigger.** 22:00. Devon (07 §2: 10–20 Sessions/day) leaves four Instances running refactors and tests;
Maya leaves one long migration. Today those Sessions either finish unseen, stall on a permission prompt
at 01:14, or die with the laptop lid (07 diary: "waited seven hours," 07 §2).

**Steps in Juggler.** (1) Instance panel → [Run overnight…] opens a pre-flight sheet:
Provider/Account/model chip, quota forecast pill ("A exhausts ~01:40 — park at wall or hand to B?"),
admission verdict (SPEC §5.2, [SPEC.md:237](../../SPEC.md#L237)), battery/sleep warning. (2) User ticks
"Notify only if waiting > 15 min" and clicks [Start overnight]. (3) Overnight, the board keeps running;
the tray shows "4 overnight · 1 needs you." (4) 07:30, user clicks the Inbox row "Overnight digest (4)" →
per-Session outcome cards: Finished / WaitingUser / QuotaBlocked / Crashed, each with last-output excerpt
and token burn. (5) [Resume] / [Hand over…] per card.

**Auto (passive).** Status emission and stuck heuristics (SPEC §5.1, [SPEC.md:220](../../SPEC.md#L220));
quota accounting and forecast; digest assembly; crash classification ([SPEC.md:250](../../SPEC.md#L250));
re-adoption after app restart ([SPEC.md:251](../../SPEC.md#L251)). **Click-confirmed.** Starting the
overnight run; any handover at the wall; any resume after a crash; dismissing a WaitingUser prompt (never
auto-answer permissions — anti-feature 7 in 10 §22).

**Failure branch.** Laptop sleeps or reboots: on wake, Sessions that died are surfaced with the same
recovery options (SPEC §5.3, [SPEC.md:252](../../SPEC.md#L252)); the digest says "machine slept 02:10–06:40;
2 Sessions lost, both resumable" instead of pretending they ran. If quota exhausts at 01:40 with no
click, the Session parks as QuotaBlocked and enters W2's queue — it never auto-switches Accounts
([PROPOSAL.md:62](../../PROPOSAL.md#L62)).

**Delight moment.** The 01:14 waiter is in the digest with "asked at 01:14, waited 6h 12m" — the exact
pain in Devon's diary becomes a number the tool owns. **Features required.** 10 §2 inbox, §7
notifications (digest mode), §1 forecast, §9 reset queue; SPEC §5.1 status emission is the hard
dependency (currently modelled, never emitted — full review F2.5). **Emotion + time saved.** Anxiety →
calm handover to the morning. `[inferred]` 20–40 min/day of tab-checking and re-prompting avoided;
falsify with 07 §11 diaries.

## W2. Quota-reset queue — park & auto-suggest resume `[inferred]`

**Trigger.** Maya hits the wall at 13:00 (07 §1; 10 §28.1). Two Sessions are mid-task; Account A resets
17:00; Account B has 20% left — too little to hand everything over.

**Steps in Juggler.** (1) Inbox row "QuotaBlocked · billing-migration · Account A" → [Park until reset].
(2) Park sheet: reset countdown (from the statusline `resets_at` feed, SPEC §4.1
[SPEC.md:192](../../SPEC.md#L192)), target choice — "Return to A at reset" (default, SPEC §7.2 flow 3
[SPEC.md:288](../../SPEC.md#L288)) or "Continue here on B." (3) Click [Park session]. Repeat for the second
Session; the Inbox gains a "Parked (2) · reset 17:00" group with a FIFO order the user can drag. (4) At
17:00, one notification: "Account A reset — 2 parked Sessions ready." (5) User clicks [Resume] on each,
in order; each resume is primed with the "while you were away" delta ([SPEC.md:288](../../SPEC.md#L288)).

**Auto (passive).** Countdown timers, reset detection, the single reset notification, delta-pack staging,
queue ordering display. **Click-confirmed.** Parking, reordering if it changes which Session resumes
first on a shared window, and *every* resume — the queue "never launches unclicked" is already the
acceptance test in 10 §9. Extends 10 §9 from one Session to an ordered, multi-Session queue with a
shared-window warning ("resuming both at once re-exhausts A in ~50 min — stagger? [Stagger 20 min]
[Resume both]").

**Failure branch.** Reset time is Estimated (Codex, SPEC §4.2 [SPEC.md:203](../../SPEC.md#L203)) and slips:
the queue shows "reset estimate stale — last checked 22m" and holds the Sessions parked rather than
resuming into a still-blocked Account. If the lineage broke (transcript moved), the row flips to `stale`
with an explanation (10 §9 controls), never a silent drop.

**Delight moment.** At 17:00 the user does two clicks, not a re-explanation of two tasks. **Features
required.** 10 §9 (extended to multi-Session), §1 forecast, §2 inbox grouping, §7 single interrupt at
reset. **Emotion + time saved.** Panic at the wall → "it is parked, it will ping me once." `[inferred]`
saves the 4-hour babysitting loop and ~15 min of re-prompting per parked Session.

## W3. Morning triage of 15 Sessions `[inferred]`

**Trigger.** 08:00. Devon opens the laptop to 15 Sessions from yesterday and overnight: 3 finished, 2
waiting on him, 1 stuck, 1 quota-blocked, the rest still running or stale (07 §2 day-in-life; rail caps
at 15/8 with no search today — full review F2.5).

**Steps in Juggler.** (1) App opens on the Inbox (10 §2: rail first item `Inbox (n)`), sorted by
waiting-time, oldest first. (2) `j`/`k` walks rows; each row shows Project · Account · Provider · model ·
why-chip · waiting timer · per-Session burn and footprint. (3) Oldest waiter: `Enter` → Instance panel
with the permission prompt focused → user answers in the terminal → row clears, badge decrements. (4)
Stuck? row: [View output] shows CPU/silence/mtime evidence (SPEC §5.1 [SPEC.md:220](../../SPEC.md#L220)) →
[Resume] or [Hand over…]. (5) Finished rows: `x x x` selects three → [Approve → archive] (writes audit
rows, 10 §17). (6) `⌘K` → "stripe" → jumps to the one Session he actually needs.

**Auto (passive).** Waiting-time ordering, status classification, burn/footprint rollups, badge counts.
**Click-confirmed.** Every answer, resume, handover, and archive; snooze/mute per Session. Nothing is
auto-archived (retention archives only, never deletes — 10 §5, [SPEC.md:233](../../SPEC.md#L233) growth
noted).

**Failure branch.** A status is a false Stuck? (model thinking, not hung): the evidence pane is the
escape hatch — the user sees "CPU active, transcript fresh 40s ago" and the chip downgrades itself. If
the watcher paused, the Inbox shows "Watcher paused — Resume" (10 §32) instead of an empty, lying list.

**Delight moment.** Seven blockers cleared keyboard-only in under four minutes (10 §28.2 target).
**Features required.** 10 §2 inbox, §10 palette, §18 keyboard, §5 archive; §11 review rows feed the same
queue. Prior 10 §28.2 scripts this morning but assumes the statuses exist; the extension here is
bulk-select + archive and the false-Stuck evidence pane. **Emotion + time saved.** Dread at 15 tabs →
tower control. `[inferred]` 25–45 min/morning versus clicking through terminals; measure with the §28.2
script.

## W4. Multi-project consultant day `[inferred]`

**Trigger.** Priya (07 §3): four clients, standups at 08:45, a screen-share at 11:00, an invoice Friday.
The risks are not quota — they are wrong-Account billing (Claude #87447,
[PROPOSAL.md:22](../../PROPOSAL.md#L22) via research) and another client's name on screen.

**Steps in Juggler.** (1) Morning: Usage view filtered by client Group chip "Client B" (10 §15) → checks
yesterday's burn before standup. (2) Launch: NewSession on Client B's Project — the sheet shows
Provider/Account/model and warns "usual Account for this Project is Work Max; you picked Personal Max" →
[Use usual] / [Launch anyway] (10 §8 affinity). (3) 11:00 call: `⌘⇧P` → Present mode "Client B" — rail,
Inbox, and notifications show only Client B; a persistent banner reads "Presenting: Client B." (4)
Mid-day handover inside Client B: Session panel → [Hand over…] → wizard step 2 is the redaction preview
(07 §3: P0 for Priya, full review F1.6) → [Confirm handover]. (5) 17:30: [Export per-client pack] → CSV +
audit JSON for the invoice.

**Auto (passive).** Affinity computation (plurality of last 20 launches), per-client quota attribution,
Present-mode filtering (pure UI filter, 10 §15). **Click-confirmed.** Overriding the affinity warning,
every handover, every export; Present mode is a click and never auto-engages on a calendar event.

**Failure branch.** The affinity warning is wrong (a deliberate one-off): one click confirms and the
choice is remembered per Project+Account pair, so it does not nag twice. If a pack contains a likely
secret, the preview blocks [Confirm] until the user redacts or explicitly overrides — the override is
audit-logged (10 §17), because for Priya the log is the product.

**Delight moment.** The client asks "what did the AI do Tuesday?" and Priya exports the answer instead of
reconstructing it. **Features required.** 10 §15 grouping + Present mode, §8 account health/affinity, §6
cost ledger, §17 audit log, §14 MCP/settings sync for cloning a client setup. Extends 10 §28.3 (Friday
invoice) into the full day. **Emotion + time saved.** Low-grade dread → audit-ready calm. `[inferred]`
~45 min/week of spreadsheet allocation (07 §3) plus unquantifiable leakage-risk removal.

## W5. Pair-switching for review — author on A, review on B

**Trigger.** Maya authors a feature on Account A (her fast model); she wants a *different* Account —
ideally a different Provider — to review it, because same-model self-review misses its own mistakes
`[inferred]`. Prior 10 §11 gates a review queue on dogfood; this workflow is narrower: it never asks
Juggler to review, only to switch chairs.

**Steps in Juggler.** (1) Author Session finishes → Inbox row "Ready for review." (2) Click [Review on
another Account…] → target sheet ranked by Provider diversity first ("different Provider: fresh eyes"),
then headroom (10 §3 score, re-weighted). (3) User picks Account B / Provider 2 → wizard shows fidelity
tier (SPEC §7.1 [SPEC.md:269](../../SPEC.md#L269): T2 same-Provider, T3 cross-Provider replay) and the
carry-list in plain words. (4) [Start review Session] launches an Instance on B primed with the full
conversation plus the diff; the author Session stays resumable, untouched (T2 copies, never moves —
[SPEC.md:287](../../SPEC.md#L287)). (5) Reviewer Session leaves comments in its own terminal; user clicks
[Send comments to author Session] → staged as a primed resume on A, executed on click.

**Auto (passive).** Target ranking, diff computation (`git diff <base>...HEAD`, 10 §11), pack staging,
lineage linking. **Click-confirmed.** Starting the review Instance, sending comments back, any follow-up
fix launch. Juggler never auto-comments and never auto-merges — the review opinion belongs to the user.

**Failure branch.** No second Provider configured: the sheet says so and offers same-Account review with
an honest "same model family" label instead of faking independence. If the diff is empty, the row says
"no changes" (10 §11 controls) rather than launching a review of nothing.

**Delight moment.** "Authored by A, reviewed by B" appears as a lineage pair in the Project timeline —
the two chairs are visible. **Features required.** 10 §11 (read-only diff v1), §3 routing (re-weighted
for diversity), §17 lineage/audit; SPEC T2/T3 tiers. **Emotion + time saved.** Self-review unease → a
real second pair of eyes. `[inferred]` 15–30 min versus manual copy-the-diff into a second terminal, per
review; demand is `[inferred]` — keep behind the 10 §11 dogfood gate.

## W6. Model arbitrage — cheap model drafts, strong model finishes

**Trigger.** Alex (07 §6) and Jonas (07 §4) run small tanks: GLM Lite credits and Claude Pro, not Max.
Scaffolding, boilerplate, and test-writing do not need the strongest model; the final fix does. Prior 10
§3 ranks *Accounts* by headroom; it never ranks *models within* a task. `[inferred]` — no prior section
covers this split.

**Steps in Juggler.** (1) NewSession with template "Cheap grunt pass" (10 §16) → Provider GLM / budget
model on Account Lite → [Launch]. The Instance panel header shows Provider · Account · model · burn rate,
per the maintainer's indicator demand ([expected-worfklow.md:13](../../expected-worfklow.md#L13)). (2)
Draft lands; Inbox row "Draft ready." (3) Click [Finish with strong model…] → sheet: target
Provider/Account/model, fidelity tier (T3 cross-Provider replay [SPEC.md:273](../../SPEC.md#L273) or T2), and
the estimated burn of *both* legs shown side by side. (4) [Hand over to strong model] → new Instance
primed with the full draft conversation; user deletes the scaffolding it got wrong. (5) Usage view shows
the split: "draft 82% on Lite ≈ cheap; finish 18% on Max" in sessions-remaining terms (07 §6: "≈N
sessions left," not raw credits).

**Auto (passive).** Burn attribution per leg, headroom ranking, "≈sessions left" translation, cost
estimate labelled Estimated (10 §6 label discipline). **Click-confirmed.** The handover itself and the
choice of strong target; Juggler may *suggest* the cheap leg at launch ("this looks like scaffolding —
start cheap?") but never downgrades a running Session's model.

**Failure branch.** The cheap draft is bad enough that replay wastes more strong-model tokens than it
saved: the sheet warns when the draft conversation exceeds a size threshold ("long draft — replay cost ≈
drafting fresh") and offers [Start fresh on strong model] as an equal first-class choice, not a hidden
fallback.

**Delight moment.** The Usage split makes the saving legible: "this feature cost one-sixth of an all-Max
run." **Features required.** 10 §16 templates, §3 routing (model-aware), §1 forecast, §6 cost ledger with
per-leg split; SPEC T2/T3. **Emotion + time saved.** Small-tank guilt → deliberate spending. `[inferred]`
the saving is quota/cost, not minutes: ~40–70% of a feature's burn moved to the cheap tier on
scaffolding-heavy work; validate by pricing two dogfood features both ways (10 §26 kill-tests).

## W7. Incident / debug workflow `[inferred]`

**Trigger.** Production is broken; or, locally, an Instance crashed and left children behind. The
maintainer's zombie paragraph is the seed: tools "crash, exit, and their subprocesses… end up running
long after the session closed… zombie processes" with "automatic and manual clean ups, all with
verifications" ([expected-worfklow.md:17](../../expected-worfklow.md#L17),
[expected-worfklow.md:18](../../expected-worfklow.md#L18)). Prior SPEC gives this one line
([SPEC.md:224](../../SPEC.md#L224)) plus crash classification ([SPEC.md:250](../../SPEC.md#L250)) —
under-weighted.

**Steps in Juggler.** (1) Tray/Inbox: "Instance crashed · api-server · 3 child processes still running."
Click → Instance panel incident card: exit signal, classification (OOM / quota / crash), process tree
with per-child CPU/RAM, and the transcript tail. (2) [Resume session] (native resume, transcripts
persist) or [Hand over…] if the Account is also quota-blocked. (3) Orphan sweep: Settings → Machine →
"Orphaned processes (3)" → table of PID, parent Session, age, command → user ticks rows → [Stop selected]
→ confirm sheet lists exactly what will be signalled (SIGTERM→SIGKILL ladder, SPEC §5.1
[SPEC.md:225](../../SPEC.md#L225)). (4) Debug: [Export incident pack] → T4 pack (SPEC §7.3
[SPEC.md:297](../../SPEC.md#L297)) plus machine snapshot, redacted by default ([SPEC.md:314](../../SPEC.md#L314))
→ save/share manually.

**Auto (passive).** Crash detection/classification, process-tree attribution (ppid map, SPEC §5.2
[SPEC.md:234](../../SPEC.md#L234)), orphan detection on restart, incident-pack assembly. Crucially, detection
is automatic; *killing is not* — see below. **Click-confirmed.** Every stop (per-row ticks + confirm),
every resume/handover, every export. Auto-kill of orphans would violate "no killing other applications"
([PROPOSAL.md:47](../../PROPOSAL.md#L47): actions "on its own sessions only") and the passive invariant. The
maintainer asks for "automatic and manual clean ups… with verifications" — Juggler's compliant reading is
automatic *detection and staging*, click-confirmed cleanup.

**Failure branch.** A child PID was reused by an unrelated process (max-depth report P0: "journal
compares PID only," [report:162](../AI-Juggler-Maximum-Depth-Report-and-Proposal.md#L162)): the sweep
must verify command line + start time before offering a row, and grey out anything it cannot attribute.
If attribution is uncertain, the row is informational only — never stoppable.

**Delight moment.** After a crash, one card answers "what died, what is still running, what do I click" —
no Activity Monitor archaeology. **Features required.** SPEC §5.1/§5.2/§5.3 (emit and surface them), 10
§2 inbox crash rows, §12 backup (incident pack rides the export path), §17 audit (every stop logged).
**Emotion + time saved.** Incident panic → a checklist. `[inferred]` 10–25 min per crash versus manual
`ps`/kill hunting, plus the avoided harm of killing the wrong process.

## W8. Offline / travel `[inferred]`

**Trigger.** Priya flies to a client; Alex works from a lab with flaky network (07 §6 pain: "flaky
network stalls login flows"). Quotas, statusline feeds, and harness-update checks all assume connectivity
the user temporarily does not have. No prior section covers offline operation; SPEC's only network uses
are the update feed and user-initiated harness actions ([SPEC.md:23](../../SPEC.md#L23)), so the core is
already local-first — the *degradation copy* is what is missing.

**Steps in Juggler.** (1) Before travel, user opens Usage → per-Account cards show last-measured quota
with timestamps ("measured 09:12, 2h ago"). (2) On the plane, launch works fully (local CLIs); banners
state "Offline — quota figures are last-measured, not live" with the honesty tier downgraded to
Estimated-stale (10 §1 controls: `estimated-stale` > 15m). (3) Handover between local Instances still
works (T1/T2 are file copies, [SPEC.md:280](../../SPEC.md#L280)); cross-Provider T3 replay works offline
because priming is local. (4) On reconnect, figures refresh and each card flips back to Measured with a
"refreshed just now" stamp; nothing queued "to send" is sent automatically because nothing was queued —
Fleet (SPEC §11) is LAN/VPN by design and simply shows peers as unreachable.

**Auto (passive).** Staleness labelling, offline banner, refresh on reconnect, harness-update check
deferred (never blocking, 10 §13 `check-failed-offline` is neutral). **Click-confirmed.** Every
launch/handover as usual; login flows that need the network are surfaced as vendor steps the user
retries, never automated ([PROPOSAL.md:43](../../PROPOSAL.md#L43)).

**Failure branch.** A login expires mid-trip: the Account card shows `expired` with the vendor-login CTA
greyed "needs network" rather than a spinner that never resolves. If the user ignores staleness and
launches into a nearly-empty window, the forecast pill itself carries the stale label — the number is
never presented as live.

**Delight moment.** The app is *calmer* offline: same board, honest labels, zero error spam. **Features
required.** 10 §1 staleness states, §13 offline states, §8 account health; SPEC local-first posture
([SPEC.md:23](../../SPEC.md#L23)) does the heavy lifting. **Emotion + time saved.** Travel anxiety → "it just
works, and says what it does not know." `[inferred]` saves no minutes directly; it saves the trip's first
hour of tooling distrust.

## W9. Team handoff — export pack to a colleague

**Trigger.** Priya goes on leave; Sara's colleague must reproduce a bug. The task *content* must move to
another person even though Accounts never do. Prior framing risked over-blocking this: "sharing profiles
or pools between people is out of scope" ([PROPOSAL.md:45](../../PROPOSAL.md#L45)) is a non-goal about
Accounts, not about a user-exported work product. Sara's own journey already needs a "sanitised pack for
internal repro" (07 §9.5).

**Steps in Juggler.** (1) Session panel → [Export pack…] → wizard: step 1 contents (task, decisions,
TODOs, diffs — PACK.md layout, SPEC §7.3 [SPEC.md:297](../../SPEC.md#L297)); step 2 redaction preview with
likely secrets highlighted, default-on for export (full review F1.6); step 3 destination: [Save
.juggler-pack…]. (2) User sends the file by their own channel (email/chat/USB) — Juggler has no send
button and no cloud relay ([SPEC.md:369](../../SPEC.md#L369)). (3) Colleague: File → [Import pack…] → preview
of exactly what will prime a new Session → picks their own Provider/Account → [Create session from pack].
(4) The imported Session's lineage records "imported pack from <file>" — provenance without any Account
sharing.

**Auto (passive).** Pack assembly from the Transcript IR, secret scanning, lineage stamping.
**Click-confirmed.** Export (after the redaction preview), the manual send (outside Juggler entirely),
and the import-side session creation. Nothing syncs, pools, or grants access; vendor credentials never
enter the pack ([SPEC.md:310](../../SPEC.md#L310)).

**Failure branch.** The colleague lacks the Provider or the repo: import still creates the primed Session
for any managed harness (T4 works for any target, [SPEC.md:274](../../SPEC.md#L274)) and warns "repo path
differs — primed context references the sender's path." If redaction finds a credential-shaped string,
[Save] stays blocked until it is redacted or the user overrides with an audit-logged confirmation (07 §3
red line).

**Delight moment.** The colleague's first Session opens already knowing the task — the pack, not a
re-told story, did the work. **Features required.** 10 §12 export path, pack redaction (F1.6), §17 audit;
SPEC §7.2 flow 5 export/import ([SPEC.md:290](../../SPEC.md#L290)). Positioning note for §23 copy: "share the
work, never the Account." **Emotion + time saved.** Leave-taking anxiety → a verified artefact.
`[inferred]` 20–45 min versus a handover doc plus a "quick call," per transfer.

## W10. End-of-week cost review `[inferred]`

**Trigger.** Friday 17:00. Maya decides whether Max #2 earns its money; Priya invoices; Jonas reports to
sponsors; Alex decides on a $5 top-up. Prior 10 §6 builds the ledger as a feature; nobody designed the
*Friday ritual* that makes it a habit (07 §7.4: "every cost view must end in a decision sentence or
export").

**Steps in Juggler.** (1) Usage → [This week] → matrix: Provider × Account × Project, tokens → Estimated
USD ("API-equivalent value, not your subscription fee," 10 §6), each Account card carrying its manual
subscription fee and one decision sentence: "Max #2 absorbed 3 quota walls this week ≈ $X of API value on
a $200 fee." (2) Anomalies row: top-3 burn Sessions, any Session that burned
> 25% of a window, any unpriced model shown as `—`, never `$0.00`.
(3) [Export CSV] for Priya's per-client invoice (10 §28.3); [Copy summary] for Jonas's sponsor post. (4)
Housekeeping strip: "N Sessions older than 30 days — [Review & archive]" (10 §5), which is also when W2's
parked leftovers and W1's overnight remains get closed out.

**Auto (passive).** Weekly rollups (SPEC §6 [SPEC.md:258](../../SPEC.md#L258)), anomaly detection,
decision-sentence templating from recorded walls/handovers (10 §17 events). **Click-confirmed.** Exports,
archiving, any Account change the review inspires. The review never cancels, downgrades, or re-routes
anything itself.

**Failure branch.** Pricing is stale (> 30d) or a model is unpriced: figures carry `stale-pricing` / `—`
and the export's cover note says the total is partial (10 §28.3 failure branch: "never silently sum a
partial total as if complete").

**Delight moment.** Renewal stops being a vibe: one sentence per Account, backed by the week's recorded
walls. **Features required.** 10 §6 ledger, §17 audit-fed decision sentences, §5 archive, §12 export
bundle. **Emotion + time saved.** Sunday-night money anxiety → a five-minute Friday close. `[inferred]`
Priya's 45-minute spreadsheet (07 §3) → < 5 min; Maya/Jonas save the reconstruction, not just the
arithmetic.

## W11. Provider outage day — lab down, failover whole Project

**Trigger.** A Provider's lab is down for hours: every Instance on that Provider fails or hangs at once.
This is not one Session's quota wall (W2) and not one crash (W7); it is correlated failure. No prior
section models it — account health (10 §8) is per-Account login state, not Provider availability.
`[inferred]`

**Steps in Juggler.** (1) Board banner appears passively: "Provider Claude: 4 of 4 Instances failing
since 10:02 — likely Provider-side" (pattern over per-Session errors, not a network probe — Juggler never
proxies traffic, [PROPOSAL.md:42](../../PROPOSAL.md#L42)). (2) User clicks the banner → Project failover
sheet: for the affected Project, target Provider/Account rows ranked by headroom and fidelity tier (T3
replay to a different Provider, [SPEC.md:273](../../SPEC.md#L273); T2 unavailable cross-Provider, stated
plainly). (3) [Fail over this Project…] → per-Session checklist (the two live Sessions move; finished
ones stay) → [Confirm]. (4) New Instances launch on the surviving Provider primed by replay; original
Sessions remain resumable for the return. (5) When the Provider recovers (first Instance on it next
succeeds), an Inbox row suggests [Return Project to Claude] — the W2 return pattern at Project scope
([SPEC.md:288](../../SPEC.md#L288)).

**Auto (passive).** Failure-correlation banner, impact list ("3 Projects, 5 Sessions on this Provider"),
pack staging for every checked Session, recovery suggestion. **Click-confirmed.** The failover itself,
per Project — never a fleet-wide auto-move, which would be auto-rotation at scale and is prohibited
([PROPOSAL.md:62](../../PROPOSAL.md#L62)). Deselected Sessions are never touched.

**Failure branch.** The surviving Provider lacks headroom for all checked Sessions: the sheet downgrades
to "fail over 2 of 4 now, park 2 until reset" (composing W2) instead of over-committing a window. If the
outage is actually local (user's network), the banner must say "Instances failing; cause unknown — could
be local network" (W8's honesty rule) rather than naming the lab.

**Delight moment.** A bad morning becomes one sheet and three clicks per Project, with the way back
already staged. **Features required.** 10 §8 extended to Provider-level health, §3 routing at Project
scope, §2 banner; SPEC T3 + return flow; EPICS M4 handover tiers are the substrate. **Emotion + time
saved.** "Everything is down" helplessness → a runbook. `[inferred]` 1–3 hours of manual re-priming per
Project avoided; frequency is low, severity is the whole day.

## 12. Cross-workflow synthesis

**One spine, eleven costumes.** Every workflow rides the same four dependencies, in order: status
emission (SPEC §5.1 — modelled, never emitted; full review F2.5), quota/forecast (10 §1), the Inbox (10
§2), and honest handover tiers (SPEC §7.1). W1–W4 and W11 are impossible before that spine; W5, W6, W9
compose on top of it; W7, W8, W10 harden its edges. This confirms — and sequences — the maximum-depth
report's Phase 1 trio (forecast + inbox + honest handover) from the workflow side.

**New or extended proposal items this pass adds:**

| Item | New / extends | Consumed by | Priority signal |
|---|---|---|---|
| Multi-Session park queue + stagger warning | Extends 10 §9 | W2, W11 | P1, rides the spine |
| Overnight pre-flight + morning digest | New slice of 10 §7 | W1, W3 | P1 `[inferred]` |
| Orphan sweep (detect auto, stop on click) | Surfaces SPEC §5.2 | W7 | P1; maintainer paragraph |
| Provider-level health + Project failover | Extends 10 §8/§3 | W11 | P2; rare, severe |
| Model-aware routing (cheap→strong) | Extends 10 §3 | W6 | P2 `[inferred]` |
| Review pair-switching (diversity ranking) | Extends 10 §11 | W5 | P2, behind dogfood gate |
| Offline staleness copy + stale-pills | Extends 10 §1/§13 | W8 | P1; cheap, copy-level |
| Weekly review ritual + decision sentences | Ritual over 10 §6 | W10, W4 | P1 habit surface |

**Guardrail re-check (all workflows).** None auto-rotates an Account, auto-resumes on a different
Account, auto-kills a process, auto-sends a pack to another person, or proxies traffic. The two
deliberate tensions, stated openly: (a) the maintainer's "automatic… clean ups"
([expected-worfklow.md:18](../../expected-worfklow.md#L18)) is honoured as automatic detection + staged
cleanup, because auto-kill conflicts with [PROPOSAL.md:47](../../PROPOSAL.md#L47); (b) W9 ships a
cross-person file while honouring the Account non-goal verbatim — the copy distinction is Accounts vs
work.

**Terminology follow-through.** If §1's ruling is accepted, UI copy passes rename Harness → Provider and
demote Profile to an advanced field; Session/Instance follow ADR-0007. Without that pass, every workflow
above reads twice — once in each vocabulary — which is the current F2.2 conflation repeated.

## 13. Validation and unresolved gaps

Falsify before building beyond the spine (in 07 §11 style):

1. **Overnight demand (W1/W3).** Five dogfood diaries, one week: do the 01:14 waiter and the 15-Session
morning recur at the modelled rates? If not, W1 shrinks to a digest setting. 2. **Arbitrage economics
(W6).** Price two real features both ways (all-strong vs cheap→strong) including replay cost; the
workflow survives only if the split saves ≥ 30% of burn. 3. **Pair-review demand (W5).** Interview signal
only today; keep behind 10 §11's diary gate. Do not build ranking first. 4. **Outage frequency (W11).**
Log Provider-correlated failures passively for a beta cycle before building the failover sheet; the
banner alone may be sufficient value. 5. **Time-saved figures.** All estimates above are `[inferred]`
against named manual workarounds; re-measure with the 10 §28 scripted walks (W3) and 07 §11 diaries (W1,
W4, W10).

Unresolved evidence, named plainly: no competitor was observed offering overnight digests, Project-scope
outage failover, or cheap→strong staged handover in the prior parts (04–06), so W5, W6, and W11 carry no
competitor URL by design — they are extensions of Juggler's own wedge (named multi-Account, quota,
handover), and that absence is exactly why they are marked `[inferred]` rather than asserted as table
stakes.

*End of Pass 2 §07. Claims cite repo `file:line`, a prior-report section, or are marked `[inferred]`; no
workflow restates the prior report's text.*

