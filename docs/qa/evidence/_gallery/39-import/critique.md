<!-- captures: 919d8e6dae7b -->
# 39-import: bringing in a Claude Code folder, its problem states, and the sign-in step after

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The page reads well at both sizes: a four-step tracker, a large "Bring in this folder?", a brighter lead sentence, the folder card (white on the grey canvas in light, a step lighter in dark), a Name field, then one solid violet "Bring it in" (or "Bring in settings only" in f-oversize) at the right with quieter Back and "Pick a different folder…" links. In dark the lighter accent now shows on the step-1 dot, "Bring it in", the "Sign in" button in f-signin and the Consumer Terms link; nothing else on these screens moved. f-non-config uses a red box and drops the primary, and f-oversize uses an amber box, so those two states are easy to read. The other problem states are not: in f-moved "That folder moved or is gone. Pick it again." is plain body text while the card still says "Signed in" and "Bring it in" stays the primary, and in f-duplicate the "Already imported as «Work»" line has its own Done button next to a still-primary "Bring it in", so two actions compete. In f-non-config the card still says its sign-in is kept in the macOS Keychain although the error says there is no sign-in. At 390 wide the step tracker wraps into three rows with stray connector lines, and in f-signin-390x844 the account name is cut to "cla…" while the row has room. All captures were re-rendered.

## Weak

- f-moved-*: the "moved or is gone" problem is plain text, not a warning, while "Bring it in" stays the solid primary and the card still says "Signed in".
- f-duplicate-*: a Done button beside the duplicate notice competes with the primary "Bring it in".
- f-non-config-*: the card's "Its sign-in is kept in the macOS Keychain…" contradicts the error "no settings, sign-in or session history in it".
- f-*-390x844-*: the step tracker wraps into three rows with stray connector lines.
- f-signin-390x844-*: the account name is cut to "cla…" though the row has space.
- f-oversize-*-light: the amber notice turns a pinkish beige in light, close to f-non-config's error red.
