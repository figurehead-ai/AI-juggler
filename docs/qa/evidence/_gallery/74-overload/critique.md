<!-- captures: 19f06408aaf2 -->
# 74-overload: the Mac-under-strain states: the memory and CPU banner above a session, Home with the pressure item, and This Mac's advice

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The new banner is one bordered strip with an amber triangle, two lines of plain text and a close mark. It sits well above the session page, and in light its pale red tint reads as a warning without shouting. The session page now has two solid violet buttons in view, "Go to it" in the header and "Open / Focus window" in the preview card, so neither clearly leads. In busy-cpu the banner says "Close the The Claude app windows you aren't using", with "the" doubled. In banner-above-session and memory-pressure the long status pill "This Mac · Under memory pressure" pushes the header's actions onto a second row under the tabs. In memory-pressure and recommendations the cards stand off the canvas, and "See what's using it" is the one strong action. banner-above-session-1024x800 (dark and light) was not re-rendered and shows the older one-line amber banner in the older shell.

## Weak

- busy-cpu-1280x800-dark/light: banner text doubles a word: "Close the The Claude app windows".
- banner-above-session-1280x800-*, memory-pressure-1280x800-*: the header wraps to two rows because of the long status pill.
- banner-above-session-1280x800-*, busy-cpu-*: two solid violet buttons (Go to it, Open / Focus window) on one page.
- banner-above-session-1024x800-*: not re-rendered.
