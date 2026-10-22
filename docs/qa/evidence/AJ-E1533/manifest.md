# Evidence manifest — AJ-E1533

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1533, one page model (a 0.1
P0, from the coordinator's screen-by-screen review against wireframes
01–09, and QA's P21). Status: **IN-REVIEW** until the release PR.

Gallery: `docs/qa/evidence/_gallery/79-pages/` holds every type's page on
the replica, at 1280 and 1024, in both themes (`pages-gallery.spec.ts`,
axe clean on each). Before: `before/` holds the same pages from
cc1e9321's visual baselines, with six fixed tabs on every page.

## What exists now

**One shell for every item type, registered per type** (ADR-0019 §3:
`ui/src/pages/registry.ts`, `ui/src/pages/PageShell.tsx`). Every page
is an identity header, then the type's own tabs:

- **The header** (`pages/ItemHeader.tsx`, `PageHeader`):
  - the type in words (Session, Project, Account, Tool);
  - the parent as a crumb that opens it: a session's project, an
    account's tool;
  - the type's own mark (AJ-E1534) and its name;
  - its state, and two or three facts of its type;
  - its actions, then the item's ⋯ menu (the AJ-E1537 registry).
- **The tabs**: the body comes first, and a tab shows only when it has
  something to show. No type has an Actions tab any more: actions live
  in the header and its ⋯ menu.

| Type | Body | Other tabs | Header facts and actions |
|---|---|---|---|
| Session | Terminal (Window for a desktop app's) | Usage, with Quality merged in under the tokens; Processes; History only once there are events | account · started · % left when low; CPU · memory · helpers once measured. Switch account…, Stop (live) or Relaunch (ended), ⋯ |
| Project | Sessions: AI Juggler's and the ones it didn't start, across accounts and tools, live first, then by last activity | Overview (sessions ever, last worked, accounts, checkouts with the shared-checkout warning, which account starts work here, how the work travelled); Usage by account and tool, once anything is measured | the path (`~/…`), the branch, ahead/behind and files not committed, read from the root by id (`projects_git_status`). Open in Finder/editor, New session here, ⋯ |
| Account | Overview (limits, renewal date, cleanup, what runs now) | Sessions, once it has any | tool · signed in. ⋯ (start a session, sign in, …) |
| Tool | Overview (accounts, version, quota per account) | none, so no tab strip | N running. Add account…, ⋯ |
| Outside session | its facts | none | account · last activity. Continue in AI Juggler, Copy resume command, ⋯ |

The account page opens from the account's **name** on Accounts. The
tool page opens from an account's crumb and from **More › Tools**
(`MoreMenu.tsx`).

**One source per fact.** The four disagreements in the review are gone:

| Before | Now |
|---|---|
| Header "2 helpers", Processes "No subprocesses" | Both read the same session. The Processes tab says "Reading the process tree…" while helpers are counted but no tree has come yet. The preview's process trees now match their sessions (`lib/preview-processes.ts`) |
| Header "CPU 4%", Quality "collecting" | Usage's "CPU and memory now" is the header's own live line (`resourceLine`), not a one-off sample that was never refreshed |
| Project Usage "none" while its session had 19k tokens | Project Usage adds up its sessions' tokens: AI Juggler's, and the ones it didn't start (an AI Juggler session's own conversation counts once) |
| Project Processes "0 sessions" while 1 ran (`cwd == root`) | That rollup is gone with its tab. A project's sessions are found by `project_id`, as everywhere else |

Found and fixed on the way:
- A session's state chip used other words than the rail ("Quota
  blocked" vs "Hit its limit"). Both now use the rail's words.
- A session's name ignored its rename. `sessionTitle` is the one rule.
- A tool counted only AI Juggler's sessions as running. Outside ones
  count too (AJ-E1529).
- The project page's "Sessions 26" sat beside the rail line's "23
  conversations". The page keeps its own count, and the rail line
  became "Last worked".
- P21: "most left first" shows only once some headroom is known. Each
  type's Overview footer names its own source.

**Smaller and in modules**:
- RightInspector.tsx goes from 3102 to 2153 lines, with the project page
  in `pages/ProjectPage.tsx`, the header in `pages/ItemHeader.tsx` and
  project usage in `pages/ProjectUsage.tsx`.
- App.tsx goes from 1315 to 1268 (the More menu is its own component;
  first run's state is in `lib/first-run.ts`).
- The caps are lowered.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 one shell, registered per type; the identity header | `pages/registry.ts`; vitest `inspector-shell` ("a Session's/Project's/Account's/Tool's page: its type, its name, and only the tabs with something to show"; the session's crumb opens its project) | PASS |
| Req.2 per-type tab sets, no empty tabs | vitest `inspector-shell` (the tab lists per type; a project's Usage appears once something is measured), `inspector-instance` (no History or Actions tab), `desktop-surface` (Window, not Terminal) | PASS |
| Req.2 Session: terminal body, Usage with Quality, Processes, actions in the header | vitest `inspector-instance` (Switch account… live, Relaunch ended), `quality-tab` and `processes-tab` through Usage and Processes | PASS |
| Req.2 Project: header path, branch, ahead/behind, dirty; Open in Finder/editor; New session here; sessions body; Overview; Usage | Rust `tests/project_git.rs` (branch, changed and untracked by id; not a checkout: none; unknown id: refused); vitest `inspector-shell` (header words from the git state; the body lists sessions live first); `inspector-aggregate` (Overview's accounts ranked, missing folder → Relocate…, empty CTA, the history line) | PASS |
| Req.2 Account and Tool pages from their names and More › Tools | `pages-gallery` (account from its name, tool from More › Tools); vitest `app-orientation` (account from its name) | PASS |
| Req.3 one source per fact | the table above; vitest `quality-tab`, `processes-tab`, `inspector-shell` (project usage total from its session) | PASS |
| Every page at 1280 and 1024, both themes, on the replica | `_gallery/79-pages/` (28 captures, axe 0 critical/serious); `before/` (16) | PASS |
| IPC, size budget, layering, tool literals | `ipc_audit.py` (projects_git_status called), `size_budget.py` (caps lowered), `check_layering.py`, `tool_literal_lint.py` | PASS |

## Walked on the replica: the three questions

Walked on the preview and on the replica gallery: session, project,
account, tool and outside-session pages, by mouse and by the crumbs.

1. **Does it solve the real problem?** Someone with several
   subscriptions opens any item and sees, in one place, what it is, where
   it belongs and what they can do with it.
   - A project now lists every session that worked there, AI Juggler's
     and their own terminal's, across accounts, live first.
   - Its header says the branch and what isn't committed, and its Usage
     says what each account spent there. Before, it showed "none".
   - A session's numbers agree from the header to every tab.
2. **Is it clear without explanation?**
   - Every page names its type and its parent, so "where am I" never
     needs the rail.
   - No tab opens onto "Unavailable".
   - Actions sit where the eye goes first, the header, and a dead
     instruction ("relaunch from Actions") became a Relaunch button.
3. **Is it consistent with its siblings?**
   - All five types share one header, one tab strip, one footer and one
     menu. Session rows look the same on a project's page, an account's
     page and Home (the same disc, tool mark and words).
   - One disagreement remains, and it waits on #120 (instructed: no
     building on `runwayOf` until it merges). The tool page's per-account
     quota is the lowest any session reported, while Home's Runway reads
     the account's own (`runwayOf`).
     - Seen on the preview: Work Max "0% left" on the tool page against
       "58% left" in Runway.
     - Once #120 is in, the tool page reads `runwayOf`, together with
       AJ-E1530.

## Decisions

- **Single column, not the wireframe's side inspector.** Wireframes 03
  and 05 show the body with a collapsible inspector beside it. The app
  had already moved to one column (header, then tabs), and at 1024 wide
  a terminal beside a panel is cramped. So the body is the first tab,
  and the other tabs sit beside it. ⌘⇧I still folds the page to its
  header.
- **Removed** (each reachable elsewhere):
  - the Actions tabs (header and ⋯ menu);
  - the conversation-record panels (only ever shown in tests; the
    outside session's page replaces them);
  - the account and project History tabs (the Sessions lists);
  - the project Processes rollup (wrong by construction);
  - `aij:edit-renewal` (the editor is on the account's body).

## Not here

- Rules & MCP on a project (AJ-E1507) gets its tab when it exists.
- Project marks with monograms and the OKLCH palette (AJ-E1532), and the
  plain words in "How this work travelled" ("transplant F1",
  "switch-back ↻": AJ-E1535).
