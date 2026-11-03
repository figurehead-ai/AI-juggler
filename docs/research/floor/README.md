# The Floor — the Office look (research and prototype)

> **Status 2026-11-03: the top-down prototype is stopped.** The maintainer
> asked for an office seen from above at an angle, with perspective: back
> walls with windows and doors, all generated. The research for that is in
> `angled-office.html` (published privately as "The Floor · Angled office").
> It covers how games do it, three look mocks drawn by our own code
> (`angled-mocks.js`) and a recommendation:
> - real 3D with three.js in the webview, from a fixed angled camera;
> - a stacked-floor cutaway as the Building level;
> - this folder's 2D renderer as the fallback where hardware WebGL is
>   missing (notably WebKitGTK).
>
> **Picked look (2026-11-03):** between B and C. It keeps A's high fixed
> angle and see-through cutaway, uses fine pixel art (not chunky) and has no
> rotation. `pixel-office.html` with `pixel-office.js` mocks it:
> - a three.js scene seen from one fixed camera;
> - rendered at a reduced internal resolution and scaled up with hard edges;
> - a small pixel filter (colour steps, depth outlines);
> - crisp labels on an overlay.
>
> A frame measured about 2 ms at any pixel size in the in-app browser.
> No build until the maintainer confirms. Everything below describes
> the stopped top-down round; its layout, data map, people-from-parts and
> cost lessons carry over.

> D-65 put the Floor in 0.5, research first. On 2026-11-03 the maintainer
> chose the Office look from the four concepts, and asked for:
> - a procedural pixel office with a section per App and a desk per session;
> - more floors when one fills up, and closed rooms for reserved functions;
> - people generated once and cached;
> - decay for unused Apps, a wave for "needs you", fire for overload and
>   red alert for a long machine overload;
> - useful props;
> - a "Re-create" setting.
>
> The build is filed as [E34](../../tasks/E34-the-floor.md).

## What's here

| File | What it is |
|---|---|
| `office-engine.js` | The prototype engine: seeded layout (bays, floors, rooms), the people and props generator, the on-demand Canvas 2D renderer, hit-testing and the desk vignette. Plain JS, no libraries. |
| `prototype.html` | A playable page on synthetic data (generic names). It covers the three company sizes, events (App overload, Mac overload, close an App, +1 week), the Office settings, Needs you, the inspector with doors, the list view and a live cost readout. |
| `pixel-office.html`, `pixel-office.js`, `pixel-measured.js` | The picked look: a fixed-angle pixel office mock with pixel-size, light and outline controls, a frame-cost measurement, and how to build it. |
| `angled-office.html`, `angled-mocks.js` | The angled-camera research page and its three mocks: A isometric pixel, B three.js 3D, C cutaway building. |
| `measure/measure.cjs` | The CPU measurement: headless WebKit and Chromium, 1440×900 at 2×, 15 s per scenario. CPU is read from the OS for the browser's whole process tree (WebKit's launchd-started WebContent, GPU and Networking processes included), and the page's own draw time is read inside it. |

Run the prototype locally with any static server from this folder (for example
`python3 -m http.server`), or open the published copy. To re-measure, set
`NODE_PATH` to a `node_modules` that has `@playwright/test`, then run
`node measure/measure.cjs webkit` and `node measure/measure.cjs chromium`.

## Decisions taken in the prototype

- **Layout.** The open office is a stable grid of bays: 4 desks per bay, 24 bays (96 desks) per floor.
  - Apps claim runs of bays in serpentine order, so zones stay contiguous.
  - A session keeps its desk for life. Growth takes the next free bay as an annex. A full floor opens the next one.
  - Treemap and BSP plans were considered and set aside: they re-flow when sizes change, and stability matters more than variety here.
- **Rooms.**
  - Server room = This Mac.
  - Meeting rooms = pools (more on later floors).
  - Kitchen, where idle people go.
  - Archive = ended today.
  - Lounge.
  - Your office = done today.
  - The lobby holds the elevator, the corkboard (notices), the clock wall (one clock per Account, showing its reset) and reception.
- **People.** Built from parts:
  - 6 skin tones;
  - 10 hairstyles in 10 colours;
  - 6 shirt styles, tinted by the App's mark hue;
  - 6 trousers and 4 shoes;
  - glasses, headphones and beards.

  Each person is seeded by office seed + App + staff slot and built once into an 18-frame atlas (252×22 px, about 22 KB). Subagents are helper bots, one atlas per App hue.
- **States.** The person's pose shows the state, together with a glyph and a word:
  - Working: typing;
  - Asks you: stands, faces you and waves;
  - Error: alarm;
  - Limit: asleep;
  - Stuck: scratching their head;
  - Done: leaning back, with a review paper;
  - Idle: still, with coffee walks;
  - Ended: an empty desk.

  Decay follows the App:
  - closed under a day: abandoned;
  - unused for a week: dust and cobwebs;
  - unused for a month, or its Account unused for a month: trash and lights off.

  Overload shows as a desk fire. Sustained Mac overload brings red alert: a steady wash breathing at 0.5 Hz, with no flashing.
- **Zoom levels.**
  - Floor plan: icons and counts above desks, and one line per zone.
  - Desks: people, bots and bubbles.
  - Up close: a cached monitor card per desk, with tasks and subagents.
- **Rendering.** Canvas 2D in the webview.
  - The floor is baked once per floor and theme, and people are atlases.
  - Frames are drawn on demand. A ticker (10 fps; Calm 4) runs only while something animated is on screen and the page is visible.
  - Zoomed in, the pixel world is drawn at its own resolution (one bitmap pixel per art pixel) and scaled up by the GPU with `image-rendering: pixelated`. A CSS translate carries the sub-pixel camera offset.
  - Text, badges and bubbles are on a full-resolution overlay that redraws only when data, the camera or the selection change.
  - Red alert breathes as a CSS animation.
  - Text bitmaps are cached.

## Measured cost (stopped mid-way)

- At rest the top-down renderer drew 0 frames and cost no measurable CPU in WebKit or Chromium.
- At Full motion the first version cost about 6% of one core in headless WebKit and 11% in a real WebKit window. The drawing code was only 0.6–1 ms per frame of that; the rest was compositing the full Retina canvas ten times a second.
- A dirty-rectangle clip did not help. Drawing the pixel world at its own resolution, with a separate text overlay, cut the per-tick bitmap from about 5 megapixels to about 0.1.
- That run was stopped when the direction changed, so there are no final numbers. The headed run that started was inconsistent (one scenario recorded 0 fps, likely because the window was throttled).
- **Lesson for any renderer:** the cost is pixels per frame, not objects. Draw on demand, and keep the animated layer's resolution low.

## Research (links verified 2026-11-03)

- **Prior art.**
  - [Pixel Agents](https://github.com/pablodelucca/pixel-agents): MIT code. Its art has no stated licence and is "based on" the CC0 [Metro City pack](https://jik-a-4.itch.io/metrocity-free-topdown-character-pack).
  - [Claude Office](https://github.com/paulrobello/claude-office): its sprites are AI-generated, per its [skill file](https://github.com/paulrobello/claude-office/blob/main/.claude/skills/office-sprite/SKILL.md).
  - Do not copy either project's art, layouts or fonts. Both guess an agent's state from transcripts or hooks; AI Juggler knows it.
- **Tilesets.**
  - [Kenney](https://kenney.nl/support): CC0, usable.
  - [LimeZu Modern Office](https://limezu.itch.io/modernoffice) and [Donarg / RetroPi office](https://retropi-art.itch.io/officetileset): no redistribution, so not in a public repo.
  - [Universal LPC generator](https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator): GPL code with CC-BY-SA art (share-alike), so avoided.
  - Our art is generated from code.
- **Layout.** [Squarified treemaps](https://diglib.eg.org/handle/10.2312/VisSym.VisSym00.033-042), [treemap floor plans (Marson & Musse 2010)](https://doi.org/10.1155/2010/624817), [BSP](https://www.roguebasin.com/index.php/Basic_BSP_Dungeon_generation), [rooms and mazes](https://journal.stuffwithstuff.com/2014/12/21/rooms-and-mazes).
- **Readable state.**
  - [Prison Architect needs](https://prisonarchitect.paradoxwikis.com/Needs): escalating balloons.
  - [Prison Architect fire](https://prisonarchitect.paradoxwikis.com/Fire): a fire needs its own alert.
  - [Gather status](https://support.gather.town/hc/en-us/articles/15910325944212): status shown on the avatar and in a list.
- **Canvas.** [MDN optimising canvas](https://developer.mozilla.org/en-US/docs/Web/API/Canvas_API/Tutorial/Optimizing_canvas), [web.dev canvas performance](https://web.dev/articles/canvas-performance), [MDN crisp pixel art](https://developer.mozilla.org/en-US/docs/Games/Techniques/Crisp_pixel_art_look), [WebKit 16.4 OffscreenCanvas](https://webkit.org/blog/13966/webkit-features-in-safari-16-4/), [WebKit bug 267291](https://bugs.webkit.org/show_bug.cgi?id=267291).
- **Accessibility.** [canvas fallback](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/canvas), [aria-live](https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Reference/Attributes/aria-live), [prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion), [WCAG 2.2.2](https://www.w3.org/WAI/WCAG22/Understanding/pause-stop-hide.html), [WCAG 2.3.1](https://www.w3.org/WAI/WCAG22/Understanding/three-flashes-or-below-threshold.html).
