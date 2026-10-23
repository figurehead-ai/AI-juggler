# Evidence manifest — AJ-E1518

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1518, a trust page and vendor terms on Add account (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/84-privacy/`:
- the page;
- a line opened to its tests;
- the tools' terms;
- Add account with the terms line.

Each capture is in both themes, with axe showing 0 critical or serious issues.

## What exists now

1. **One list of claims**, `ui/src/lib/trust-claims.json`, in four groups: what AI Juggler reads, what it changes, what it sends, and what it never does. Each claim names its tests (`path::test`).
   - **In the app:** More › **Privacy** (`screens/PrivacyScreen.tsx`, the `privacy` screen in `lib/nav.ts`). Settings › Privacy links to it with "What AI Juggler reads, changes and sends ›", through a new `aij:open-screen` event any card can use.
   - **In the docs:** `docs/guide/trust.md` ("What AI Juggler touches"). The README's privacy section and the guide index link to it.
   - **The tests stay behind each line** (D-25). Each is linked to its file, so a security-minded reader can check it.
2. **Every claim maps to a test, mechanically** (`crates/juggler-app/tests/trust.rs` `every_trust_claim_names_tests_that_exist`).
   - Each named test must exist, as a `fn` with a test attribute, in the named file.
   - Every claim must name at least one.
   - `docs/guide/trust.md` must say each claim word for word, under its group's heading.
3. **New guarantees, where no test held a claim before** (same file). Each was proven able to fail by planting a violation:
   - **`only_the_listed_crates_can_reach_the_network`:**
     - Only three crates may ship a network crate (an HTTP client or server, TLS, WebSocket, or Tauri's http, updater, websocket or upload plugins): juggler-account (Figurehead and stats), juggler-link (paired Macs) and juggler-app (the updater, and the test bridge's local server).
     - juggler-app's `hyper` and `hyper-util` must stay optional.
     - *Planted:* `ureq` added to juggler-core fails it.
   - **`the_app_reaches_only_its_own_addresses`:**
     - The account goes to `figurehead.ai`, stats to `stats.figurehead.ai`, and the feed to `github.com/figurehead-ai/`.
     - No workflow, script or the justfile sets the updater, crash or account address variables. So shipped builds have no crash-report address, and the page says "This version can't send them anywhere."
   - **`the_window_can_reach_nothing_but_the_app`:**
     - The CSP's `connect-src` is exactly `ipc: http://ipc.localhost`, and the Dev and Nightly configs don't override security.
     - No capability grants an `http:`, `shell:`, `fs:`, `websocket:` or `upload:` permission.
     - *Planted:* adding `https://api.anthropic.com` to `connect-src` fails it.
   - **`ai_juggler_never_points_a_tool_at_another_address`:**
     - No shipped source names a provider-address or proxy variable (`ANTHROPIC_BASE_URL`, `OPENAI_BASE_URL`, `HTTPS_PROXY` and the rest).
     - A value the user types into a launch setting's environment is theirs.
     - *Planted:* `"HTTPS_PROXY"` in juggler-supervisor fails it.
   - **`nothing_switches_on_its_own`:**
     - Every call that moves work (the handover, switch, undo, switch-back and Fleet executors) is made from another of them or from one of six window commands, each a `#[tauri::command]`.
     - The scan skips `#[cfg(test)]` modules by brace matching.
     - *Planted:* a call to `switch_execute_impl` inside the usage pipeline fails it.
   - **`every_tool_names_its_terms_and_the_preview_matches`:**
     - Every registered adapter names its terms.
     - The preview catalog carries the same words.
     - The register records each URL and its check date.
4. **The tools' terms** (`ToolSpec::terms`: name, URL, a line on several accounts, the day it was checked).
   - Each adapter supplies its own (`claude_code_spec`, `codex_spec`). The unused `terms_notes` is gone. The DTO is `ToolTermsInfo`.
   - The words come from the register (`terms-claims-register.md`). Every row is `Verified-primary`, except one `Negative-evidence`: Anthropic's terms say nothing on holding several accounts.
   - Anthropic's pages were fetched. OpenAI's refuse automated reading (403), so they were read in a browser on the same day.
5. **Add account shows the line**, with the terms linked (`TermsLine` in `screens/FirstRunSteps.tsx`).
   - It shows on the bring-in, fresh-account and folder steps, below the step: one line per tool being added.
   - To make room, `StepMarks` moved out of FirstRun.tsx; its cap went from 1343 to 1308.
   - App.tsx's six scrolling screens now share `ScrollingScreen`. Its cap went from 1232 to 1228, with the Privacy route in.

## Corrections the work made to earlier wording

- **Crash reports** are not "never sent": the setting's default is off. This version can't send them at all, because no build carries the address. The page says that, and a test fails if a build ever does.
- **Codex's `auth.json`** is read for its shape (which fields exist, and the account id), never for its token values (`no_sign_in_file_is_read`). The claim says "never reads your passwords, tokens or API keys", which that covers.
- **Uninstall:** "Prepare to uninstall" takes AI Juggler's setting out and keeps the user's later edits. It isn't always a byte-for-byte restore.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 one trust page, in the app and in `docs/` | `PrivacyScreen`; `docs/guide/trust.md`; vitest `privacy-page` (each claim, grouped; tests behind each; Settings opens it) | PASS |
| Req.1 each line checked against a guardrail test | `trust.rs` `every_trust_claim_names_tests_that_exist`, plus the five new guarantees above (each proven able to fail) | PASS |
| Req.2 Add account links the tool's terms, with one line on several accounts | `TermsLine`; vitest "Add account shows it for the tool being added", "one line per tool…"; gallery `add-account-*` | PASS |
| Gate: every trust claim maps to a test | the same test, and vitest "every claim names at least one test" | PASS |
| Vendor terms labelled per AJ-E810 | `terms-claims-register.md`; `trust.rs` `every_tool_names_its_terms_and_the_preview_matches` | PASS |
| Visual | gallery `84-privacy`, both themes, axe 0 critical/serious; regression baselines re-rendered (see the PR) | PASS |

## Walked by hand

In the browser preview (1280×800, dark), on 2026-10-23:
- More › Privacy opened the page.
- A line opened to its four tests, each a link.
- The terms section showed both tools.
- Accounts › Add account showed the terms line.

The walk found two things, both fixed:
- **The terms line sat above the step's heading**, so it read as a warning before the task. It now sits below the step, under a hairline.
- **The terms link was told apart from the text by colour alone** (axe `link-in-text-block`). Inline links on the page and the line are now underlined.

## Follow-ups

- **The terms register is stale after 90 days** (`scripts/research_staleness.sh`). Re-check it at each release candidate.
- **`docs/guide/trust.md` repeats the list by hand.** The test keeps it word for word with the app's list, so the two can't drift.
