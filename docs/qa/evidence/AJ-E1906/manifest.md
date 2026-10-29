# Evidence manifest — AJ-E1906

Task: `docs/tasks/E19-agent-tasks.md` AJ-E1906, links to GitHub issues and Linear (P2).
Status: **DONE-EVIDENCED** on fakes (IN-REVIEW until the release PR). 2026-10-25, claude-opus.

Design: `juggler_core::tasks::external_link` (an `https` link, one line, at most 500; GitHub issues and pull requests named `owner/repo#n`, Linear issues `ENG-123`), `crates/juggler-app/src/features/tasks/issues.rs` (the look-up), D-43 point 5. Test output: [tasks-tests.txt](docs/qa/evidence/AJ-E1901/tasks-tests.txt).

## What exists now

1. A task's link shows its name from the link itself (GitHub, Linear); any other `https` link opens as it is. `javascript:`, `http:`, `file:` and links with spaces are refused.
2. A GitHub link's title and state (open, closed, merged) are looked up **only on the user's click**, by the user's own installed `gh` (`gh api repos/<owner>/<repo>/issues/<n>`, owner and repository checked to be plain names, 10 s at most, no prompts), with `gh`'s own sign-in; AI Juggler never reads it and opens no connection itself. The title and state are kept with when, and the look-up is logged on the task. No sync.
3. Offline, signed out, or without `gh`: the task keeps its link and says it couldn't look it up.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a title and state where public or the user's `gh` login allows | `tasks.rs::the_issue_is_looked_up_only_on_a_click_and_offline_says_so` (a stand-in `gh` printing GitHub's answer: kept with when) | PASS |
| Req.1 no sync in v1 | the same test: nothing is looked up before the click; `issues::look_up` is reached only from its command (`tasks_guardrails.rs::no_agent_starts_a_session_or_picks_an_account`) | PASS |
| Gate: offline behaviour | the same test (a `gh` that fails: "offline, or gh isn't signed in", the link kept) | PASS |
| Links named and refused | `tasks::tests::links_name_github_and_linear_and_refuse_the_rest`; `issues::tests::only_plain_names_reach_gh` | PASS |
| Live: the look-up with the user's own `gh` | `docs/qa/tasks-lab.md` step 7, 2026-10-26 ([AJ-E1902/lab-2026-10-26.md](docs/qa/evidence/AJ-E1902/lab-2026-10-26.md), bug 3): gh 2.101.0 installed by Homebrew; the link showed `figurehead-ai/AI-juggler#1` and nothing was looked up before a click, but `can_look_up: false`, and a look-up said "Looking it up needs GitHub's own command line, gh, on this Mac". `trusted_executable` refuses `/opt/homebrew/bin` (`drwxrwxr-x`, group `admin`), so `issues::gh()` finds none | FAIL |
| Live: offline says so | the same step: Wi-Fi is a system setting this lab may not change, and the look-up can't run | UNPROVEN (needs the maintainer, after the fix) |
| Visual: the task with a link | gallery `pool-tasks-*` (`figurehead/shop#142`, open, its title, "looked up 20 min") | PASS |

Last updated: 2026-10-26.
