# Sign-off: AI Juggler 0.4.4 on Beta

> **Rehearsal.** Built by `signoff_record.py rehearse` from Promote run 43413923798's `beta` approval, with the scope checked at HEAD. Nothing was promoted, uploaded or committed.

D-109: the sign-off is the maintainer's approval of the `beta` environment in the Promote workflow; this record was written from that run.

| Field | Value |
|---|---|
| Version | 0.4.4 |
| Commit | [`7b527eb83aea4a524dac5c378a939fe1f4302a22`](https://github.com/figurehead-ai/AI-juggler/commit/7b527eb83aea4a524dac5c378a939fe1f4302a22) |
| Approved by | `maintainer-login` (GitHub `beta` environment) |
| Approval comment | 0.4.4 to beta, approved by the maintainer in chat on 2026-11-12 |
| Approved at | 2026-11-13T11:04:44Z (the approved job's start) |
| Recorded at | 2026-11-13T15:04:46Z |
| Run | https://github.com/figurehead-ai/AI-juggler/actions/runs/43413923798 |
| Roadmap scope | NOT complete (exit 1): 135 done and proven, 0 deferred, 4 open, 0 unproven, 0 unknown; checked at `a8ad43cf85` |

## Roadmap scope (`release_scope.py check 0.4.4 --channel stable`)

```text
release scope 0.4.4 (stable: the whole 0.4): 139 tasks named, 135 done and proven, 0 marked done but unproven, 0 deferred, 4 open
  OPEN      AJ-E3002 Labs: talk enforced per session, Claude's own channel, OpenCode and Gr — TODO
  OPEN      AJ-E3304 Pools validated: the exhaustive tables, the pairs, and P1–P11 by hand — IN-PROGRESS (L1: `crates/juggler-core/tests/pools_l1.rs` aga
  OPEN      AJ-E3306 The release candidate walked whole: every scenario again, a day in the — TODO
  OPEN      AJ-E3307 Persona-based validation: personas walk their day from first run, scor — IN-PROGRESS (the research and the protocol written, 2026-11-
release_scope: 0.4 is not complete. Finish the open tasks, or list them under '**Deferred (D-NN):**' with the maintainer's decision.
```

| Task | State | Evidence |
|---|---|---|
| AJ-E212 Move to account: a Claude → Claude switch moves the whole family (D-93) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E212/manifest.md) |
| AJ-E710 Fleet beta gate, dogfood, and security evidence pack | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E710/manifest.md) |
| AJ-E808 Provider-pair fidelity validation matrix (operations) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E808/manifest.md) |
| AJ-E1305 Live validation on real accounts and the docs | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E1305/manifest.md) |
| AJ-E1907 A task's session starts from a short pointer | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E1907/manifest.md) |
| AJ-E2001 Sign and publish separately from the build | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2001/manifest.md) |
| AJ-E2002 Agent bridge security design | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2002/manifest.md) |
| AJ-E2003 Lock down the webview: command allowlist, capabilities, navigation | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2003/manifest.md) |
| AJ-E2004 Review for sensitive paths | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2004/manifest.md) |
| AJ-E2005 The Fleet identity key in the Keychain | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2005/manifest.md) |
| AJ-E2006 Hostile files: restore and import | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2006/manifest.md) |
| AJ-E2007 Supply chain and release evidence | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2007/manifest.md) |
| AJ-E2008 Fuzzing on a schedule | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2008/manifest.md) |
| AJ-E2009 Incident readiness: key-compromise runbook, SECURITY.md, security.txt | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2009/manifest.md) |
| AJ-E2010 Process, crypto and listener hygiene | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2010/manifest.md) |
| AJ-E2011 The cheap checks: stop the debt growing | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2011/manifest.md) |
| AJ-E2012 A UI linter and stricter TypeScript | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2012/manifest.md) |
| AJ-E2013 Tracing with redaction | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2013/manifest.md) |
| AJ-E2014 Slow commands off the main thread | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2014/manifest.md) |
| AJ-E2015 ARCHITECTURE.md | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2015/manifest.md) |
| AJ-E2016 Adapter building blocks and focused traits | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2016/manifest.md) |
| AJ-E2017 Features as modules: split commands.rs and AppState | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2017/manifest.md) |
| AJ-E2018 UI: split bridge.ts, keep preview out of production, a shell registry | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2018/manifest.md) |
| AJ-E2019 Typed command wrappers | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2019/manifest.md) |
| AJ-E2101 Live sessions from the apps' own files | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2101/manifest.md) |
| AJ-E2102 Go to it | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2102/manifest.md) |
| AJ-E2103 Open first: the rail's order | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2103/manifest.md) |
| AJ-E2104 Places, rows and the View | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2104/manifest.md) |
| AJ-E2105 Home: what needs you and your apps | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2105/manifest.md) |
| AJ-E2106 Menus and ⌘K with icons and an order | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2106/manifest.md) |
| AJ-E2107 Needs you from real signals | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2107/manifest.md) |
| AJ-E2108 The session page: where it runs and its last messages | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2108/manifest.md) |
| AJ-E2109 Accounts are subscriptions; profiles sit under them | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2109/manifest.md) |
| AJ-E2110 Settings panes, docs and the Privacy page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2110/manifest.md) |
| AJ-E2111 Open starts an app or profile that isn't running | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2111/manifest.md) |
| AJ-E2112 Keep alive per app and profile | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2112/manifest.md) |
| AJ-E2301 The runtime lab on real tools | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2301/manifest.md) |
| AJ-E2302 The runtime contract | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2302/manifest.md) |
| AJ-E2303 Codex through app-server, by default | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2303/manifest.md) |
| AJ-E2304 Open in terminal, and Start in terminal | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2304/manifest.md) |
| AJ-E2305 Switching and hand-over across runtimes | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2305/manifest.md) |
| AJ-E2306 Claude terminal sessions with hooks and the status line | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2306/manifest.md) |
| AJ-E2307 Answers from Needs you and the session page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2307/manifest.md) |
| AJ-E2308 Run without a terminal: answer from AI Juggler (Beta) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2308/manifest.md) |
| AJ-E2309 Docs, Privacy and terms lines, evidence | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2309/manifest.md) |
| AJ-E2310 Driven sessions come back after AI Juggler restarts | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2310/manifest.md) |
| AJ-E2401 The process graph: one model of apps, sessions and processes | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2401/manifest.md) |
| AJ-E2402 Sightings of other apps' sessions' processes (D-41a) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2402/manifest.md) |
| AJ-E2403 The user's stop of another app's leftover (D-41b) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2403/manifest.md) |
| AJ-E2404 The Processes page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2404/manifest.md) |
| AJ-E2405 Automatic rules shown, previewed and told | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2405/manifest.md) |
| AJ-E2406 Privacy lines, guide, and the processes lab | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2406/manifest.md) |
| AJ-E2407 One source: the session tab, Cleanup and This Mac on the graph | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2407/manifest.md) |
| AJ-E2501 The one notification model | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2501/manifest.md) |
| AJ-E2502 One count: the bell, the app badge, Needs you | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2502/manifest.md) |
| AJ-E2503 The notification centre | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2503/manifest.md) |
| AJ-E2504 §2 Banner, the alert banners, and the offline banner | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2504/manifest.md) |
| AJ-E2505 §2 Toast | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2505/manifest.md) |
| AJ-E2506 The menu-bar item (§4.17) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2506/manifest.md) |
| AJ-E2507 Your menu (the avatar) | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2507/manifest.md) |
| AJ-E2801 Shared controls: choice line, checkbox, radio and picker | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2801/manifest.md) |
| AJ-E2802 The rail's grouping switch and a real View menu | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2802/manifest.md) |
| AJ-E2803 The header: Home · Projects · Pools · Tasks · Accounts · Usage · History | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2803/manifest.md) |
| AJ-E2804 The Projects screen | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2804/manifest.md) |
| AJ-E2805 The project page: Overview · Sessions · Tasks · Pool · Subagents · Usage · History | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2805/manifest.md) |
| AJ-E2806 The Tasks screen: a board and the plan across every project | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2806/manifest.md) |
| AJ-E2807 The Pools screen | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2807/manifest.md) |
| AJ-E2808 The pool canvas: nodes, edges, alerts, edit in place, an inspector and a remembered layout | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2808/manifest.md) |
| AJ-E2809 A subagent index and the adapter contract for subagent details | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2809/manifest.md) |
| AJ-E2810 The Subagents tab: running, failed and cost first, grouped by the message that started them, with tree lines | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2810/manifest.md) |
| AJ-E2811 A subagent's detail: its prompt, result, work, usage and processes | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2811/manifest.md) |
| AJ-E2812 Cross-links: the session's Usage and Processes, and the project's subagents | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2812/manifest.md) |
| AJ-E2813 The sweep's smaller fixes | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2813/manifest.md) |
| AJ-E2814 The design-quality review: a critique beside every new or changed screen, and every popover captured | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2814/manifest.md) |
| AJ-E2901 The command palette: commands with nothing selected, accounts and apps, recents, fuzzy match, a preview, a real dialog | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2901/manifest.md) |
| AJ-E2902 Cost you can act on: per finished task, wasted spend, a weekly review, top project per account, dated prices | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2902/manifest.md) |
| AJ-E2903 Quality per account: errors and retries, stuck episodes, context at finish, tokens per finished task | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2903/manifest.md) |
| AJ-E2904 Notifications tuned to you: a waiting threshold, quota warnings at 80 and 95 %, Send test | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2904/manifest.md) |
| AJ-E2905 Settings you can search, and shortcuts you can rebind with a conflict check and reset | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2905/manifest.md) |
| AJ-E2906 First run shows its confidence: Found, Likely, Guessed, where each came from, and Look again at any time | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2906/manifest.md) |
| AJ-E2907 A confirm before deleting launch settings, and no delete while they're in use | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2907/manifest.md) |
| AJ-E2908 An ended session's page: Resume, Archive, Replay and Copy ID | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2908/manifest.md) |
| AJ-E2909 A keyboard-operable rail splitter | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2909/manifest.md) |
| AJ-E2910 Home: filter chips, the r / h / s / e keys, and Mark all finished as reviewed | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2910/manifest.md) |
| AJ-E2911 Report a bug: what goes, shown first, from Help, Settings and every failure | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2911/manifest.md) |
| AJ-E2912 Debug mode: a detailed local log of switches and launches | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E2912/manifest.md) |
| AJ-E3001 Pools from real use: research, the model and a proposal for review | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3001/manifest.md) |
| AJ-E3002 Labs: talk enforced per session, Claude's own channel, OpenCode and Grok tools per launch | open | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3002/manifest.md) |
| AJ-E3003 The pool model stored: scope, member kinds, several pools, levels per member, pair rules, sealed, and today's pools moved | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3003/manifest.md) |
| AJ-E3004 Membership resolved: who is in which pool, how, and with what role and level | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3004/manifest.md) |
| AJ-E3005 Roles, levels and talk enforced: Reviewer, per-member level, allowed / ask first / blocked, sealed | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3005/manifest.md) |
| AJ-E3006 Sessions other apps run, as Watched members, and Continue in AI Juggler | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3006/manifest.md) |
| AJ-E3007 New pool opens a wizard: templates, where and who from what exists, roles and talk, a review in sentences | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3007/manifest.md) |
| AJ-E3008 Set up by hand: the canvas that teaches, member group nodes, the add palette with reasons, an edge's rule | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3008/manifest.md) |
| AJ-E3009 Pools fit everything else: the switch check, notices, mutes, the Tasks filter, usage per pool | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3009/manifest.md) |
| AJ-E3101 What relates to what: the map, each page's tabs, and one list for all of them | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3101/manifest.md) |
| AJ-E3102 Relations made cheap: instance and conversation indexes, and a relation-count cache | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3102/manifest.md) |
| AJ-E3103 One related-items list and one row, everywhere | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3103/manifest.md) |
| AJ-E3104 Every page leads to what's related: header chips and counted tabs on project, session, App, Account, pool, Mac and Tool | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3104/manifest.md) |
| AJ-E3105 Every picker: suggested for where you are, then All, with search | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3105/manifest.md) |
| AJ-E3106 Beside a session's conversation, what the tools' own apps show: subagents running, the sessions it started; and each tool's own marks | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3106/manifest.md) |
| AJ-E3201 One word per thing: Account, App, Tool, Project, Session, on screen, in the spec and in the guide | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3201/manifest.md) |
| AJ-E3202 Accounts and Apps stored: two additive tables, named, with plan, renewal and fee set once | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3202/manifest.md) |
| AJ-E3203 Claude app profiles found when they aren't running | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3203/manifest.md) |
| AJ-E3204 The App page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3204/manifest.md) |
| AJ-E3205 The Account page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3205/manifest.md) |
| AJ-E3206 Every name opens its own page | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3206/manifest.md) |
| AJ-E3207 New session: the usual choice made and said, one button, the rest a step away | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3207/manifest.md) |
| AJ-E3208 Accounts and Apps, side by side: an Apps section on the Accounts screen | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3208/manifest.md) |
| AJ-E3209 Restart a running app or profile, after a confirm | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3209/manifest.md) |
| AJ-E3301 The validation kit: scenario files, the reference model, the pairwise generator, timing and the walk sheet | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3301/manifest.md) |
| AJ-E3302 Account and App validated | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3302/manifest.md) |
| AJ-E3303 Links validated | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3303/manifest.md) |
| AJ-E3304 Pools validated: the exhaustive tables, the pairs, and P1–P11 by hand | open | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3304/manifest.md) |
| AJ-E3305 Every 0.4 screen scored against the 14 principles | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3305/manifest.md) |
| AJ-E3306 The release candidate walked whole: every scenario again, a day in the life, the load | open | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3306/manifest.md) |
| AJ-E3307 Persona-based validation: personas walk their day from first run, scored as users feel it | open | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3307/manifest.md) |
| AJ-E3308 Releases that don't stall | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3308/manifest.md) |
| AJ-E3501 The Floor window: crates, a native surface, input and the draw-on-change loop | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3501/manifest.md) |
| AJ-E3502 Memory settled: the drawables held while animating | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3502/manifest.md) |
| AJ-E3503 The office model: cells, order, sizes, floors and saved choices | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3503/manifest.md) |
| AJ-E3504 Style families as tool data, and each area's kit | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3504/manifest.md) |
| AJ-E3505 The feed: one snapshot from the backend, for the Floor and its list | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3505/manifest.md) |
| AJ-E3506 Models, textures and the scene | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3506/manifest.md) |
| AJ-E3507 The renderer: the pixel look, picking and the orbit camera | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3507/manifest.md) |
| AJ-E3508 The floor logo: the tool's mark and the Account's initials | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3508/manifest.md) |
| AJ-E3509 Signals from real data: every row of the table | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3509/manifest.md) |
| AJ-E3510 Doors and play: hover, clicks, keys and the fun touches | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3510/manifest.md) |
| AJ-E3511 Settings › The Floor, and Re-create | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3511/manifest.md) |
| AJ-E3512 The Floor list in the main window: the same facts, accessible | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3512/manifest.md) |
| AJ-E3513 The Floor's gates: a gallery of golden frames, budgets and the other systems | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3513/manifest.md) |
| AJ-E3514 Walked by hand: persona walks on the release build | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3514/manifest.md) |
| AJ-E3515 Labels by zoom: flags that stay readable, attention first | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3515/manifest.md) |
| AJ-E3516 One floor at a time: lift buttons, tab counts and the tower | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3516/manifest.md) |
| AJ-E3517 Hover cards: what they ask or do, and where to answer | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3517/manifest.md) |
| AJ-E3518 Inactive Apps: lights off, dust and dust sheets | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3518/manifest.md) |
| AJ-E3519 The View menu, top right | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3519/manifest.md) |
| AJ-E3520 Day and night from the Mac's clock | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3520/manifest.md) |
| AJ-E3521 The reworked look: light, shadow, occlusion and grade | proven | [manifest](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E3521/manifest.md) |

## Review inputs (AJ-E807 R2)

- **Accessibility (AJ-E804):** the run's accessibility gate passed on this commit before the approval (https://github.com/figurehead-ai/AI-juggler/actions/runs/43413923798).
- **Visual gallery:** 1974 captures; no earlier Stable to compare with, so review [the gallery](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/_gallery/index.html) whole.
- **Performance, measured (AJ-E805):** [the measured budgets](https://github.com/figurehead-ai/AI-juggler/blob/a8ad43cf85dfa12f9ddce2811f8de7e1988b291e/docs/qa/evidence/AJ-E805/manifest.md#measured-budgets).
- **Unresolved:**
  - OPEN      AJ-E3002 Labs: talk enforced per session, Claude's own channel, OpenCode and Gr — TODO
  - OPEN      AJ-E3304 Pools validated: the exhaustive tables, the pairs, and P1–P11 by hand — IN-PROGRESS (L1: `crates/juggler-core/tests/pools_l1.rs` aga
  - OPEN      AJ-E3306 The release candidate walked whole: every scenario again, a day in the — TODO
  - OPEN      AJ-E3307 Persona-based validation: personas walk their day from first run, scor — IN-PROGRESS (the research and the protocol written, 2026-11-
