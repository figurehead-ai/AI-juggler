<!-- captures: 6d5850eba213 -->
# 51-identity: a session's identity chip on its page, and the wrong-account dialog that stops a launch

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

In rail-and-chip the darker dark canvas and the grey light canvas lift the terminal, Where it runs and Runway cards off the page, and New session in the top bar is one solid violet that clearly outranks Switch account… and Relaunch. The red "Stopped with an error" pill and the blue Work Max account chip sit side by side under the title and read well in both themes. Two design fixes show here. In the Where it runs card, "Not signed in yet — the identity is checked once it is" is now a plain line of text that fits inside the card at 1280 instead of a chip running past its right edge. In the rail, status lines now show their extra parts whole or not at all, so the selected row reads "Stopped with an error", the next "Probably stuck" and the Fake Desk row "Ended", with nothing cut; the cost for this family is that the sign-in state ("not signed in", "identity unchecked") these rows used to carry is gone from the rail, while rows with room still show "first sign-in". At 1024 the top bar keeps one row with the broom and 7. The second pass takes the terminal's black to its card's edge, so the lighter frame is gone and "Work Max · session" and the prompt sit on a black margin instead of against the edge; in the wrong-account captures only the dimmed terminal behind the dialog changed. The wrong-account dialogs are clear: an amber-edged dark card or a white light card, with an Expected / Signed in as table and a brighter sentence below, and the way out, "Use «Personal» (Default) — it is signed in as …", is now a solid violet bar above the outlined Check again and Cancel, so the dialog has one obvious action. All eight captures were re-rendered.

## Weak

- rail-and-chip (all four): the rail rows for sessions not signed in drop their sign-in state entirely ("Stopped with an error", "Probably stuck", "Ended"), so the rail no longer shows which sessions have an unchecked identity.
- rail-and-chip (all four): the selected rail row shows a small "···" under its status icon with no visible meaning.
