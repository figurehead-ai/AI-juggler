<!-- captures: 62ab0afaef98 -->
# 51-identity: a session's identity chip on its page, and the wrong-account dialog that stops a launch

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In rail-and-chip the darker dark canvas and the grey light canvas finally lift the terminal, Where it runs and Runway cards off the page, and New session in the top bar is one solid violet that clearly outranks Switch account… and Relaunch. The red "Stopped with an error" pill and the blue Work Max account chip sit side by side under the title and read well in both themes. Two things are cut, though: the selected rail row reads "Stopped with an error · not" with the rest lost, and at 1280 wide the "Not signed in yet — the identity is checked once it i…" chip runs past the right edge of the Where it runs card. The wrong-account dialogs are clear: an amber-edged dark card or a white light card, with an Expected / Signed in as table and a brighter sentence below. But "Use «Personal» (Default)…" is a pale lavender strip, not the solid violet primary, so no action stands out. All eight captures were re-rendered.

## Weak

- rail-and-chip-1280x800-dark/light: the "Not signed in yet — the identity is checked once it is…" chip overflows the Where it runs card and is cut at its right edge.
- rail-and-chip (all four): the selected rail row's line after "Stopped with an error ·" is cut to "not", so the age and the sign-in state are lost.
- wrong-account (all four): the way out, "Use «Personal» (Default) — it is signed in as …", is a pale outlined strip, as weak as Check again and Cancel; the dialog has no clear primary action.
- rail-and-chip-1024x680: the terminal text starts flush against the terminal's left edge with no padding.
