# ADR-0021: A hand-built SVG chart kit, no charting library

**Status:** Accepted
**Date:** 2026-10-25
**Deciders:** the maintainer (the 0.1.4 review asking for "graphs, piecharts, additional historical data"), the 0.1.5 visual-usage work

## Context

The maintainer, on 0.1.4 at a 2000 px window: Usage "should have a lot more
info, graphs, piecharts, additional historical data … Everything needs to be a
lot more visual, easy to read". The app had no chart components (one switch
diagram), and the Usage, project and account pages were text cards.

The data is already stored: `usage_events` (tokens and estimated cost per
message, by profile, session and model), `quota_samples` (each window's
readings and resets), the sessions and the conversations index. The pages need
seven kinds of picture: a stat tile with a sparkline and the change against the
previous period; area and stacked bars over time; plain and stacked bars; a
donut with a legend and percentages; a calendar heatmap of active hours; a
horizontal share bar; and a runway gauge for a limit window.

Every chart has the same obligations:

- themed from the app's tokens, dark and light, with the profile badge
  palette (`lib/palette.ts`), so one profile is one colour on every page;
- hover tooltips with exact values, keyboard focus on every mark, an
  accessible description and a "Show as table" alternative;
- an honest empty state that keeps the chart's frame;
- reduced motion respected;
- the 12 px floor of the readability scale on axes and labels.

Constraints: the repository is public and every dependency is supply-chain
surface (`deny.toml`, Dependabot, the security guidelines); the first paint
must not grow (lazy-load everywhere); the size budget caps a UI file at 400
lines; ADR-0019 asks for no duplication and no special cases.

## Decision

AI Juggler draws its charts itself: a small kit of React components that
render SVG, in `ui/src/components/charts/`, with no new dependency.

- **One scale module** (`scale.ts`): linear scales, "nice" ticks, compact
  numbers, and the stacked layout. Every component places marks, ticks and
  labels with it, so a label always names a value the chart reaches.
- **One frame** (`ChartFrame.tsx`): the title row, the legend, the empty state
  that keeps the frame's height, the "Show as table" toggle with a real
  `<table>`, and the tooltip. Each chart supplies its marks and its rows.
- **Colour from data, never from rank**: series are keyed by entity, so a
  profile's swatch is `accountColor(id)` everywhere and a filter that removes
  one series never repaints the others. Text never wears a series colour.
- **Marks per the house spec**: bars at most 24 px with a 4 px rounded data
  end, a 2 px surface gap between stacked segments, 2 px lines, area washes,
  hairline grids, ticks at 12 px.
- **Interaction**: each bar, cell, segment and point is a focusable SVG
  element with an `aria-label` that states its value; pointer and focus show
  the same tooltip; a crosshair snaps to the nearest bucket on time charts.
- **Lazy**: the kit is imported only by screens that are themselves loaded on
  first use (`lazyParts.ts`), so nothing of it is in the first paint.

## Alternatives considered

- **Recharts** — the most common React choice. Around 130 KB gzipped with its
  d3 and lodash dependencies (more than the whole Usage screen with this kit),
  dozens of transitive packages to audit in a public repo, colours set through
  props rather than our CSS tokens, and its SVG marks aren't keyboard-focusable
  without wrapping each one. Rejected.
- **visx** — modular d3 primitives, 40–60 KB gzipped for the parts we would use
  and a dozen `@visx/*` and `d3-*` packages. It removes little code: we would
  still write every chart, legend, tooltip and table, only on top of its
  scales. Rejected for the dependency surface it adds for the maths it saves.
- **uPlot / Chart.js** — fast canvas charts (about 22 KB and 70 KB gzipped).
  A canvas has no marks to focus or describe, so accessibility needs a second,
  parallel DOM layer; theming is through JavaScript options, not CSS
  variables, so a theme switch must redraw. Our series are small (at most a
  few hundred points per chart, because the backend sends summaries), so
  canvas speed buys nothing. Rejected.
- **No charts, better tables** — the status quo the maintainer rejected.

## Consequences

- No third-party code, so no decision-log entry for a dependency and nothing
  new for `deny.toml`, Dependabot or the licence inventory.
- The kit is ours to maintain: a few hundred lines, each file under the size
  budget, with unit tests for the scale maths and the component contracts
  (focus, labels, empty state, table view).
- Anything beyond these seven forms (zoom and pan, brushing, thousands of
  points, maps) would need this decision revisited; the backend's summary
  queries keep every series small enough that none is planned.
- The profile palette is equal in lightness by design (white letters on every
  swatch), so neighbouring hues are close for colour-blind readers. Charts
  therefore never rely on colour alone: a legend in stack order, direct labels
  on donuts, gaps between segments, tooltips and the table view carry
  identity too.
