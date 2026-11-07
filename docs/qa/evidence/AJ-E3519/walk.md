# AJ-E3519 hand walk in AI Juggler Dev (2026-11-06)

Same run as AJ-E3516's walk (replica, keychain-free, evening). Each item:
click VIEW, then the item in the native menu; the time is from the item's
click to a capture of the window (it includes the tool's pauses and the
capture itself, about 0.45 s).

| Item | Time | Result | Scores |
|---|---|---|---|
| VIEW button | 552 ms to the menu | the native menu: Show, Arrange, Labels, Light, Look, Settings › The Floor…, a check by each current choice (`dev-view-menu.png`) | 5/5/5/5 |
| Light › Always day | ≈ 0.6 s to day | day sky, sun, clouds; windows glass (`dev-always-day.png`, `dev-building-day.png`) | 5/4/5/5 |
| Look › Full resolution | 748 ms | chosen; the capture right after still shows the frame before it (taken too soon to tell) | 4/–/5/5 (not confirmed by eye) |
| Show › Active Apps only | 809 ms | the building with 0 Apps (nothing is active in the replica) (`dev-active-only.png`) | 5/5/4/5 |
| Labels › All names | 744 ms | applied | 5/5/5/5 |
| Labels › Attention only | 763 ms | applied | 5/5/5/5 |
| Arrange › Active Apps packed together (Floor 2) | 754 ms | Floor 2 with 0 Apps shown (nothing active) (`dev-packed-floor2.png`) | 5/5/4/5 |
| Arrange › Kept places | 761 ms | every area back | 5/5/5/5 |
| Light › Always night | 771 ms | night | 5/5/5/5 |
| Light › Follow my clock | 759 ms | back to the clock (evening: night) | 5/5/5/5 |
| Look › Pixel art | 778 ms | applied | 5/5/5/5 |
| Show › All Apps | 748 ms | all 9 Apps back | 5/5/5/5 |
| Labels › Smart | 808 ms | applied | 5/5/5/5 |

Not captured each time: the menu's checks after every choice (the first
menu shows them on the starting choices). That each choice is kept as its
`floor.*` key and checked is the store and window tests' (manifest R2).

Clarity 4 for "Active Apps only" and "packed" here: with nothing active
the building and the floor are empty, as asked, but an empty view could
say why ("No App is active now").
