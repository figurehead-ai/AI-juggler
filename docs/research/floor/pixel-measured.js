// Measured 2026-11-03 in the Claude desktop app's browser pane (Apple silicon, WebKit-based),
// mock view 959x539 CSS px at 1x, 24 frames per size, GPU finished each frame (gl.finish).
window.PIXEL_MEASURED = {
  rows: [
    { k: 1, lw: 959, lh: 539, ms: 2.21 },
    { k: 2, lw: 479, lh: 269, ms: 1.50 },
    { k: 3, lw: 319, lh: 179, ms: 1.97 },
    { k: 4, lw: 239, lh: 134, ms: 1.75 },
    { k: 6, lw: 159, lh: 89, ms: 1.86 },
  ],
  note: 'Measured in the in-app browser on an Apple-silicon Mac, with the view at 959×539 at 1×, 24 frames per size and the GPU finished on each frame. At this size, a frame costs about 2 ms whatever the pixel size, because the cost here is the ~560 separate objects (draw calls), not pixels. Instancing removes most of that. On a 2× Retina screen at full width the pixel count is about 8 times higher, and that is where smaller internal resolutions pay off. "≈ CPU at 8 fps" is frame time × 8 per second: an upper bound for animating. Press Measure to re-run it on your machine.',
};
