<!-- captures: f1106f1a37be -->
# 45-remote: a remote session or machine on Mac mini seen from Fleet: Overview live, stale and offline, Processes, and a view-only Actions tab

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The header reads cleanly (breadcrumb, Work Max with Working and Mac mini chips, then a freshness line), and the Overview field card sits a step lighter than the canvas in dark and white on grey in light, with labels dimmer than values as intended. The freshness line carries the state: amber for Stale and Offline, but Live has only a grey dot, so the good state looks like the neutral one. In instance-actions-view-only the only action, Stop on Mac mini, is disabled as a pale red outline that is hard to read in light, while the reason sits beside it in body text. The data still contradicts itself: instance-processes says "Processes 3" right above "Unavailable — Mac mini hasn't reported this session's processes", and instance-overview-stale and -offline label every field "measured 1 s ago" while the header says the last update was 40 s ago or the Mac is offline. The design fixes show only in the chrome: at 1440 the rail's error rows now read "Stopped with an error" whole and the Personal row "Limit reached · 0% left" without a trailing cut, the top bar stays on one row, and in dark the Overview / Processes / Actions tab highlight and the logo mark take the lighter violet. Every capture was re-rendered.

## Weak

- instance-processes: "Processes 3" and "Unavailable — Mac mini hasn't reported this session's processes" contradict each other.
- instance-overview-stale, instance-overview-offline: fields say "measured 1 s ago" under a Stale (40 s) or Offline header.
- instance-actions-view-only (light): the disabled Stop on Mac mini is pale red on pale pink and barely readable.
- instance-overview-live, instance-processes, machine-overview: the Live dot is grey, the same as a neutral state.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

Fleet › Mac mini › Session with Overview, Quality, Processes and Actions. Still as before: stale and offline overviews also say "measured 1 s ago"; Processes 3 sits above "Unavailable — Mac mini hasn't reported…"; `instance-actions-view-only` shows an Offline header.
