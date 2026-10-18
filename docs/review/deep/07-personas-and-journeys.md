# 07 — Personas & Journeys (Deep Dive)
> Parent: [full review](../full-competitive-ux-review.md#L1) · Inputs: [PROPOSAL.md](../../PROPOSAL.md#L49), [SPEC.md](../../SPEC.md#L86), prior map [review:729](../full-competitive-ux-review.md#L729). Quotes/diaries are `[composite]` verbatim-style, not interviews — validate §9. Repo claims cite `file:line`; competitors link out.
## 0. Reading guide & cross-persona truths
Marks: `[repo]` grounded · `[composite]` invented voice · `[inferred]` needs dogfood · `[competitor]` URL.
Prior 4 seeds ([PROPOSAL.md:51](../../PROPOSAL.md#L51)) split into 6: solo maximiser, salaried operator, and billable consultant optimise for momentum, throughput, and client separation — opposite at the quota wall.
1. Quota-blindness is #1 for 5/6; roadmap inverts it (M3 after adapters) [review:15](../full-competitive-ux-review.md#L15). 2. `Stuck/WaitingUser/QuotaBlocked` modelled, never emitted [review:30](../full-competitive-ux-review.md#L30) — "which needs me?" unanswerable. 3. Handover v0 copies one JSONL, siblings left [review:20](../full-competitive-ux-review.md#L20); trust won/lost there. 4. No worktree; session=only `cwd` [review:63](../full-competitive-ux-review.md#L63). 5. No USD view [review:76](../full-competitive-ux-review.md#L76). 6. Onboarding best-built, protect it [review:53](../full-competitive-ux-review.md#L53). 7. Suggest-only, never auto-rotate ([PROPOSAL.md:62](../../PROPOSAL.md#L62)). 8. Instances/Sessions conflated [review:33](../full-competitive-ux-review.md#L33).
| # | Persona | Subs/mo* | Sessions/day | Pays how |
|---|---|---|---|---|
| 1 | Maya, solo indie Multi-Maxer | ~$450–620 | 4–8 | Self, reluctantly |
| 2 | Devon, staff eng 10+ parallel | $0 (employer ~$500+) | 10–20 | Employer; OSS time |
| 3 | Priya, agency multi-client | $600–1,200 pass-thru | 6–14 | Client-billed |
| 4 | Jonas, OSS maintainer | $20–100 | 3–10 bursty | Free; contributes code |
| 5 | Sara, enterprise-locked | $0 visible | 2–6 approved | Procurement; cannot pay |
| 6 | Alex, student optimiser | $20–38 | 1–4 | Free only; churns fast |
\* Estimates from tiers in [research/zcode.md:15](../../research/zcode.md#L15) (GLM Lite $18/Pro $80/Max $168 via [codingplan.org](https://codingplan.org/Coding%20Plans)); order-of-magnitude only.
---
## 1. Maya — Solo Indie Multi-Maxer (seed A, [PROPOSAL.md:51](../../PROPOSAL.md#L51))
**Snapshot:** 34, Lisbon; solo SaaS founder ~$4k MRR `[inferred]`; M3 Pro 36GB; 2×Claude Max+Codex Plus+GLM Lite; 4–8 sessions/day, 2–3 repos; expert CLI, aliases+direnv today ([PROPOSAL.md:25](../../PROPOSAL.md#L25)).
**JTBD:** Functional "keep shipping when one account dies mid-feature" · Emotional "stop feeling stupid paying twice and still stalling" · Social "look prolific without hiring".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:30 | Tray: A 78%, B 22%, reset 17:00 — calm *if* quota exists; today blind |
| 09:00 | Launch feature+tests — delight 1-click; fear wrong account |
| 11:20 | Session waited 25m on permission, unseen — status unemitted `[repo]` |
| 13:00 | Quota wall A mid-refactor — panic: re-explain or wait 4h? |
| 13:05 | Handover A→B — distrust: "what did it drop?" (siblings, F3.4) |
| 15:30 | B finishes — momentum kept *iff* handover honest |
| 17:05 | Reset, return to A — aha if 1-click; churn if manual copy |
| 21:00 | Cost check — anxiety: no USD ledger (F1.2) |
### Top-10 pains (F×S, 1–5 each)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Quota wall, no warning | 25 | 6 | Same-branch collision | 12 |
| 2 | Waiting invisible | 16 | 7 | Alias fragility on CLI update | 9 |
| 3 | Handover drops todos/subagents | 15 | 8 | Reset, no resume nudge | 8 |
| 4 | Wrong-account billing/tier | 15 | 9 | No archive/search pile-up | 8 |
| 5 | No USD for Max#2 renewal | 12 | 10 | Notifications none-or-spam | 6 |
**Workarounds:** `CLAUDE_CONFIG_DIR` aliases+direnv ([research/claude-code.md:34](../../research/claude-code.md#L34)), ccusage cron, sticky-note reset times, manual JSONL `cp`.
> `[composite]` "I pay for two Max accounts so I never stop. Then I stop — I can't see the wall coming and the spare needs the whole story re-told."
> `[composite]` "Don't auto-switch me. Tap my shoulder at 80%, have the other account *ready*. The click is mine."
**Diary Tue 13:04:** *"A died mid billing-migration. Todos were in that session. B re-asked about Stripe webhooks I'd answered at 10am."* — sibling loss [review:289](../full-competitive-ux-review.md#L289).
**Aha:** forecast "A exhausts ~40m; B 80%" (WOW-b [review:769](../full-competitive-ux-review.md#L769)) + fidelity label pre-click. **Churn:** one silent handover loss or wrong-account bill. **WTP/OSS:** $0 core; sponsor $5–15/mo, $49 one-off Pro pack; bug reports, no Rust. **A11y/neuro:** ADHD context-switch cost — inbox by waiting-time, colour+text never colour-only [review:840](../full-competitive-ux-review.md#L840), reduced motion.
---
## 2. Devon — Staff Engineer, 10+ Parallel (seeds D+C, [PROPOSAL.md:54](../../PROPOSAL.md#L54), [PROPOSAL.md:53](../../PROPOSAL.md#L53))
**Snapshot:** 41, Austin; staff eng 80-person startup; M4 Max 64GB + Linux desktop (fleet); employer Claude Max×2/Codex/Cursor expensed; 10–20 sessions/day, 4 services; judges Juggler as infra.
**JTBD:** Functional "run agents like CI: triage, not babysit" · Emotional "control at 15 sessions, not haunted by tabs" · Social "the person whose agents ship".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:00 | Overnight: 3 done, 2 stuck, 1 quota-blocked — needs inbox; statuses unemitted |
| 09:00 | Launch 6 parallel — admission OK/Tight/No matters (SPEC §5.2) |
| 10:30 | RAM/swap thrash — wants per-session footprint, not Activity Monitor |
| 12:00 | 5 finished, diffs unseen — review bottleneck `[inferred]` (WOW-h) |
| 14:00 | Quota wall shared Max — forecast would have routed at launch |
| 16:00 | Jump sessions — rail caps 15/8, no search (F2.5); wants ⌘K |
| 18:30 | Handover A→Codex (T3/T4) — advertised, not built (F3.1): trust hit |
| 20:00 | 4 overnight left — digest not pings (WOW-g) or he mutes all |
Expectations set by Conductor worktree+diff [concepts](https://www.conductor.build/docs/concepts/workflow), Vibe inline-comment-to-agent [review:639](../full-competitive-ux-review.md#L639), Squad 1-task=1-worktree [review:579](../full-competitive-ux-review.md#L579). Juggler wins only on juggling.
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | "Which of 15 needs me?" | 25 | 6 | Rail truncation, no search | 15 |
| 2 | Review bottleneck `[inferred]` | 20 | 7 | Cross-harness handover absent | 10 |
| 3 | Worktree/branch collisions | 20 | 8 | Scale jank 20 sess. (F5.1) | 9 |
| 4 | Machine overload 10+ | 16 | 9 | Alert fatigue→mute→miss | 8 |
| 5 | Stuck vs waiting-on-model | 16 | 10 | Parallel quota burn (Pro cap [review:617](../full-competitive-ux-review.md#L617)) | 16 |
**Workarounds:** Claude Squad/Conductor trial, tmux+worktrees, [ccusage](https://github.com/ryoppippi/ccusage), `ps`/notify scripts, Slack self-DMs as inbox.
> `[composite]` "I don't have a coding problem. I have air-traffic control with fifteen planes and no tower."
> `[composite]` "If 'handover' is copy-one-file-and-pray, say so on the button. I'll click — I just won't bet a migration on it."
**Diary Thu 08:02:** *"One agent asked a question at 01:14, waited seven hours. A badge '1 needs you' would have saved a sprint day."* — F2.5 lived.
**Aha:** inbox by waiting-time (WOW-a) + admission verdict; first morning it surfaces the 01:14 waiter in <10s. **Churn:** scale jank (watcher full-reads, Mutex — F5.1) or T3/T4 overclaim found live. **WTP/OSS:** employer $20–50/seat/mo plausible (vs Conductor Pro $50 [pricing](https://www.conductor.build/pricing)); personally $0; contributes Rust, load fixtures, conformance tests. **A11y/neuro:** keyboard-only ⌘K ≤3 keystrokes [review:855](../full-competitive-ux-review.md#L855), icons+text (dyslexia), batched notifications for focus.
---
## 3. Priya — Agency/Consultant, Multi-Client (B generalised, [PROPOSAL.md:52](../../PROPOSAL.md#L52))
**Snapshot:** 38, Toronto; 6-person agency, 4 clients; M3 Max 48GB, client VPNs; agency Claude Max×2 + per-client Codex + GLM Pro, $600–1,200/mo pass-thru; 6–14 sessions/day; she is who clients audit. Wrong-account = wrong *client* billing (Claude #87447, [research/claude-code.md:36](../../research/claude-code.md#L36)).
**JTBD:** Functional "prove which account touched which client repo, when" · Emotional "never fear a client seeing another client's name" · Social "look enterprise-grade in security review".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:45 | Client A standup — needs per-client filter; rail is flat today |
| 09:30 | Launch on wrong profile (similar labels) — dread; affinity guard absent (F1.4) |
| 11:00 | Screen-share, other client's session visible — shame risk; needs Present mode `[inferred]` |
| 13:00 | Handover within Client B — needs redaction preview before pack moves (F1.6) |
| 15:00 | Client: "what did AI do Tuesday?" — lineage minimal (F3.3), no CSV (F1.2) |
| 17:30 | Invoice: tokens→USD per client — manual ccusage+spreadsheet, 45m |
| 19:00 | Quota wall shared Max — other client's work must NOT auto-move (guardrail) |
Red line: packs verbatim, redaction only for diagnostics [review:120](../full-competitive-ux-review.md#L120) — for Priya a confidentiality incident on a tired Friday. Secret-scan preview is P0 *for her*, P1 globally; put it in the wizard, not Settings.
### Top-10 pains (F×S; * = existential, rank by S first)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Per-client cost/provenance export absent | 20 | 6 | Quota shared, no per-client budget | 12 |
| 2 | Cross-client leakage on share* | 15 | 7 | Handover lineage too thin to audit | 12 |
| 3 | Wrong-client billing* | 15 | 8 | Pack leaks secrets verbatim* | 10 |
| 4 | Credential-copy vs SPEC S8 in audit (F4.1)* | 10 | 9 | Profile clone/MCP sync absent | 9 |
| 5 | No Present/client mode `[inferred]` | 9 | 10 | Client data retained forever (F1.3) | 8 |
**Workarounds:** separate macOS users per big client, [cc-switch](https://github.com/farion1231/cc-switch) tray, ccusage per config dir, manual redaction, Toggl+spreadsheet allocation.
> `[composite]` "Clients don't care which model. They care Client A's roadmap never appears in Client B's status call."
> `[composite]` "Give me one page — this account, this repo, this spend — and your logo goes in my proposal deck."
**Diary Fri 16:58:** *"Exported a pack to work at home. Staging key sat in a tool result. Nothing bad happened. I still feel sick. A preview catches it in four seconds."* — F1.6 embodied.
**Aha:** wizard step-2 redaction preview + usual-account marker (UX.5 [review:843](../full-competitive-ux-review.md#L843)), then per-client CSV. **Churn:** any cross-client incident, or auditor hitting `.credentials.json` copy (F4.1) unanswered. **WTP/OSS:** highest — $30–60/seat/mo client-billed *if* audit export+redaction land, else $0 and OS-user separation stays; case studies, redaction patterns, sponsorship. **A11y/neuro:** screen-share anxiety is a feature — Present mode hides other clients/emails; client colour+name, never colour alone; predictable focus for live demos.
---
## 4. Jonas — OSS Maintainer, Bursty & Broke (new; OSS model [review:719](../full-competitive-ux-review.md#L719), sunset window [review:507](../full-competitive-ux-review.md#L507))
**Snapshot:** 29, Berlin; employed by day, maintainer by night; M2 Air 16GB + CI; Claude Pro + GLM Lite $18, $20–100/mo partly sponsored; 3–10 sessions/day, release weeks 15+; expert, reads Rust, files perfect bugs.
**JTBD:** Functional "triage 40 issues without burning week's quota by Tue" · Emotional "help everyone without resenting the project" · Social "good steward: transparent, responsive, kind".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 07:30 | Launch 3 issue-repro sessions — admission Tight on 16GB: honest, useful |
| 09:00 | Day job; Juggler sessions must survive/adopt (SPEC reliability) |
| 18:00 | Weekly quota 91% Thu on small Pro tank — forecast matters *more*, not less `[inferred]` |
| 19:00 | Handover Pro→GLM Lite to save Claude — cross-harness T3 absent, replay lossy |
| 20:30 | Review contributor PRs — needs diff queue; terminal-only is slow |
| 22:00 | Archive release-week sessions — no lifecycle → 200 stale rows (F1.3) |
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Small Pro quota, no forecast → mid-week wall | 20 | 6 | No sponsor cost reporting | 9 |
| 2 | 16GB: 3 sessions = swap death | 16 | 7 | Update breakage (weekly harness churn) | 9 |
| 3 | Repro sessions pile up indistinguishable | 15 | 8 | Onboarding imports day-job profiles too | 8 |
| 4 | Claude→GLM handover absent | 12 | 9 | Dense rail on 13" laptop | 8 |
| 5 | Stuck detection absent overnight | 12 | 10 | No adapter SDK to contribute (F1.8) | 4 |
**Workarounds:** [Claude Squad](https://github.com/smtg-ai/claude-squad) TUI, tmux+worktrees, ccusage weekly sponsor report, GitHub labels as session tracker, "quota-budget Sunday" ritual.
> `[composite]` "Pro is not 'less Max'. It's a different sport. Your forecast must work when the tank is tiny."
> `[composite]` "I can't pay you. I can write your adapter for the tool I maintain — give me a conformance test to run."
**Diary Thu 18:12:** *"91% and it's Thursday. Release is Saturday. Review the security PR now or save quota for the release fire? A tool that just shows the cliff edge is already worth installing."*
**Aha:** burn-down on *small* quota + admission respecting 16GB — "built for me, not Max whales" (positioning risk if ignored). **Churn:** Max-only feel (Pro weekly ignored) or nagging RAM advisor. **WTP/OSS:** $0, $5/mo symbolic sponsorship; real currency adapter PRs, fixtures, i18n, dogfood — unlock via conformance kit (F1.8/F5.5). **A11y/neuro:** burnout-aware quiet digest default, no streaks/gamification; screen-reader labels (VoiceOver triage when eyes tire — untested, Appendix A [review:1014](../full-competitive-ux-review.md#L1014)); low-vision density options.
---
## 5. Sara — Enterprise-Locked Developer (stresses compliance [PROPOSAL.md:56](../../PROPOSAL.md#L56), non-goal [PROPOSAL.md:45](../../PROPOSAL.md#L45))
**Snapshot:** 36, Stockholm; bank/fintech regulated; MDM MacBook Pro 32GB, no admin, allowlist; employer Claude SSO + approved Codex only, $0 visible; 2–6 sessions/day approved accounts; strong dev, zero ToS-ambiguity tolerance. User, not buyer — potentially a blocker.
**JTBD:** Functional "use two approved accounts without IT ticket per switch" · Emotional "never worry Legal flags my tooling" · Social "recommend to team without staking my name".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:30 | Install needs signed/notarised + MDM — unsigned = non-starter (M5) |
| 09:00 | Add work account via vendor SSO in embedded terminal — good (F4.3 vendor-owned login) |
| 09:10 | Personal account prompt — she must NOT; needs hard separation/opt-out |
| 11:00 | Security: "does it copy credentials?" — F4.1 ambiguity blocks deployment |
| 14:00 | Quota wall, only approved spare — suggest-only is a *feature* (click audit trail) |
| 16:00 | Export pack for colleague? — non-goal sharing ([PROPOSAL.md:45](../../PROPOSAL.md#L45)) must refuse clearly |
| 17:00 | Telemetry prompt — opt-in only (SPEC privacy) or uninstall |
### Top-10 pains (F×S; * = existential blocker, any one = zero adoption)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Multi-account optics "circumventing?" | 12 | 6 | Pack redaction absent | 9 |
| 2 | No handover click audit log `[inferred]` | 12 | 7 | Windows/Linux absence, mixed team | 8 |
| 3 | F4.1 vs SPEC S8 unanswered* | 10 | 8 | DB backup/restore story (F1.7) | 8 |
| 4 | Unsigned build cannot install* | 10 | 9 | Onboarding scan finds personal profiles | 6 |
| 5 | Fleet mDNS/gRPC triggers netsec review* | 5 | 10 | Vendor ToS change, no comms plan* | 5 |
**Workarounds:** vendor CLIs only, IT-approved VS Code, logout/login account switch (30m), Confluence "which account for which project". Envies Maya, cannot copy her.
> `[composite]` "Your guardrails page is why I'm here. Your credential copy is why Legal said 'not yet'. Both are you — pick."
> `[composite]` "I don't need team features. I need a signed binary, a one-page data-flow diagram, and proof you never proxy."
**Diary Wed 11:20:** *"Security review, minute 14: 'Does it copy credentials?' I said spec says never. Reviewer found the import code. Meeting over. The tool didn't fail — the story did."* — F4.1/F6.2 truth gap.
**Aha:** an artefact, not a feature — decided F4.1 + `guardrails.rs` exists (F4.2) + signed build; then inbox. **Churn:** never adopts or forced uninstall; organisational, not experiential. **WTP/OSS:** cannot pay; procurement $50–100/seat/yr only with signed/MDM/support — do NOT promise in OSS v1; contributes security reviews, threat-model PRs. **A11y/neuro:** WCAG AA as procurement checkbox (UX.8 P1s are *sales* features); predictable UI, no `window.alert` surprise modals (F2.1 fails enterprise review instantly); full keyboard + focus-visible.
---
## 6. Alex — Student/Hobbyist Cost-Optimiser (segment proven by GLM credits [research/zcode.md:14](../../research/zcode.md#L14))
**Snapshot:** 22, Nairobi, remote MSc + hackathons; M1 Air 8GB or lab Mac; Claude Pro *or* GLM Lite $18 + $5 API top-ups, $20–38/mo max; 1–4 sessions/day (thesis + side app); fast learner, price-first, skims docs.
**JTBD:** Functional "stretch $20 across thesis + demo" · Emotional "don't feel poor next to Max screenshots" · Social "portfolio project that looks senior-built".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 09:00 | Lab Mac fresh install, scan finds nothing — onboarding must shine empty (protect F2.6) |
| 09:20 | GLM Lite credits 2,000/5h as *budget* — needs Computed label + "≈N sessions" |
| 10:00 | 2 sessions on 8GB — admission No + lighter suggestion, not shame |
| 13:00 | Quota 80% before demo rehearsal — forecast saves demo; wall kills it |
| 15:00 | Handover Claude→GLM (cheaper) — T3 replay quality decides trust in cheap tier |
| 19:00 | Hackathon sprint, burn visible live — delight if glanceable |
| 23:00 | $5 top-up decision — USD ledger (F1.2) vs vibes |
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Wall before deadline/demo, no forecast | 20 | 6 | Budget cap/alert absent `[inferred]` | 12 |
| 2 | 8GB: heavy session swaps, all slow | 20 | 7 | Jargon learning curve (profile/transplant) | 8 |
| 3 | Credits not translated to money/time | 15 | 8 | Search absent at 60 thesis sessions | 6 |
| 4 | Built-for-multi-Max feel in onboarding | 12 | 9 | Flaky network stalls login flows | 6 |
| 5 | Handover to cheaper tool absent | 12 | 10 | Paywall/enshittification anxiety | 6 |
**Workarounds:** one terminal, `/usage` checks, [ccusage](https://github.com/ryoppippi/ccusage) screenshots to himself, manual free-tier hopping, deleting sessions to "save quota" (myth — UI should educate honestly).
> `[composite]` "Everyone's screenshot has three Max rings. Mine has one Pro ring at 91% and a demo at nine. Build the cliff alarm for me, I'm your loudest campus user."
> `[composite]` "Don't show me credits. Show me 'about six sessions left this week at how you're working'. Then I can plan."
**Diary demo-eve 13:20:** *"Rehearsal used the last window. Demo at four runs on the free tier with a model that formats code wrong. I practised the talk. I didn't practise begging the quota gods."*
**Aha:** "≈6 sessions left" + 80% alarm with cheap-tier suggestion (suggest-only WOW-d); first saved demo = advocate. **Churn:** paywall on quota/forecast (core promise), Max-centric onboarding, 8GB jank blamed on him. **WTP/OSS:** $0 core forever, maybe $12/yr student sync; tutorials, campus talks, walkthrough videos = top-of-funnel gold; docs/i18n/first-run bugs (only persona who *is* a new user every semester). **A11y/neuro:** colour-blind-safe + text labels, plain-language glosses, single-session focus view, no dense multi-ring overwhelm.
---
## 7. Cross-persona deep cuts (what the one-line pains hide)
### 7.1 The quota wall is three different walls
Maya hits a *momentum* wall (context in flight), Devon a
*throughput* wall (parallel burn multiplies it), Alex/Jonas
a *budget* wall (small tank, deadline-shaped). One ring UI
serves all three only if it shows: time-to-exhaust at current
burn (Maya), per-session burn attribution (Devon), and
sessions-remaining translation (Alex/Jonas). A percentage
alone — the hardcoded `percent={0}` placeholder today
([review F2.4](../full-competitive-ux-review.md#L193))
— serves none. Honesty tiers (Measured/Computed/Estimated,
SPEC.md:254-259 via review) must be visible per wall type.
### 7.2 Handover trust is persona-asymmetric
For Maya/Devon a lossy handover wastes an afternoon; for
Priya it can leak a client secret; for Sara it fails an
audit; for Alex it decides whether cheap tiers are usable
at all. Hence fidelity labelling cannot be a tooltip: it is
the wizard's step 2 (UX.5), with carry-list in plain words
("transcript + subagents carried; MCP state does not
transfer"). T3/T4 advertised-but-absent (F3.1) hurts Devon
most because he will *discover it mid-incident*.
### 7.3 "Which needs me?" scales non-linearly
At 2 sessions (Sara/Alex) the rail suffices; at 8 (Maya/
Priya) badges matter; at 15+ (Devon) only a waiting-time-
sorted inbox works, and rail truncation (15/8 caps, F2.5)
actively hides the oldest waiter. Design the inbox data
model fleet-ready now (WOW-j) even though Fleet ships late.
### 7.4 Cost is justification, not analytics
Maya: renew Max #2? Priya: bill the client. Jonas: report
to sponsors. Alex: spend $5 or not? Devon: show his manager.
Nobody wants a chart for its own sake — every cost view
must end in a decision sentence or export. CSV/PDF export
is therefore P1, prettier charts P2 (inverts typical Usage
dashboard instinct; cf. ccusage decision-shaped reports,
[ccusage](https://github.com/ryoppippi/ccusage)).
### 7.5 Accessibility is not one persona's row
Colour-only status fails Maya (ADHD scanning), Priya (live
demo), Sara (procurement WCAG), Alex (colour-blindness) —
four different reasons, one fix (text+icon+colour, UX.8).
Keyboard flow is Devon's speed, Jonas's VoiceOver triage,
and Sara's audit demo. Treat UX.8 P1s as cross-persona P1.
---
## 8. Anti-personas — who NOT to build for
| Anti-persona | Why not | Response instead |
|---|---|---|
| Account sharers/poolers (5 friends, 1 Max) | Violates user-owned guardrail ([PROPOSAL.md:62](../../PROPOSAL.md#L62)) | Refuse in copy; point to team plans |
| Auto-rotation seekers ("rotate at 90%") | claude-swap auto-rotates [review:674](../full-competitive-ux-review.md#L674); Juggler is suggest-only | Suggested switch + 1 click; never auto |
| Proxy/gateway seekers | Prohibition zone ([PROPOSAL.md:125](../../PROPOSAL.md#L125)); non-goal ([PROPOSAL.md:42](../../PROPOSAL.md#L42)) | Document why; hard no |
| Team-pool managers (RBAC/SSO admin) | Non-goal v1 ([PROPOSAL.md:45](../../PROPOSAL.md#L45)); Fleet ≠ team | Fleet = my machines only |
| Chat-UI seekers | Non-goal ([PROPOSAL.md:46](../../PROPOSAL.md#L46)) | Embed terminal; harnesses own UX |
| Autonomous assigners | Non-goal ([PROPOSAL.md:44](../../PROPOSAL.md#L44)) | Inbox + suggestions; human assigns |
| Non-dev AI users | No harness/quota job | Out of scope; don't dilute onboarding |
| Windows-only teams (today) | macOS-first (SPEC platform) | Say so plainly; ports later |
**Anti-persona test per feature:** "Does this require
sharing, proxying, auto-rotating, or auto-assigning?"
Yes → decline citing the guardrail, never "roadmap it".
This also protects Sara: visible refusals are audit assets.
---
## 9. Journey maps (Discover→Onboard→Daily→Wall→Handover→Ship→Advocate)
Legend: 😀 delight · 😐 neutral · 😖 friction · 💥 trust-break. Opportunity = smallest flipping change.
### 9.1 Maya
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | "Another orchestrator?" → lead with juggling wedge (F7.1) |
| Onboard | 😀 | Scan finds t3 profiles; state F4.1 decision in summary |
| Daily | 😐 | Waiting invisible → emit WaitingUser + inbox badge (WOW-a) |
| Quota wall | 💥 | Blind + re-explain → forecast at launch (WOW-b) pre-empts |
| Handover | 😖 | Fidelity unknown → wizard tier + carry-list (UX.5) |
| Review/ship | 😐 | No diff queue `[inferred]` → read-only diff on finished (F7.3) |
| Pay/advocate | 😐 | Nothing to buy → sponsor + Pro pack, core free |
### 9.2 Devon
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Vibe sunset migrant (F7.4); import path absent → Squad/Vibe import |
| Onboard | 😖 | Instances≠Sessions (F2.2) → merge to Projects/Sessions now |
| Daily | 😖 | Rail caps, no search → ⌘K + filters (WOW-c) |
| Quota wall | 😖 | No per-session burn → burn attribution in inbox row |
| Handover | 💥 | T3/T4 advertised, absent (F3.1) → downgrade flags to RESUME-only |
| Review/ship | 😖 | 5 finished pile up `[inferred]` → review queue: diff stat + pack preview |
| Pay/advocate | 😀 | Employer pays if ROI shown → cost CSV for manager (F1.2) |
### 9.3 Priya
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | "Auditable?" → security/lineage one-pager (F4.1 decided) |
| Onboard | 😖 | Flat labels, leakage fear → client grouping + Present mode `[inferred]` |
| Daily | 😖 | Wrong-profile near-miss → usual-account marker + confirm (F1.4) |
| Quota wall | 😖 | Cross-client temptation → per-client budgets; suggest, never move |
| Handover | 💥→😀 | Secret in pack (F1.6) → redaction preview flips it |
| Review/ship | 😀 | Lineage thin for client Q&A → jump-links both ways (UX.5) |
| Pay/advocate | 😀 | Needs invoice-grade export → per-client PDF/CSV (F1.2) |
### 9.4 Jonas
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Vibe/Crystal sunset (F7.4), "free really?" → MIT clarity + import |
| Onboard | 😖 | Scan mixes day-job/OSS → selective import checkboxes |
| Daily | 😐 | 16GB Tight verdicts → lighter-profile suggestion, not block (SPEC §5.2) |
| Quota wall | 💥 | Small-tank forecast absent → Pro weekly burn-down, not Max-only 5h |
| Handover | 😖 | Claude→GLM T3 absent → honest "replay, lossy" label + pack preview |
| Review/ship | 😐 | Archive pile-up → lifecycle archive-after-release (F1.3) |
| Pay/advocate | 😀 | $0 but contributes → conformance kit invites adapter PR (F1.8) |
### 9.5 Sara
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | Multi-account optics → guardrails page as landing (PROPOSAL §5) |
| Onboard | 💥 | Unsigned + F4.1 unanswered → signed build + decided S8 = gate pass |
| Daily | 😐 | `window.alert`, dead nav (F2.1/F2.4) → banner errors; hide ComingSoon (UX.1) |
| Quota wall | 😐 | Only approved spare → click-audited suggestion (lineage log) |
| Handover | 😖 | Redaction absent → sanitised pack for internal repro (F1.6) |
| Review/ship | 😖 | No audit trail `[inferred]` → lineage + click-log export |
| Pay/advocate | 😐 | Procurement >> product → earn internal champion; don't sell yet |
### 9.6 Alex
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Max screenshots alienate → show Pro/GLM-Lite hero too |
| Onboard | 😀 | Jargon profile/transplant → plain-language glosses; protect wizard (F2.6) |
| Daily | 😖 | Swap jank blamed on user → admission + "1 session is fine" framing |
| Quota wall | 💥 | No cliff alarm → 80% alert + cheap-tier suggestion (WOW-d) |
| Handover | 😖 | Replay quality unknown → fidelity label "summary risk: medium" |
| Review/ship | 😀 | Search absent at 60 sessions → ⌘K (WOW-c) |
| Pay/advocate | 😀 | Paywall fear → "core free forever" pledge; campus advocacy |
### 9.7 Journey synthesis
Across 6×7=42 steps 💥 occurs only at **quota wall,
handover, and Sara's onboard** — never discover or ship.
Spend P0 on forecast + honest handover + truth artefacts,
not landing pages or review polish. Review friction (😖×5)
is survivable; one silent handover loss is not. This
corroborates Phase 1 ordering from the demand side
([review:914](../full-competitive-ux-review.md#L914)).
Emotion curves also diverge at *advocate*: Devon/Priya/Jonas
advocate with artefacts (CSV, case study, adapter PR),
Maya/Alex with stories (saved demo, never-stopped day),
Sara only inside her org after an audit passes. Marketing
must produce artefact-shaped proof, not testimonials alone.
---
## 10. Persona × Feature Priority Matrix
P0 churns/blocks without it · P1 before v0.1-beta *for that
persona* · P2 delight/later · — n/a. Global = max severity.
| Feature | Maya | Devon | Priya | Jonas | Sara | Alex | Global |
|---|---|---|---|---|---|---|---|
| Quota + forecast (WOW-b) | P0 | P0 | P1 | P0 | P1 | P0 | **P0** |
| Needs-Attention inbox (WOW-a) | P1 | P0 | P1 | P1 | P1 | P2 | **P0** |
| Honest handover wizard (UX.5) | P0 | P0 | P0 | P1 | P1 | P1 | **P0** |
| Worktree-on-launch (F1.1) | P1 | P0 | P1 | P1 | P2 | P2 | **P0/P1** |
| ⌘K + search (WOW-c) | P1 | P0 | P1 | P1 | P2 | P1 | **P1** |
| Cost ledger USD/CSV (F1.2) | P1 | P1 | P0 | P1 | P2 | P0 | **P1** |
| Redaction preview (F1.6) | P2 | P2 | P0 | P2 | P1 | P2 | **P1*** |
| Account health/affinity (F1.4) | P1 | P1 | P0 | P2 | P1 | P2 | **P1** |
| Reset-queue (F1.5) | P1 | P1 | P2 | P1 | P2 | P1 | **P1** |
| Lifecycle archive/search (F1.3) | P1 | P1 | P1 | P1 | P1 | P2 | **P1** |
| Admission advisor (SPEC §5.2) | P2 | P0 | P2 | P0 | P2 | P0 | **P1** |
| Tray quick-switch (F1.9) | P1 | P1 | P1 | P2 | P2 | P2 | **P1** |
| Guardrails.rs + F4.1 decision | P2 | P2 | P1 | P2 | P0 | P2 | **P0†** |
| Signed/notarised (M5) | P2 | P2 | P1 | P2 | P0 | P2 | **P1†** |
| Quiet notifications (WOW-g) | P1 | P1 | P2 | P1 | P2 | P2 | **P1** |
| Review queue/diff (WOW-h) | P2 | P1 | P1 | P1 | P2 | P2 | **P2‡** |
| Profile clone/MCP sync (F1.9) | P2 | P1 | P1 | P2 | P2 | P2 | **P2** |
| Fleet roll-up (WOW-j) | P2 | P1 | P2 | P2 | P2 | P2 | **P2** |
| Conformance kit (F1.8) | — | P1 | — | P0 | P2 | — | **P1** |
| Present/client mode `[inferred]` | — | — | P1 | — | P1 | — | **P2** |
| Budget cap alert `[inferred]` | P2 | — | P1 | P1 | — | P0 | **P2** |
\* P0 in Priya/Sara contexts — ship inside wizard.
† Blocker-class: gates trust/adoption, low daily visibility.
‡ `[inferred]` — dogfood-validate ([review:1007](../full-competitive-ux-review.md#L1007)).
**Reading:** forecast+handover+inbox are each P0 for ≥2
personas, P1 for the rest — Phase 1 trio survives persona
scrutiny. Fleet/review queue are never P0: keep post-1.0 /
dogfood-gated despite vocal Devon demand. Worktree is the
only near-P0 the prior Phase 1 under-weights for Devon.
---
## 11. Validation plan (falsify before building on this)
1. **5 dogfood diaries** (Maya+Devon, 1 week): does the
   01:14-waiter recur? Are these diaries representative?
2. **Handover trust A/B:** fidelity label vs none; measure
   click-through + post-handover "lost work?" — tests §9.7.
3. **Pro-quota check:** Jonas/Alex need weekly- or 5h-shaped
   forecast? Statusline covers both (SPEC §4.1) — verify UI.
4. **Priya interviews (n=3 agencies):** is Present mode and
   per-client PDF real, or `[inferred]` invention?
5. **Sara gate test:** F4.1 options paper to a real security
   reviewer *before* building; if neither passes, remove
   enterprise positioning rather than selling around it.
6. **Alex onboarding test (n=5 students, fresh Mac):** time
   install→first session; where does jargon stop them?
   Success metric mirrors PROPOSAL: install→2 accounts
   parallel <5m ([PROPOSAL.md:142](../../PROPOSAL.md#L142)) — Alex proves the 1-account variant.
## 12. Opportunity backlog by persona (sized, sequenced)
Sizing: S ≤1wk slice · M milestone item · L post-1.0.
Each row names the journey step it repairs (§9).
### Maya — keep momentum, keep trust
| Opportunity | Step | Size |
|---|---|---|
| 80% shoulder-tap + staged spare profile (no auto-switch) | Wall | S |
| Return-on-reset 1-click with delta summary ("while you were away", PROPOSAL G3) | Handover | M |
| Renewal report: "Max #2 saved N stalls ≈ $X vs API" | Advocate | M |
| Sticky ReadyHome skip per healthy profile (F2.7) | Daily | S |
### Devon — tower view or nothing
| Opportunity | Step | Size |
|---|---|---|
| Inbox default home; rail badge = unsnoozed count (UX.3) | Daily | M |
| Per-session burn + footprint in every inbox row | Wall/Daily | M |
| 20-session load test in CI before adapter #2 (F5.1) | Daily | M |
| Worktree-on-launch + same-branch warn (F1.1) | Daily | M |
| Fleet-ready inbox schema now, Fleet UI later (WOW-j) | Advocate | S |
### Priya — separation is the product
| Opportunity | Step | Size |
|---|---|---|
| Client grouping + Present mode (hide others/emails) `[inferred]` | Daily | M |
| Launch confirm when profile ≠ project's usual account (F1.4) | Daily | S |
| Redaction preview as wizard step 2, default-on export (F1.6) | Handover | M |
| Per-client budget bars on shared quota (no cross-move) | Wall | M |
| One-page provenance PDF (account/repo/spend/lineage) | Advocate | M |
### Jonas — small tank, big heart
| Opportunity | Step | Size |
|---|---|---|
| Weekly-window forecast equal to 5h in UI (not Max-only) | Wall | S |
| "Sessions remaining" translation for Pro/Lite tiers | Daily | S |
| Release-mode: archive-all-finished + sponsor cost summary | Ship | S |
| Conformance kit invites his adapter PR (F1.8) | Advocate | M |
| Quiet digest default; no streaks, no shame metrics | Daily | S |
### Sara — artefacts before features
| Opportunity | Step | Size |
|---|---|---|
| F4.1 decision doc + guardrails.rs (F4.2) | Onboard | M |
| Signed/notarised build + MDM note (M5) | Onboard | M |
| One-page data-flow: no proxy, vendor login, local-only | Discover | S |
| Click-audit export (who handed over, when, fidelity tier) `[inferred]` | Ship | M |
| Fleet off-by-default with netsec explainer | Onboard | S |
### Alex — translate, don't display
| Opportunity | Step | Size |
|---|---|---|
| "≈N sessions left" beside every credits/tokens number | Daily | S |
| Budget cap alert $5/$10 with cheap-tier suggestion (WOW-d) | Wall | M |
| Single-session focus view; advanced panels opt-in | Daily | M |
| Plain-language glossary inline (profile, handover, pack) | Onboard | S |
| Pro/GLM-Lite hero screenshots in README/store | Discover | S |
### Sequencing note
S-rows above are Phase 0/1 fillers with outsized persona
value (shoulder-tap, renewal report, weekly forecast,
glossary). None requires a new adapter. If capacity forces
choice, cut Fleet/review polish before any S-row: §9.7
shows trust breaks at wall/handover/onboard, and every
S-row repairs exactly one of those three steps.
## 13. What this changes vs the prior review
The prior persona map ([review:729](../full-competitive-ux-review.md#L729))
named five jobs and eight ranked pains for an undifferentiated
"5–15 sessions/day" user. This file changes three conclusions:
1. **Pain ranking is persona-conditional.** Redaction is
   rank-8 globally but existential for Priya/Sara; admission
   is invisible to Maya but P0 for Devon/Jonas/Alex on
   constrained machines. A single ranked list (prior §743)
   mis-sequences both. Use §10's max-severity column.
2. **Willingness-to-pay is artefact-conditional.** Nobody
   pays for "quota charts". Priya pays for provenance PDFs,
   Devon via employer for ROI CSV, Maya for a renewal
   report. Monetisation follows exports and trust artefacts,
   not dashboard tiers — keep core quota/forecast free or
   Alex/Jonas (the OSS contributor funnel) never arrive.
3. **Advocacy has three shapes** (story, artefact, internal
   audit pass — §9.7), so launch proof must ship in all
   three: a saved-demo story, a downloadable provenance
   sample, and the F4.1/guardrails decision doc. Testimonials
   alone persuade only Maya and Alex.
Net: Phase 1 (forecast + inbox + honest handover) is
confirmed, but its *acceptance tests* should be written as
persona scenarios — Maya's 13:00 wall, Devon's 01:14 waiter,
Priya's Friday pack, Sara's minute-14 review, Alex's demo-eve
— not as feature checklists. A feature that passes its unit
tests but fails Priya's Friday has not shipped.
## 14. Sources
Repo: [PROPOSAL.md](../../PROPOSAL.md#L49), [SPEC.md](../../SPEC.md#L86), findings F1.1–F7.4 + WOW/UX in [full review](../full-competitive-ux-review.md#L59), quota/pricing in [research/zcode.md](../../research/zcode.md#L13), [research/claude-code.md](../../research/claude-code.md#L51). Competitors: [Conductor workflow](https://www.conductor.build/docs/concepts/workflow) · [pricing](https://www.conductor.build/pricing) · [cc-switch](https://github.com/farion1231/cc-switch) · [ccusage](https://github.com/ryoppippi/ccusage) · [Claude Squad](https://github.com/smtg-ai/claude-squad).
*End — composites are design tools; never quote externally as research.*
