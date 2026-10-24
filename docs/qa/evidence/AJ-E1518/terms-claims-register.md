# Vendor terms register (AJ-E1518)

What each tool's terms say about one person using several accounts, behind
the line Add account shows (`ToolSpec::terms`). One row per claim, under
`docs/research/EVIDENCE-STANDARD.md`. Each page was read on the vendor's own
site on 2026-10-23: Anthropic's by fetching it, and OpenAI's in a browser,
because they refuse automated reading (403). Re-check at each release
candidate. A claim older than 90 days is stale.

## Claude Code (Anthropic)

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Pro and Max use the Consumer Terms; the API, Team and Enterprise use the Commercial Terms | code.claude.com/docs/en/legal-and-compliance · 2026-10-23 | Verified-primary | "Consumer Terms of Service - for Free, Pro, and Max users" |
| An account may not be shared or made available to anyone else | www.anthropic.com/legal/consumer-terms (effective 2025-10-08) · 2026-10-23 | Verified-primary | "You also may not make your Account available to anyone else." |
| Pro and Max limits assume one person's use | code.claude.com/docs/en/legal-and-compliance · 2026-10-23 | Verified-primary | Advertised limits "assume ordinary, individual usage of Claude Code" |
| The terms don't address one person holding several accounts | www.anthropic.com/legal/consumer-terms · 2026-10-23 | Negative-evidence | Not found by this search: the terms were read in full; no sentence on holding several accounts |
| Third-party apps may not collect, store or intermediate Claude.ai credentials or session tokens; sign-in completes through Anthropic's own flow | code.claude.com/docs/en/legal-and-compliance · 2026-10-23 | Verified-primary | It "does not prevent an end user from signing in to the unmodified Claude Code binary" |

The last row is why AI Juggler never reads, stores or moves a sign-in (the
trust page's "It never reads your passwords, tokens or API keys…" and "It
never signs in for you…"). The tool's own sign-in runs, unmodified, in a
terminal.

## Codex (OpenAI)

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| One person may have several accounts | help.openai.com/en/articles/20001068 · 2026-10-23 | Verified-primary | "You can create additional accounts", with billing kept separate per account |
| An account may not be shared or made available to anyone else | openai.com/policies/terms-of-use/ (effective 2026-01-01) · 2026-10-23 | Verified-primary | "You may not share your account credentials or make your account available to anyone else" |
| Usage limits may not be circumvented | openai.com/policies/terms-of-use/ · 2026-10-23 | Verified-primary | You may not "circumvent any rate limits or restrictions" |

## OpenCode (Anomaly)

Read for the research (`docs/research/opencode.md` §7), on the vendor's own
site, on 2026-10-21 (Terms of Service effective 2026-08-15).

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Several accounts may not be used to get around limits or billing | opencode.ai/legal/terms-of-service · 2026-10-21 | Verified-primary | Users may not use "multiple accounts to circumvent usage limits, access restrictions, billing obligations" |
| No one-account-per-person rule | opencode.ai/legal/terms-of-service · 2026-10-21 | Negative-evidence | Not found by this search (research S-2) |

## Kimi Code (Moonshot AI)

Read for the research (`docs/research/kimi.md` §7), on the vendor's own
sites, on 2026-10-21 (Terms of Service effective 2024-01-05; the Kimi
Code Community Guidelines show no date).

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Accounts registered or used improperly as several may be frozen or deleted | www.kimi.ai/user/agreement/modelUse (§Account) · 2026-10-21 | Verified-primary | "If there is evidence … that you have improperly registered or used multiple accounts, we may take measures such as freezing, deleting …" |
| A Kimi Code plan is for interactive use only | www.kimi.com/code/docs/en/kimi-code/community-guidelines.html · 2026-10-21 | Verified-primary | "Kimi Code subscriptions are for interactive use only"; "Don't use Kimi Code for non-interactive automation" |
| Several devices and tools on one account are normal use | www.kimi.com/code/docs/en/kimi-code/community-guidelines.html · 2026-10-21 | Verified-primary | Switching between devices or coding tools "is a completely normal usage pattern" |

## Grok Build (xAI)

Read for the research (`docs/research/grok.md` §5, §7) on 2026-10-21: the
Terms of Service (last updated 2026-10-12) from the Wayback Machine's copy
of the vendor's page (x.ai refuses automated reading, 403), and the X
Subscription docs on docs.x.ai.

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| An account may not be shared or made available to anyone else | x.ai/legal/terms-of-service (snapshot 20261021055255) · 2026-10-21 | Verified-primary | "You may not share your account credentials or make your account available to anyone else" |
| One weekly usage pool is shared by Grok's products and Grok in Cursor on the X Subscription | docs.x.ai/grok/faq, docs.x.ai/x-subscription/usage-limits · 2026-10-21 | Verified-primary | "one shared weekly usage pool" across Chat, Imagine, Voice and Build, and Grok models in Cursor |
| No one-account-per-person rule | x.ai/legal/terms-of-service · 2026-10-21 | Negative-evidence | Not found by this search (research NE-7) |

## What Add account says

Word for word, from each adapter's `terms` (the test
`every_tool_names_its_terms_and_the_preview_matches` checks it):

- **Claude Code**, linking Anthropic's Consumer Terms: "For Pro and Max, the Consumer Terms say not to share an account or make it available to anyone else, and Anthropic says those plans' limits assume one person's use."
  - The first part is the Consumer Terms' own; the second is from code.claude.com's Legal and compliance page (rows 2 and 3).
- **Codex**, linking OpenAI's Terms of Use: "OpenAI says one person can have several accounts, and its Terms of Use say not to share one or get around its usage limits."
  - The first part is OpenAI's help centre (row 1); the second is the Terms of Use (rows 2 and 3).
- **OpenCode**, linking OpenCode's Terms of Service: "OpenCode's Terms of Service say not to use several accounts to get around usage limits, access restrictions or billing."
  - The OpenCode table's row 1.
- **Kimi Code**, linking Kimi's Terms of Service: "Kimi's Terms of Service let it freeze accounts registered or used improperly as several, and its Kimi Code guidelines say plans are for interactive use only."
  - The Kimi Code table's rows 1 and 2.
- **Grok Build**, linking xAI's Terms of Service: "xAI's Terms of Service say not to share an account, and its usage pool is shared with Cursor and Grok on the X Subscription."
  - The Grok Build table's rows 1 and 2.

## What AI Juggler does with them

- Add account shows one line per tool, with its terms linked (`terms` in
  `claude_code_spec()` and `codex_spec()`; checked 2026-10-23).
- Every switch starts from a click, on this Mac or on a Mac the user
  paired (`trust.rs` `nothing_switches_on_its_own`).
- Using several of your own accounts to get around a vendor's limits is the
  user's responsibility under their terms. AI Juggler doesn't do it for them.
