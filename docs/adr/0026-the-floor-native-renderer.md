# ADR-0026: The Floor is a native wgpu window inside the app, fed by the backend

**Status:** Accepted
**Date:** 2026-11-03
**Deciders:** the maintainer (rulings of 2026-11-03, D-67)

## Context

The Floor shows every App, session, subagent, Account and This Mac's load as an office seen from above: low-poly 3D drawn as pixel art, one area per App, people at desks whose poses are the sessions' states (SPEC §9.7, D-67). It is meant to stay open all day beside the main window, so what it costs while open matters as much as how it looks.

Three rounds were measured on the maintainer's kind of Mac (`docs/research/floor/`, the spikes on the branch `proto/floor-office-2`):

| Round | What | At rest | Animating, 8 fps | Covered or minimised |
|---|---|---|---|---|
| Spike | The same scene in Canvas 2D in the webview against native wgpu | webview: several times native's CPU, mostly in WebContent and GPU | native: about 1% of one core | native: 0 frames |
| 1 | Native, flat pixel art painted at one angle | 0.05% | 0.93% | 0.15% / 0.05% |
| 2 | Native, low-poly 3D, shadows, 4,300 parts, 70 people, up to 96 lights | 0.05%, 1 frame, about 125 MB for the app | 1.8%, about 0.9 ms of CPU a frame | 0 frames after the next visibility check |

Memory follows drawing. While frames are presented continuously, about 500 MB of Metal drawables is held on top of the app's own. It is released again when drawing stops, and stayed flat over 5 minutes.

The app today:

- **Unsafe code is forbidden** in the workspace lint baseline. Only the audited shims (D-17, D-26, D-27) carry it.
- **Tauri 2.12** makes webview windows. A window without a webview exists only behind Tauri's `unstable` feature, and Tauri passes such a window no mouse, key or occlusion events.
- **The data the Floor needs lives in the backend:**
  - Apps with colour slots, sessions and their needs, other apps' conversations;
  - subagents, tasks, pools;
  - usage per Account;
  - the process table and This Mac's health.

  The UI reads the same data through commands and events.
- **Rust tests run on Linux runners with no GPU.** macOS runs only selected tests. The visual and a11y gates see only the webview.
- **macOS ships alone until Windows' (0.6) and Linux's (0.7) first releases** (D-52, D-65). Shared code must keep their seams.

## Decision

1. **The Floor is a separate native window, drawn with wgpu (Metal on macOS), in the app's own process.** It is not a page in the webview.
   - The main window stays the app. A click on the Floor opens the place it points at in the main window.
   - The Floor window is created on first open, never at start-up unless the user asked for it in Settings.
   - Closing it drops its GPU device and surface.
2. **Three new crates, so the logic is testable without a GPU and unsafe code stays in one audited place:**
   - **`juggler-floor`: pure Rust, no GPU, no platform code.**
     - The office model: cells, order, sizes, floors, saved choices.
     - The style families and kits.
     - The signal mapping from a snapshot to what is drawn.
     - The scene builder: meshes, models, textures as pixels, the instance lists.
     - It runs on every OS and in Linux CI.
   - **`juggler-floor-render`: the wgpu renderer.**
     - The passes: sun shadow; the scene at the pixel size with a pick id per pixel; glass; post (outlines, hover, sky, alert wash); the screen-space overlay.
     - The orbit camera, picking, and offscreen capture for tests.
     - Its surface comes from wgpu's safe `create_surface` with the Tauri window as an owned handle. There is no unsafe code here.
   - **`juggler-window-events`: an audited shim, the fourth carve-out from the lint baseline (after D-17, D-26, D-27).**
     - On macOS it delivers mouse, trackpad, key and visibility events for a window that has no webview: an AppKit local event monitor filtered to that window, and `occlusionState` polled at 2 Hz while drawing.
     - Windows (a window-procedure hook) and Linux (GTK signals) return `Unsupported` until their first releases implement them.
     - The crate is small, reviewed line by line, and does nothing else.
3. **The window comes from Tauri's `unstable` window builder,** with the Tauri version pinned exactly. If an update breaks it, the fallback keeps the renderer and changes only how the window is made: a webview window whose content view hosts the wgpu surface.
4. **A render thread draws on change only.** It wakes for:
   - input;
   - a camera tween;
   - a snapshot change;
   - an animation tick, only while something moving is in view and the window can be seen.

   With nothing moving, nothing is drawn. While covered or minimised, nothing is drawn. A refused frame (for example, occluded at launch) is retried shortly, never dropped.
5. **The backend feeds the Floor directly.** `features/floor` in `juggler-app` builds one `FloorSnapshot` from `AppState`'s services:
   - Apps, sessions, subagents, tasks, pools, Accounts' usage;
   - the process table, This Mac;
   - and the new inputs SPEC §9.7 lists.

   It rebuilds the snapshot when their existing events fire (`session://updated`, `processes://changed`, `machine://this-mac`, `session://quota`, `tasks://changed` and so on), coalesced to at most 4 a second. The render thread gets it over a channel. The main window's list view gets the same snapshot from a command (`floor_snapshot`) and an event (`floor://changed`), so the canvas and the list never disagree.
6. **One mapping from session data to a state, shared with the rail.**
   - The rail's state rules (`ui/src/lib/rail-items.ts`) and the Floor's Rust mapping are both tested against one fixture of cases (input, state, word).
   - The colour table (`MARK_COLOURS`) gets a Rust copy, with a test that it matches `mark-hues.ts`.
   - Neither can drift without a failing test.
7. **The tool's logo in each area's floor is drawn by the UI.** The main window draws the tool's mark as it does everywhere (`ToolMark`, or the installed app's icon via `app_icons_read` for desktop apps) with the Account's initials. It renders that to a PNG once and hands it to the backend, which keeps it in a registered local cache (ADR-0023). Until a logo arrives, or if none can be drawn, the Floor writes the letters itself. Nothing is bundled.
8. **Style families are tool data.**
   - Each adapter's `ToolSpec` (ADR-0019) names its family: Workshop, Studio, Industrial, Maker garage, Campus, Lab, or a neutral default for tools with none.
   - `juggler-floor` holds the family catalog. No code branches on a tool.
9. **The office's choices persist in the app's SQLite store.** Per App:
   - order, layout type, seed, size, and when it last fitted a smaller size.

   They go in a new `floor_areas` table. The Floor's settings live in `app_settings` under `floor.*`. Re-create replaces the office seed and every App's choice, after a confirmation.
10. **Budgets** (SPEC §9.7 has the table) are tested where they can be. A headless clock test proves that nothing is drawn at rest or while hidden. Frame and memory budgets are read on the release build with the measurement harness.

## Alternatives considered

- **Canvas 2D or WebGL in the webview, as a page.** Simplest to build and already covered by the visual and a11y gates. It cost several times native's CPU, mostly in WebKit's processes. It also keeps drawing a page the user can't see unless the page is suspended. Rejected on cost (the spike).
- **three.js in the webview.** Same cost as above, plus a large library to lazy-load.
- **A separate helper process for the Floor.** It would isolate the GPU memory from the app's budget. But it needs its own copy of the data feed or an IPC protocol, and the memory would still be the user's. Rejected: the budget is met honestly instead (point 10, and SPEC §9.7's memory budget).
- **winit directly instead of Tauri's window.** A second windowing stack beside tao inside one app. Rejected while Tauri's `unstable` builder works; it is the fallback's fallback.
- **Rasterising the tools' marks in Rust (an SVG renderer dependency).** It would duplicate `ToolMark`'s choices (which marks are allowed, letter marks) and add a dependency. Rejected for the UI-drawn logo (point 7).

## Consequences

- **New dependencies:** `wgpu` (pinned), `bytemuck` and `pollster` in `juggler-floor-render`, and `objc2-app-kit` features in `juggler-window-events`. Each is MIT or Apache-2.0, and `cargo deny` stays green. `docs/plan/layering.toml` gains the edges `juggler-app → juggler-floor`, `→ juggler-floor-render` and `→ juggler-window-events`, and `juggler-floor-render → juggler-floor`.
- **Tauri's `unstable` feature is on in the app.** Every Tauri bump re-runs the Floor's smoke test on macOS before it merges.
- **The visual and a11y gates don't see the canvas.** A Floor gallery is added: golden frames rendered offscreen with wgpu on software Vulkan in CI, compared with a tolerance, plus a Metal smoke render on the macOS job. The list view is the accessible surface and goes through axe like any page.
- **Memory is the known risk.** The drawables held while animating are investigated first (E35's second task). Until the budget is met, the Floor releases its surface after 30 seconds without a frame.
- **Windows and Linux** build and lint the three crates in CI from the first commit. The Floor's menu entry says "not available on this system yet" with the reason until `juggler-window-events` gains their implementations in E26 and E27.
