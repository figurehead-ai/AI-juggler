# Evidence manifest — AJ-E2803 The header: Home · Projects · Pools · Tasks · Accounts · Usage · History

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2803. The header is
`ui/src/components/AppHeader.tsx` with `MainNav.tsx` over `lib/nav.ts`'s
`PRIMARY_NAV`; the window's title is `lib/window-title.ts`; a pool's crumb is
`pages/pool/PoolHeader.tsx`. Built in 0.3 (e0645f9dc, 051cf0382, shipped in
ca2247e04) and finished on `release/0.4.1`; re-checked against the code, its
tests and the committed gallery, 2026-11-06.

Run for this manifest (one command):
`cd ui && pnpm -s exec vitest run src/app-keys.test.tsx src/lib/nav-focus.test.tsx src/lib/window-title.test.tsx src/screens/nav-screens.test.tsx src/app-orientation.test.tsx src/pages/pool/canvas.test.tsx src/components/inspector-shell.test.tsx src/pages/session/subagents-tab.test.tsx src/components/notifications.test.tsx`
→ 9 files, 136 tests passed. Playwright was not run here: its tests run in
the gallery gate, and the captures cited are the committed ones.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 `PRIMARY_NAV` is Home · Projects · Pools · Tasks · Accounts · Usage · History, in that order (D-54) | vitest `screens/nav-screens.test.tsx`: "are exactly Home · Projects · Pools · Tasks · Accounts · Usage · History"; `_gallery/97-nav/header-home-1280x800-dark.png` | that order exactly | passes; the capture shows the seven tabs in order | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R2 the header's visible `<h1>` goes; the lit tab carries `aria-current="page"`; each screen keeps a visually hidden `<h1>` with its name | `AppHeader.tsx:61` (`<h1 className="sr-only">`); vitest `screens/nav-screens.test.tsx`: "light the tab a page sits under: the lit tab is the title"; `app-orientation.test.tsx`: "Home lights only on Home; a session's page lights nothing and is titled by its project", "an account's page is under Accounts; the open project's rail heading is selected"; `app-keys.test.tsx`: "⌘⇧P, ⌘⇧O, ⌘⇧T and ⌘⇧Y open Projects, Pools, Tasks and History, and the window says so (AJ-E2803 R4, R5)" (the one `h1` is the screen's name); Playwright `nav-gallery.spec.ts` "the header at 1280" (Home `aria-current="page"`) | aria-current on the lit tab; one hidden h1 per screen | all pass | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R3 a page below a screen (a project, a pool, a session, a subagent) shows its breadcrumb in the page, each step a link, the last plain | vitest `app-orientation.test.tsx`: "an account's page is under Accounts; the open project's rail heading is selected" ("Projects" crumb on a project's page); `pages/pool/canvas.test.tsx`: "says where it sits, Pools › its name, and Pools goes back (AJ-E2803 R3)"; `components/inspector-shell.test.tsx`: "the session header carries the I-04 state chip, and its project as the crumb"; `pages/session/subagents-tab.test.tsx`: "a subagent's page: its task, result, work, usage, processes and how to stop it; Esc goes back" (crumb "Fix the cart totals › Subagents › …"); `_gallery/97-nav/project-overview-1280x800-dark.png` | a crumb on each of the four; steps link | all pass | PASS | 987a811bd | 2026-11-06 | Claude (self) |
| R4 the window's title is "AI Juggler · <screen>"; a page's by its name | vitest `lib/window-title.test.tsx`: "is AI Juggler and the screen, and follows it"; `app-keys.test.tsx`: "⌘⇧P, ⌘⇧O, ⌘⇧T and ⌘⇧Y open Projects, Pools, Tasks and History, and the window says so (AJ-E2803 R4, R5)" (and a pool's page "AI Juggler · <its name>") Walked by hand: `docs/qa/evidence/E28-walk-2026-11-07.md`. | exact title per screen | pass; walked: "AI Juggler · Projects", "AI Juggler · app-c"; the walk found every pool titled "Pool", fixed in 352f6acc6 | PASS | 352f6acc6 | 2026-11-07 | Claude (self) |
| R5 keys ⌘⇧P Projects, ⌘⇧O Pools, ⌘⇧T Tasks, ⌘⇧Y History in the registry; listed by the ? sheet, ⌘K and each tab's tooltip | vitest `app-keys.test.tsx`: "⌘⇧P, ⌘⇧O, ⌘⇧T and ⌘⇧Y open Projects, Pools, Tasks and History, and the window says so (AJ-E2803 R4, R5)", "lists exactly the registered shortcuts, by area, with their keys"; `screens/nav-screens.test.tsx`: "each has a key on the ? sheet and in ⌘K, and no two shortcuts share one", "each tab's tooltip names its key, from the one registry" | each key opens its screen; listed in all three places | all pass | PASS | 987a811bd | 2026-11-06 | Claude (self) |
| R6 Processes, This Mac and Settings stay in your menu; New session, the bell and the avatar stay on the right | vitest `components/notifications.test.tsx`: "without a Figurehead account: one quiet Connect, icons on every line, three groups" (the menu's lines include Settings, This Mac, Processes); `_gallery/97-nav/header-home-1280x800-dark.png` (New session, the bell, the avatar on the right) | in your menu; on the right | passes; seen in the capture | PASS | e0645f9dc | 2026-11-06 | Claude (self) |
| R7 the header doesn't wrap at 1024 × 700, no tab label truncates | Playwright `nav-gallery.spec.ts` "the header at 1024 with the list open" and "the header at 1280" (`oneRow`: every tab on New session's line, none with `scrollWidth > clientWidth`); `_gallery/97-nav/header-home-1024x700-{dark,light}.png` | one row; no truncation | runs in the gallery gate; the 1024 captures show one row, every label whole | PASS | 987a811bd | 2026-11-06 | Claude (self) |
| Gate: the header's tests | the vitest command above | pass | 136 / 136 pass | PASS | 987a811bd | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/97-nav/header-home-*.png`,
`_gallery/97-nav/project-overview-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
