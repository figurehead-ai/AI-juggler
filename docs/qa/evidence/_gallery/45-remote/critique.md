<!-- captures: 9324280d68a7 -->
# 45-remote: a remote session or machine on Mac mini seen from Fleet: Overview live, stale and offline, Processes, and a view-only Actions tab

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The header reads cleanly (breadcrumb, Work Max with Working and Mac mini chips, then a freshness line), and the Overview field card now sits a step lighter than the canvas in dark and white on grey in light, with labels dimmer than values as intended. The freshness line carries the state: amber for Stale and Offline, but Live has only a grey dot, so the good state looks like the neutral one. In instance-actions-view-only the only action, Stop on Mac mini, is disabled as a pale red outline that is hard to read in light, while the reason sits beside it in body text. The data contradicts itself in places: instance-processes says "Processes 3" right above "Unavailable — Mac mini hasn't reported this session's processes", and instance-overview-stale and -offline still label every field "measured 1 s ago" while the header says the last update was 40 s ago or the Mac is offline. Every capture was re-rendered.

## Weak

- instance-processes: "Processes 3" and "Unavailable — Mac mini hasn't reported this session's processes" contradict each other.
- instance-overview-stale, instance-overview-offline: fields say "measured 1 s ago" under a Stale (40 s) or Offline header.
- instance-actions-view-only (light): the disabled Stop on Mac mini is pale red on pale pink and barely readable.
- instance-overview-live, instance-processes, machine-overview: the Live dot is grey, the same as a neutral state.
- Rail at 1440 (all captures): "Stopped with an error · 7 …" cuts the age.
