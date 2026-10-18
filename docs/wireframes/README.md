# Visual Wireframes — Agent & Approval Reference

> Open [index.html](index.html) in a browser. It is a single
> self-contained file (no build, no server): 27 wireframe screens
> covering every proposed change, each with Approve / Changes
> requested verdicts stored in the browser and exportable as JSON.
> The maintainer's exported `wireframe-approvals.json` is the
> approval record; agents must not treat an unreviewed wireframe as
> approved, and wireframe approval never authorises code (D-15).

## Screen → task map (canonical)

| Screen | Title | Owning tasks |
|---|---|---|
| 01 | App shell + rail indicators (+ focus filter: All/Projects/Instances/Sessions; + rail search) | AJ-E401-E404 |
| 02 | Morning triage / Needs me | AJ-E1004, AJ-E1005 |
| 03 | Projects + worktrees | AJ-E408, AJ-E1002 |
| 04 | Instances (real entity; desktop = Focus window) | AJ-E104, AJ-E401, AJ-E413-E415 |
| 05 | Session + terminal (+ maximize, + pop-out to own OS window) | AJ-E407, AJ-E1001 |
| 06 | Inspector Overview | AJ-E406-E408 |
| 07 | Inspector Usage + Quality | AJ-E409-E412 |
| 08 | Inspector Processes | AJ-E410, E5 |
| 09 | Inspector History + Actions | AJ-E406 |
| 10 | Launch sheet + Launch settings | AJ-E105, AJ-E306 |
| 11 | Switch 1-click | AJ-E206 |
| 12 | Switch wizard (lossy) | AJ-E203, AJ-E206 |
| 13 | Switch-back + divergence | AJ-E207 |
| 14 | Context pack + redaction | AJ-E202, AJ-E208 |
| 15 | Switch Graph | AJ-E201, AJ-E209 |
| 16 | Onboarding scan | AJ-E301-E303 |
| 17 | Onboarding recommended set | AJ-E304, AJ-E305, AJ-E309 |
| 18 | Add Provider/Account/Instance | AJ-E306-E308 |
| 19 | Usage two clocks | AJ-E606-E608, AJ-E610 |
| 20 | Cost ledger | AJ-E609, AJ-E611 |
| 21 | Cleanup Centre | AJ-E506, AJ-E508, AJ-E512 |
| 22 | Verify-before-kill | AJ-E503, AJ-E507 |
| 23 | Fleet pairing | AJ-E701-E704 |
| 24 | Fleet remote Inspector (+ remote Focus window) | AJ-E705-E707, AJ-E709, AJ-E711 |
| 25 | Search / Archive / Replay | AJ-E1007 |
| 26 | Settings (user-owned policies + Appearance/theme) | AJ-E208, AJ-E508, AJ-E610 |
| 27 | Palette / notifications / parked | AJ-E404, AJ-E1003, AJ-E1004 |

## Rules for agents implementing from these wireframes

1. The wireframe fixes layout, control inventory, copy and states.
   Exact colours/fonts are design-system work, not settled here.
2. Every control shown must exist in the implementation, and every
   state named in the side notes (empty/loading/error/stale/offline,
   validated/unvalidated, verified/unverified) must be reachable —
   AJ-E802/E803 visual tests capture them against these frames.
3. Where a wireframe conflicts with `docs/review/decision-log.md`
   (D-01..D-15), the decision wins; file a wireframe fix, never a
   silent deviation.
4. Changes-requested notes from the maintainer are requirements:
   quote the screen ID and note text in the task you amend.
5. After implementing a screen, attach its AJ-E803 comparison
   (wireframe intent vs built capture) in the task evidence.

## Approval workflow for the maintainer

1. Open `index.html`, review screens 01-27 top to bottom.
2. Click Approve or Changes requested per screen; add notes naming
   the exact element and wanted behaviour.
3. Export approvals (top bar) and commit or hand back the JSON.
4. Agents record the outcome in the relevant task files; disputed
   screens return here for a revised wireframe before any build.

## Global conventions (apply to every screen)

- **Dual theme (D-16):** Light + Dark, both first-class. Dark
  default; `System` follow mode; Settings → Appearance + `⌘⇧L`
  cycle. Every visual-test capture runs in BOTH themes (AJ-E402/
  E403/E804); the token layer is the only place colours live.

## Global layout conventions (apply to every screen)

- **Left rail collapsible everywhere** — ⌘B toggles between full rail
  and icon strip (state dots + type icons persist; selection kept;
  width remembered). Already shipped in the app today.
- **Right Inspector collapsible everywhere** — ⌘⇧I collapses it to a
  slim edge tab that keeps the selection label + attention badge
  visible and live; one click restores. Screens with a maximized
  terminal (05) treat maximize as the same family.
- These are chrome-level behaviors owned by the shell tasks
  (AJ-E401/AJ-E403 + wireframe 01); every other screen inherits them
  and MUST spec its collapsed states (what the edge tab shows) in its
  own states list.

## Change log

- 2026-10-13 — ALL screens (maintainer instruction, post-approval):
  **dual theme Light + Dark** recorded as decision D-16 and a global
  convention; Settings (screen 26) gains Appearance; all visual
  testing captures both themes.
- 2026-10-13 — ALL screens (maintainer request, pre-approval): rail
  and Right Inspector are collapsible on every screen (⌘B / ⌘⇧I;
  inspector collapses to a live edge tab with label + attention
  badge). Recorded as a global convention above; shell tasks own it,
  each screen specs its collapsed state.
- 2026-10-13 — Screen 05 (maintainer request, pre-approval): added
  **Maximize** (terminal fills the app window, ⌥⤢, Esc restores) and
  **Open in window** (⇧⧉) — the terminal detaches into a real OS
  window for multi-monitor users: same PTY, same scrollback, live in
  both, session keeps running when either window closes; rail badges
  external-window sessions with ↗. Owning task: AJ-E407 (extend with
  maximize + pop-out states); Tauri multi-window is the mechanism.
- 2026-10-13 — Screen 01 (maintainer request, pre-approval): added a
  **focus filter** at the top of the left navbar (All / Projects /
  Instances / Sessions — combined display preserved, other sections
  hide when focusing; ⌘\ cycles) and a **rail search bar** filtering
  rows across all three sections (⌘F focuses, Esc clears). Owning
  tasks: AJ-E402 (render) + AJ-E404 (filtering/search — extend its
  scope to include type-filter + search, not just sort).
