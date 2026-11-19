# Evidence manifest — AJ-E813

Home draws again only when This Mac's summary level changes, not on every 5 s reading. Built on release/0.4.6, 2026-11-16; verified by Opus.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Home reads This Mac's summary level through `useMachineLevel` (`ui/src/lib/machine.ts`): the level alone from This Mac's store with `useSyncExternalStore`, as the rail rows' heavy note since AJ-E805 round 4. Home (`ui/src/screens/Home.tsx`) no longer reads the whole reading; `homeNeedsOf` takes the level (`ui/src/lib/needs-you.ts`). Commits 1d205929d, 228870270 |
| R2 | PASS | `ui/src/screens/home-machine-level.test.tsx`: Home's own draws counted through its per-draw `useKeys("home-action")` (React's Profiler reports nothing in jsdom): two readings at the same level draw nothing, a new level draws again. On the old code it fails: "readings at the same level: expected 2 to be +0" (named in 1d205929d) |

Quality gates: the test, tsc and vitest pass (Home, needs and machine suites: 11 files, 81 tests). The AJ-E805 bar is measured with AJ-E812's runs (`docs/qa/evidence/AJ-E805/idle-0.4.6.md`).
