# Fleet, keyboard only (AJ-E710's a11y sweep), 2026-11-07

The design preview with the Fleet gallery's own fixtures (`capture.spec.ts`), dark and light, driven by a Playwright script with keys only (Tab, Shift+Tab, Enter, Space, arrows, Escape and the app's shortcuts). Run by a Sonnet agent against a written sheet; its findings checked against the code by Opus, and every fix tested.

| Flow | Keys | Reached | Found | Now |
|---|---|---|---|---|
| Pair, as the host | ⌘⇧F; Tab ×2 to Add a machine…; Enter; (the other side joins); Enter on Fingerprints match; ↓/↑ between permissions; Tab ×2 to Pair; Enter; Enter on Done | every step | focus visible at each step; nothing mouse-only | — |
| Pair, joining | Enter on Join a machine…; Tab to the machine; Space; type the code (auto-advances); Tab to Connect | every control | the dialog opened with nothing focused (its machines arrive after it opens): 9 Tabs through the page behind before the first machine; Tab left the dialog; closing left focus on the page body; Pair by address's field sits above its toggle, so Tab from the toggle skipped it | fixed in 2d6df1e07: focus goes in, stays in, returns to the opener; opening Pair by address focuses its field (`pairing-wizard.test.tsx`) |
| A cross-machine switch | from Home to Continue in another app…; Enter; Tab to a Mac's row; Enter; each step's controls by Tab and Space; Tab to Switch to CLI on Mac mini (not pressed); Escape | every step and the final button | focus moves to each step's heading; meaning in words and checks, not colour alone. A terminal keeps Tab for itself (Ctrl+Tab leaves it, as its hint says) | — |
| A remote cleanup | the menu (Enter on the avatar, ↓ to Processes); Tab to Clean node (pid 4242) on Mac mini; Enter; Tab to Clean on Mac mini (not pressed); Escape | every control | Tab left the confirm, and closing gave focus to the page body | fixed in 2d6df1e07: Tab stays in, focus returns to Clean… (`remote-cleanup.test.tsx`) |

**Colour alone:** none found. States carry words or marks (Connected, Synced, Offline, Stale, Live, ✓ Verified owned, Not owned — never touched, Parked).

**Not covered:** pairing a real peer (the preview has no backend; the live run is the second-Mac check, D-89).
