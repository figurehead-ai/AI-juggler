import { afterEach, describe, expect, it, vi } from "vitest";

/** The backend's side (main_window.rs's CloseNote): a close that hid the
 * window makes it due; once told it is recorded and never due again. */
const backend = { closed: false, told: false, toldCalls: 0 };
const closeHidesWindow = () => {
  backend.closed = true;
};
vi.mock("./ipc/app-lifecycle", () => ({
  appCloseNoteDue: () => Promise.resolve(backend.closed && !backend.told),
  appCloseNoteTold: () => {
    backend.told = true;
    backend.closed = false;
    backend.toldCalls += 1;
    return Promise.resolve();
  },
}));
const shown: Array<{ message: string; detail?: string }> = [];
vi.mock("../components/Toast", () => ({
  showToast: (options: { message: string; detail?: string }) => {
    shown.push(options);
    return "close-note";
  },
}));
import { watchCloseNote } from "./close-note";

const setHidden = (hidden: boolean) => {
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => (hidden ? "hidden" : "visible") });
  document.dispatchEvent(new Event("visibilitychange"));
};
/** Lets the backend's answers land. */
const settle = () => new Promise((done) => setTimeout(done, 0));

afterEach(() => {
  setHidden(false);
  shown.length = 0;
  Object.assign(backend, { closed: false, told: false, toldCalls: 0 });
});

describe("the close note (D-112)", () => {
  it("shows once, on the next show after the first close that hid the window, and never again", async () => {
    const stop = watchCloseNote();
    // Shown with no close yet: nothing to say.
    setHidden(true);
    setHidden(false);
    await settle();
    expect(shown).toEqual([]);

    // The close hides the window: nothing while it's hidden.
    closeHidesWindow();
    setHidden(true);
    await settle();
    expect(shown).toEqual([]);

    // On screen again: the toast, once, and the backend is told.
    setHidden(false);
    await settle();
    expect(shown).toHaveLength(1);
    expect(`${shown[0]?.message} ${shown[0]?.detail}`).toBe("AI Juggler kept running while its window was closed. It stays in the menu bar. ⌘Q quits.");
    expect(backend.toldCalls).toBe(1);

    // Hidden and shown again, and another close: never again.
    setHidden(true);
    setHidden(false);
    closeHidesWindow();
    setHidden(true);
    setHidden(false);
    await settle();
    expect(shown).toHaveLength(1);
    expect(backend.toldCalls).toBe(1);
    stop();
  });

  it("asks once while an answer is on its way, and stops listening when stopped", async () => {
    const stop = watchCloseNote();
    closeHidesWindow();
    setHidden(true);
    setHidden(false);
    setHidden(true);
    setHidden(false);
    await settle();
    expect(shown).toHaveLength(1);
    stop();
    backend.told = false;
    closeHidesWindow();
    setHidden(true);
    setHidden(false);
    await settle();
    expect(shown).toHaveLength(1);
  });
});
