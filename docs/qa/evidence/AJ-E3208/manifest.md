# AJ-E3208 — The Accounts screen: Accounts and Apps

Captures here, both themes, 1024 and 1280, checked by eye (2026-11-06, release/0.4.1).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Accounts n and Apps n, each counted; the choice remembered (walks A2, A6; `apps-*.png`: "Accounts 2 · Apps 7", 7 rows). The switch answers in 43–65 ms |
| R2 | PASS | Each App row: icon and mark, whole name, its Account ("on Claude Max 20x", "Not signed in", "Its account isn't known yet"), what's open or Closed, its one action (New session, Go to it, Start) and ⋯ with its page, Restart… and Keep alive (`apps-*.png`; walk A6). A row's name opens its page in 114 ms |
| R3 | PASS | Added profiles listed running or not; found ones offered, not listed (A6, D-84) |
| R4 | PARTLY | Restart from the Apps section's ⋯ asks first: "Restart Claude app · Work Max? 2 sessions are working in it…", Cancel focused, Escape closes, opens in 27–38 ms (`restart-confirm-*.png`). The restart itself is AJ-E3209's walk, with the sanctioned test profile in the live sitting |
| Gallery | PASS | Both themes at 1024 and 1280, here |
| Fix | — | The switch (the usage screens' own segmented control, on 7 screens) now marks the chosen side in the accent's tint like the app's standard control (the 0.4 design review, 32): Usage, Tasks, the Account and project pages change with it; the gallery baselines of those screens are reviewed with the release |
