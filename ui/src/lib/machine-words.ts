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
