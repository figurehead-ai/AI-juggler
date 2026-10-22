# Evidence manifest — AJ-E1532

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1532, Telling projects and
sessions apart (P1), with the maintainer's account avatars (Req.4) and
QA's note from the real setup (labels that share a first word gave
near-identical initials). Status: **DONE-EVIDENCED** (IN-REVIEW until
the release PR).

Gallery: `docs/qa/evidence/_gallery/80-marks/` (the rail with a folded
project, History, the mark sheet before and after an emoji), and
`_gallery/78-outside/` (Home and the rail), both themes, on the replica's
16 projects.

## What exists now

1. **A project's mark, decided by the backend and served as data**
   (ADR-0019). `ProjectInfo.mark` (`features/projects/mark.rs`) carries
   its letters, its swatch, its own icon's version, what it shows (icon,
   letters or emoji), its clone number and the user's choice.
   - **(a) Its own icon** (`features/projects/icon.rs`), found read-only.
     The places, in order: a Tauri app's `src-tauri/icons/` or
     `crates/<app>/icons/`, an Electron app's `build/icon.png` (and
     similar), an Xcode `AppIcon.appiconset` (its largest image that
     fits), `public/favicon.*`, `logo.*` or `icon.*` at the root, and a
     logo under `.github/`.
     - Only a regular file (never a link) of at most 512 KB, whose first
       bytes say PNG, JPEG, GIF, WebP, ICO or SVG, is taken. An SVG with
       a script or foreign content is refused. Anything else is passed
       over for the next place.
     - The UI asks for an icon by project id only
       (`projects_icon_read`). It gets only a file the list found, as a
       `data:` URL drawn in an `<img>` (the CSP allows only `data:`
       images).
   - **(b) Otherwise, a monogram on a colour.** A two-word name gives
     its initials ("AW"), a one-word name its first two letters ("Pa"),
     split at camel case ("AIJuggler" → "AJ").
     - Letters are kept apart: a later project whose first choice is
       taken gets its next (data-pipeline "DP", docs-portal "DO").
     - Swatches go to repositories in the order AI Juggler first saw
       them, so the first twelve never share one and a project keeps its
       colour as others come and go.
   - **The user's choice** ("Change its mark…" in every project menu,
     AJ-E1537): one of the twelve colours, letters or an emoji; "Use its
     own" goes back. It is kept in the app's settings
     (`projects.mark.<id>`; no migration), checked in the user's words.
     Clones share the change.
2. **Clones of one repository share its mark, with a small number** (in
   the order they were first seen). The rail's clone group shows the
   shared mark.
3. **The palette** (`lib/palette.ts`, `lib/oklch.ts`): twelve OKLCH
   swatches 30° apart. Each hue's lightness is solved for one luminance,
   so every swatch is as light as the others to the eye, and its chroma
   is as much of 0.13 as fits in sRGB. A swatch is the same colour in
   both themes, with white letters.

   | Swatch | CSS | white letters | darkest surface (dark) | lightest surface (light) |
   |---|---|---|---|---|
   | Blue | oklch(0.555 0.130 265) | 4.84 | 3.25 | 4.02 |
   | Green | oklch(0.537 0.130 145) | 4.84 | 3.25 | 4.02 |
   | Magenta | oklch(0.567 0.130 325) | 4.84 | 3.25 | 4.02 |
   | Cyan | oklch(0.541 0.090 205) | 4.84 | 3.25 | 4.02 |
   | Violet | oklch(0.562 0.130 295) | 4.84 | 3.25 | 4.02 |
   | Olive | oklch(0.545 0.120 115) | 4.84 | 3.25 | 4.02 |
   | Azure | oklch(0.546 0.112 235) | 4.84 | 3.25 | 4.02 |
   | Teal | oklch(0.538 0.099 175) | 4.84 | 3.25 | 4.02 |
   | Pink | oklch(0.568 0.130 355) | 4.84 | 3.25 | 4.02 |
   | Orange | oklch(0.561 0.130 55) | 4.84 | 3.25 | 4.02 |
   | Gold | oklch(0.553 0.110 85) | 4.84 | 3.25 | 4.02 |
   | Red | oklch(0.566 0.130 25) | 4.84 | 3.25 | 4.02 |

   Neighbouring hues are at least ΔE 0.0495 apart in OKLab (0.02 is
   about a just-noticeable step). Red, orange and gold, the app's error
   and warning colours, are given out last.
4. **Account avatars** (`components/Avatar.tsx`): round, in the account's
   colour, with 1–2 initials and its name as a tooltip; decorative.
   - **Initials that tell accounts apart.** Labels that begin with the
     same words are known by the words after them: claude-work "Wo",
     claude-personal "Pe", claude-client-acme "Ac", claude-max-2 "M2".
     Where two still meet (claude-work and codex-work), the later takes
     its next ("CW"). This was QA's note from the real setup.
   - **Where:** every session row (the rail, Home, History, a project's
     and an account's lists), beside the tool's mark, following the
     account serving it; the Accounts cards; the account page's header;
     Home's Runway.
   - **Project rows:** up to 3 avatars of the accounts that worked there
     (its sessions, the conversations started outside AI Juggler, its
     imported history), live ones first with a ring in the live colour,
     then "+N".
   - **In words:** a rail session row is named "…, Claude Code on Work
     Max"; a project row "…, on Personal (running), Work Max and Old
     Pro"; Home's and History's rows "on Work Max".
5. **Session rows show the project mark, or a coloured stripe when a
   group is folded.**
   - Lists not grouped by project show the mark on every row: Home,
     History, and the rail grouped by account or tool, or flat.
   - In the rail's project groups the mark heads the group, and isn't
     repeated on each row under it.
   - A folded project group shows its rows as a stripe in its colour
     under its heading.
6. **One set of components** (D-25): `ProjectMark`/`MarkTile`,
   `AccountAvatar`/`AccountStack`, `ToolMark`. The old account disc
   (`AccountDisc`, `.acct-disc`) and the HSL hues are gone.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1(a) icons found read-only; oversized and odd files refused | unit `features::projects::icon` (every kind in order; app icons first; an oversized file, a fake image and a link passed over; magic bytes for six types; SVG with script refused; a data URL only while still an image); integration `tests/project_marks.rs` (through `projects_list_impl` and `icon_read_impl`, an unknown id gets nothing); the replica: design-system (600 KB logo) and cli-tools (text "logo") show letters, the other icons show | PASS |
| Req.1(b) monogram on a palette of ~12, AA both themes, stable | unit `features::projects::mark` (initials, camel case, collisions, the first twelve distinct and kept when a 13th comes); vitest `palette` (AA white on all 12, sRGB, neighbour ΔE ≥ 0.045, warm hues last) and `tests/bundle/palette-surfaces` (3:1 against all four surfaces of each theme, read from styles.css) | PASS |
| Req.1 the user's override (colour, emoji, letters) | unit `a_choice_is_checked_in_the_users_words`; integration `a_choice_is_kept_for_every_clone_refused_in_words_and_undone`; vitest `project-marks` (the sheet keeps a colour and an emoji, letters set the icon aside, an unchanged sheet keeps nothing, "Use its own", a refusal in the sheet); gallery `sheet`, `sheet-emoji`, `rail-emoji` | PASS |
| Req.2 rows show the project mark (or a stripe when folded), the tool mark and the account | vitest `project-marks` (a rail row's mark only outside project groups, its tool and avatar); gallery `rail-folded` (the stripe), `history`, `78-outside/home-rail` | PASS |
| Req.3 clones share a mark with a number | unit `clones_share_a_mark…`; integration (two remotes, one repository; numbered 1, 2; one icon served to both); replica acme-web 1/2/3 one swatch | PASS |
| Req.4 avatars: session rows, project stacks (≤3, +N, live ring), cards, headers, accessible names | vitest `project-marks` (avatar, stack, `projectAccounts`, the rail's group heading names its accounts), `palette` (initials, QA's labels), `outside-conversations`; gallery | PASS |
| Baselines with 15+ projects | `marks-gallery` reads the replica fixture (16 projects, 10 icons) and checks each project's mark first | PASS |
| Walked on the replica | below | PASS |

The replica (AJ-E1531) now draws every kind of icon as a small, real
image (Tauri, Electron and Xcode app icons, a favicon, a root logo, a
`.github` logo), next to the oversized logo and the fake one. Its
self-test checks them. `just replica-fixture` records the icons as the
backend serves them, and only when it asks: a pass over real data
records none. Its selftest's missing `large_rollout_mb` (from QA batch
10) is fixed.

## Walked: the three questions

Walked on the preview with the replica's fixture (16 projects, 9
accounts, 345 conversations, 8 live), both themes, at 1280×800.

1. **Does it solve the real problem?** Fifteen-plus projects read apart
   before their names: the rail's headings, Home's Running list and
   History each carry the project's mark, so a scan of the colours and
   icons finds acme-web2 or payments-api. The account serving each
   session is a coloured avatar beside the tool, so "which account is on
   this?" is answered on the row. The walk caught "CW" twice
   (claude-work and codex-work), now "Wo" and "CW", and "Ai" for
   AIJuggler, whose letters came from its lower-cased remote; it now
   gets "AJ".
2. **Is it clear without explanation?** The mark is a rounded square and
   the avatar is round, so a project and an account never read as one.
   A clone's number sits on its mark. The sheet shows the mark as it
   will be, and says clones change with it.
3. **Is it consistent with its siblings?** One component per kind on
   every screen; the sheet follows the rename and account sheets (the
   same modal, radio rows and buttons); the action sits in the
   project's one menu (AJ-E1537), after Rename.

## Not in this task

- The menu bar's avatars come with the menu bar (the spec's "later").
- A project page's header shows the project's mark, not its accounts'
  stack: its Sessions tab lists them by account.
