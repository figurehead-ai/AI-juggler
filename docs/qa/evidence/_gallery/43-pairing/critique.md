<!-- captures: df710882276b -->
# 43-pairing: the Add a machine and Join a machine dialogs over Fleet, from code and fingerprint check to permissions, done and every failure

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

Every step reads as one clear dialog over a dimmed Fleet page: the six-digit code in 1-host-find and host-new-code is the biggest thing on screen, the Find → Verify → Permissions stepper ticks off as you go, and the primary action (Connect, Fingerprints match, Pair, Done, Start over, Turn pairing back on, Try again) is now one solid violet fill in both themes, clearly ahead of the outlined Cancel or Close. In light the white dialog lifts well off the grey canvas, and in dark the dialog surface is a visible step above the page, so nothing blurs together. The failure dialogs (fail-cancelled, fail-disabled, fail-expired, fail-locked, fail-mismatch) are calm and short, but their title is plain body-weight text with no warning mark or colour, and the stepper still shows 1–2–3 as if nothing happened. Every capture was re-rendered.

## Weak

- fail-cancelled, fail-disabled, fail-expired, fail-locked, fail-mismatch: the failure title has no warning mark or colour, and the stepper gives no hint of where it stopped, so a failure looks like an ordinary step.
- fail-none-found: "No machines found…" sits in a bordered box styled like the machine picker, so it reads as a row you could select rather than a message.
- 4-done: the success line is a plain "✓ Paired with Laptop" in body text; it is the end of the journey and could carry the success colour.
- Rail at 1280 (all captures): "Stopped with an error · 7 …" cuts the age.
