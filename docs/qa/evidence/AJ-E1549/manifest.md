# Evidence manifest — AJ-E1549

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1549 (no white flash, ever: the window, the page and every loading boundary paint the theme's base colour from the first frame). 2026-11-23, reviewer claude-opus.

## Root cause (written before the fix)

**The white is the native web view's own background, never the page.** The main window's entry in `crates/juggler-app/tauri.conf.json` (lines 9–21) sets no `backgroundColor`, and nothing sets one at run time. wry 0.57 turns WKWebView's `drawsBackground` off only when a window asks for transparency or a background colour (`wry-0.57.0/src/wkwebview/mod.rs:371-375`), so AI Juggler's web view kept WebKit's default: an opaque white layer under the page. Every frame in which WebKit has no page content on screen shows that layer across the whole window.

WebKit has no page content on screen in these moments:

- **The window is shown again.** The close button and ⌘W hide the window (D-112, `crates/juggler-app/src/main_window.rs:200`, `:250`, `:272`, `:308`). The Dock, the menu-bar item, a notification and the go-to hotkey show it again (`main_window.rs:65`). WebKit dropped the hidden view's layers, and its first frame back comes before the page's.
- **The window comes back from under another.** On macOS 14+, WebKit's default policy suspends the page of a view nobody can see (an occluded window, another Space, a minimised window; Tauri leaves `background_throttling` at WebKit's default, see `webview_guard.rs`). The page is resumed when the window is seen again, and it repaints a frame or more late. During the wizard's sign-in step this happens every time: the tool's sign-in opens the browser over AI Juggler, and the user comes back to the live terminal. Coming back is the moment of the flash. The wizard's state survives because the page was suspended, not reloaded, which matches the system log (no WebContent crash, no reload, no WebGL context loss).
- **A resize uncovers part of the window**, for the frames before the page is laid out at the new size (only the newly uncovered part).

**The page itself never paints white, in the dark theme.** Each candidate was read in the code and then measured frame by frame:

| Candidate | Finding |
|---|---|
| A lazy part's empty Suspense fallback (`ui/src/lib/lazy.tsx:39-44`: `Pending` draws nothing when a part has no `placeholder`; 72 of the 74 parts have none) while the page's background lives on an inner element | The page background doesn't live on an inner element. `index.html:15-16` paints `<html>` with the canvas colour before any script runs, and `styles.css`'s `body` paints the canvas gradient. An empty part leaves a hole in the *layout*, but the hole is dark. It is still a gap (SK-8, PR-9: an empty box for up to 300 ms), and it is fixed in layer (c). It is not the white flash. |
| The theme applied by script after the first paint | `public/theme-boot.js:14` sets `<html data-theme>` synchronously in `<head>` (`index.html:8`), before the body is parsed. `src/theme.ts:18` only repeats it. No frame flips theme. |
| The embedded terminal's canvas re-created | `TerminalView.tsx:128-257` makes a new xterm (and, through `lib/terminal-render.ts`, a new WebGL canvas) whenever `id`, `title`, `hintId`, `away` or `say` changes (`:257`). A new canvas is transparent until its first draw, so the terminal host's own dark background shows through it. Measured in WebKit with WebGL on: no white frame. |
| The Floor's wgpu surface reconfiguring | The Floor is a window of its own, with no web view (`features/floor/window.rs:401`, `tauri::window::WindowBuilder`). Its surface can't paint in the main window. |

The frame check (`ui/tests/visual/no-white-flash.spec.ts`) ran on the tree **before the fix**, in Chromium (the gate's browser, no WebGL) and in WebKit (WebGL on, the engine inside the app). It switched nine screens, opened two live sessions' terminal panels and stepped through Add account to its sign-in terminal. It found **0 mostly-white frames** in about 460 probed frames and 180 screenshots. Its seeded test, which takes the page's background away for 250 ms, **fails it as it should**. So the flash is below the page, in the native window, where only layer (a) can reach.

## Fix

1. **(a) The native window.** `tauri.conf.json`'s main window sets `"backgroundColor": "#0b0e14"`, the dark canvas (dark is the default theme). Every window the app makes goes through `webview_guard::guarded`, which now sets `BASE_COLOUR` on it too, so popped-out terminal windows get it as well. With a colour, wry turns WKWebView's `drawsBackground` off, so a frame without page content shows the canvas, not white. At run time `ui/src/theme.ts` `applyTheme` repaints the window and its web view in the chosen theme's canvas (`#0b0e14` or `#eceef3`). It runs at start, on ⌘⇧L and Settings, and when macOS's appearance changes under "System", through Tauri's own `set_background_color` and `set_webview_background_color` commands. The main and terminal capabilities grant exactly those two permissions, and nothing new is registered.
2. **(b) The page before any script.** `ui/index.html` paints `html` and `body` in the canvas in inline CSS, before its first `<script>`. It covers both `prefers-color-scheme` cases while no theme is set, and the stored theme once `theme-boot.js` has set `data-theme`. The CSS sits in the lowest cascade layer (`@layer aij-base`), so `styles.css`'s body gradient still wins once it loads, and nothing on screen moves.
3. **(c) No empty loading boundary.** `lib/lazy.tsx` has a `PartPlaceholder` context. A lazy part with no placeholder of its own draws the area's skeleton (`ListSkeleton` rows, `AreaSkeleton`) while its code loads. Every non-root `ScreenBoundary` provides it, and so does the work area (`shell/pages.tsx` `workArea`). The Privacy redirect's one frame draws it too. The sheets and dialogs that open over a page (Keep apart, Wasted, Cost review, Replay) opt out with `() => null`. The root's overlays draw nothing, as before.
4. **(d) The trigger.** The trigger is the white web view under the page, and (a) fixes it where it is. Neither WebKit's suspend-and-resume nor a shown-again window can show white any more.

### Left for the owners of the wizard's files

- `ui/src/screens/FirstRun.tsx:71`: the sign-in terminal's lazy part sits in the wizard overlay, outside any screen, so it draws nothing for up to 300 ms on a first open. One-line fix for the wizard's owner: `const TerminalView = lazyNamed(() => import("../components/TerminalView"), "TerminalView", true, () => <ListSkeleton label="Loading the terminal" rows={4} />);` (with `import { ListSkeleton } from "../components/ListSkeleton";`).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 native background, config and run time | `tauri.conf.json` `backgroundColor`; `webview_guard.rs` `guarded` sets `BASE_COLOUR`, and its test `the_main_window_is_made_on_the_base_colour` checks the config entry against it. `theme.ts` `applyTheme` → `setBackgroundColor`. vitest `tests/bundle/theme-base.test.ts`: `BASE_COLOUR` equals `--color-canvas` in both themes, and the config's colour equals the dark one. `cargo check -p juggler-app --lib` passes. The Rust test target didn't build in the shared tree because of others' in-flight edits (`handover/moving_pick_tests.rs`), so the Rust test is UNPROVEN until it does. | PASS (Rust test UNPROVEN) |
| Req.2 `index.html` before scripts, both schemes | vitest `tests/bundle/theme-base.test.ts` (4 tests): the inline CSS before the first `<script>` gives `html` and `body` the canvas with no theme set, under each `prefers-color-scheme`, and for each stored theme; it is in the lowest layer. A seeded mismatch (light `#ffffff`) fails it. | PASS |
| Req.3 no empty fallback | vitest `src/screen-boundary.test.tsx` "a screen's part draws the area's skeleton while its code loads": a screen's part and a page's part each draw the skeleton; a sheet opted out and the root's part draw nothing. `src/lib/lazy.test.tsx` unchanged and green. | PASS |
| Req.4 root cause, fixed where it is | Above: the native white layer (`tauri.conf.json`, wry `drawsBackground`), shown when WebKit has no page content. The four page-side candidates were read in code and measured frame by frame: none paints white. | PASS |
| Gate: frame check, dark | `ui/tests/visual/no-white-flash.spec.ts` (3 tests; CI runs them in the regression config's `assertions` project). It probes every animation frame at a grid of 40 points and takes back-to-back screenshots while it switches nine screens, opens two live sessions' terminal panels, and steps through Add account to its sign-in terminal. Before the fix and after: 0 mostly-white frames in Chromium (about 310 frames and 90 screenshots) and in WebKit with WebGL (about 460 frames and 175 screenshots). The seeded test (no page background for 250 ms) is caught. | PASS |

The frame check passed **before** the fix too, by design of the finding: the white was never in the page, so a browser can't see it. The seeded test shows the check does fail on a page-side gap. The native layer is proven by configuration and its tests, not by pixels.

## Usability answers, re-checked

1. Everyone, both themes: the window and the page now share one base colour from the first frame, and it follows a theme switch. Holds.
2. Nothing to explain: no new words or surface. Holds.
3. Calm and trustworthy: a gap now shows the canvas, and loading parts show skeleton rows. Holds.
4. Every loading state is a skeleton inside the layout: true for screens, pages and their parts. The wizard overlay's terminal is left for its owner (above). Holds, except that one site.
5. None. Holds.
6. No flash left for anyone to take for a restart: the window's own layer is no longer white. On the real app this is proven by configuration and tests, not yet by eye. Holds, pending the maintainer's dogfooding.
