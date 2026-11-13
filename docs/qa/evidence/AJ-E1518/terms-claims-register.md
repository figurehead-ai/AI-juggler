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

## ZCode (Z.ai)

Read on the vendor's own sites on 2026-11-13 (AJ-E1605): ZCode's User
Agreement (`ZCode用户协议`, Chinese, updated 2026-06-15; research
`docs/research/zcode.md` §A3 read it first on 2026-10-12) and the GLM
Coding Plan's Usage Policy on docs.z.ai (no date shown).

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| Accounts registered or used improperly as several may be frozen or closed | zcode.z.ai/cn/terms (§3.2) · 2026-11-13 | Verified-primary | "如有证据证明…您存在不当注册或不当使用多个ZCode账户的情形，ZCode可采取冻结或关闭账户…" (if you improperly register or use several ZCode accounts, ZCode may freeze or close them) |
| A GLM Coding Plan may not be shared | docs.z.ai/devpack/usage-policy · 2026-11-13 | Verified-primary | "Account sharing or multi-user access is prohibited." |
| A GLM Coding Plan may be used only in officially supported tools | docs.z.ai/devpack/usage-policy · 2026-11-13 | Verified-primary | "GLM Coding Plan may only be used within officially supported tools and products." |
| No one-account-per-person rule | zcode.z.ai/cn/terms, docs.z.ai/devpack/usage-policy · 2026-11-13 | Negative-evidence | Not found by this search: both read in full; only *improper* use of several accounts is named |

## Muse Code (Meta)

Read on Meta's own site on 2026-11-13 (AJ-E1608): the Meta Model API Terms
of Service ("Last updated: October 9, 2026"). They don't name Muse Code;
they call it a "coding harness" under a "Coding Harness Subscription". The
subscriptions page says the subscription "works only through the Muse Code
CLI" (research `docs/research/muse-code.md` §7).

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| A subscription's credential is for use only with its coding harness | dev.meta.ai/legal/terms-of-service (§3.2) · 2026-11-13 | Verified-primary | "That key or credential is for use only with the coding harness under your Coding Harness Subscription" |
| An API key is never shared | dev.meta.ai/legal/terms-of-service (§3.1) · 2026-11-13 | Verified-primary | "You may not share, sell, or transfer your Meta Model API key with any third party without our prior written permission." |
| Rate limits and usage quotas are never evaded | dev.meta.ai/legal/terms-of-service (§10.1(xii)) · 2026-11-13 | Verified-primary | Users may not "circumvent, violate, hack, disable, or evade rate limits, usage quotas, content filters, access controls" |
| No one-account-per-person rule | dev.meta.ai/legal/terms-of-service · 2026-11-13 | Negative-evidence | Not found by this search: the terms read in full name sharing and evasion, not several accounts of one person |

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
- **ZCode**, linking ZCode's User Agreement: "ZCode's User Agreement lets it freeze or close accounts registered or used improperly as several, and Z.ai's GLM Coding Plan policy says not to share an account."
  - The ZCode table's rows 1 and 2.
- **Muse Code**, linking the Meta Model API Terms of Service: "Meta's Model API terms say a subscription's credential is for use only with its coding harness, an API key is never shared, and rate limits and usage quotas are never evaded."
  - The Muse Code table's rows 1 to 3.

## How a tool's sessions run (D-40)

Read on the vendors' own pages on 2026-10-25 (Anthropic's by fetching it,
OpenAI's app-server page through its learn.chatgpt.com redirect); the full
quotes, with Wayback history, are in `docs/research/session-runtime.md`
§2.1 and §2.2. Re-checked at each release (`docs/RELEASING.md` step 3).

| Claim | Source · accessed | Label | Evidence |
|---|---|---|---|
| An end user may sign in to the unmodified Claude Code binary with their own subscription | code.claude.com/docs/en/legal-and-compliance · 2026-10-25 | Verified-primary | It does not "prevent an end user from signing in to the unmodified Claude Code binary with their own Claude subscription" |
| `claude -p` is the Agent SDK through the CLI | code.claude.com/docs/en/headless · 2026-10-25 | Verified-primary | "This page covers using the Agent SDK via the CLI" |
| A local or open-source app may use Codex's app-server with Codex's own sign-in; a commercial or hosted service may not | learn.chatgpt.com/docs/app-server · 2026-10-25 | Verified-primary | App-server authentication "has never been permitted for commercial or hosted services" |

The Privacy page shows each line under the tool's terms, word for word,
from each adapter's `terms_note` (the test
`every_tool_names_its_terms_and_the_preview_matches` checks it):

- **Claude Code**: "Its sessions run in their terminal. Run without a terminal (Beta) runs your own Claude Code with your own sign-in, which Anthropic's terms allow; Anthropic may count it as Agent SDK use."
  - The table's rows 1 and 2.
- **Codex**: "Its sessions run through your own Codex's app-server, with your own sign-in: OpenAI lets a local or open-source app do that, never a commercial or hosted service."
  - The table's row 3.
- **ZCode**: "Its sessions run on ZCode's own engine, started the way ZCode's desktop app starts it; AI Juggler hands it no credential."
  - ZCode's section above (its User Agreement and the GLM Coding Plan policy); the runtime is the engine's own `app-server`, run as ZCode's desktop app runs it (AJ-E1605 round 3, `docs/research/zcode.md` § E).
- **Muse Code**: "Its sessions run through your own Muse Code's session server (muse serve), with that account's own sign-in: Meta's SDK is made for that, and AI Juggler never takes Muse's credential."
  - Muse Code's section above (§3.2: the subscription's credential stays with Muse Code itself, which is what runs); Meta's MIT SDK (`meta-models/muse-code-sdk`) exists to "programmatically control Muse Code" over this server (D-105: running the tool's own CLI for the user, under the user's own sign-in). One Muse account per subscription, never rotated to get past a limit (§10.1(xii)).

## What AI Juggler does with them

- Add account shows one line per tool, with its terms linked (`terms` in
  `claude_code_spec()` and `codex_spec()`; checked 2026-10-23).
- Every switch starts from a click, on this Mac or on a Mac the user
  paired (`trust.rs` `nothing_switches_on_its_own`).
- Using several of your own accounts to get around a vendor's limits is the
  user's responsibility under their terms. AI Juggler doesn't do it for them.
