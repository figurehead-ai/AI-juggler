<!-- captures: 4b0b1cffbb2b -->
# 96-switch-setup: what the other account is missing before you switch, and how to set it up

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

These are close crops of the switch's set-up section: can-set-up lists release-notes (ticked, set up as it is) and github (needs your GITHUB_TOKEN) with numbered Terminal steps, a command block and Copy / Check again; turn-on adds computer-use with Show me how, Open claude.ai settings and its own steps; unused says Personal already has everything, with the extra items folded under one disclosure. The words are good: each item says what it is, why it is listed ("Used here") and what you do, and the secret is never copied. The 0.3.2 colours help in can-set-up, where the command block is a lighter surface that stands off the darker page and a grey well on white in light. But the command block wraps anywhere, splitting "--sc|ope", "u|ser" and "comman|d", so the shown command looks broken even though Copy pastes it whole. turn-on is the worst capture: the dialog is wider than the frame, so its × and the computer-use steps box are cut at the right edge, it opens scrolled to "secret." with its heading and first two items above the fold, and its "Switch now" is a tinted outline rather than the solid violet primary used elsewhere. nothing-missing-dark and nothing-missing-light were not re-rendered and still show the 0.3.1 colours and the pale Switch button.

## Weak

- can-set-up-dark, can-set-up-light, turn-on-dark, turn-on-light: the command block breaks words mid-token (--sc|ope, comman|d).
- turn-on-dark, turn-on-light: the dialog overflows the frame on the right (× and the steps box cut) and opens scrolled past its heading, release-notes and the start of github.
- turn-on-dark, turn-on-light: "Switch now" is not the solid violet primary.
- nothing-missing-dark, nothing-missing-light (not re-rendered): the "Continues exactly · usual account · Beta · Not proven yet" chip wraps onto a second line with its border broken across the break; the content has no left margin.
