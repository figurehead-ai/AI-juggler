/**
 * This machine's name on screen, in one place (the OS seam, D-52): "This
 * Mac" on macOS; the Windows and Linux releases name theirs here.
 */
export const THIS_MACHINE = "This Mac";

/** The same, inside a sentence ("No sessions on this Mac yet"): from the
 * one name above, so each OS sets it once. */
export const THIS_MACHINE_IN_TEXT = THIS_MACHINE.replace(/^This\b/, "this");

/** One machine, as a kind of page (a page's label above its name). */
export const MACHINE_WORD = THIS_MACHINE.replace(/^This\s+/, "");

/** Several machines ("Macs"), from the one word above. */
export const MACHINES_WORD = `${MACHINE_WORD}s`;

/** D1 (the 0.4.6 persona walk: "Nothing running — your Mac has room"
 * beside "72 % of all 30 cores" and "11 GB of 13 GB moved to disk"): what
 * "room" measures, in plain words, where its level shows. The figures are
 * the backend's own thresholds (`RoomThresholds::default`,
 * juggler-supervisor `machine.rs`): change them together. */
export const ROOM_MEASURES =
  "Room is free memory, swap, the processor, heat and disk space. It turns tight only when one runs short: the processor over 90% for a full minute, swap nearly full and holding a quarter of your memory or more, under a tenth of memory free, the Mac slowing down to cool off, or a disk with under 20 GB free.";

