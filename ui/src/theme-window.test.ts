/**
 * 0.5.1 F21: with AI Juggler on Light and the Mac in dark mode, a cold
 * launch's first frame was dark. The backend makes the window in the
 * chosen theme's colour, so the page tells it every choice it makes.
 */
import { afterEach, describe, expect, it, vi } from "vitest";

const appSettingSet = vi.fn(async () => {});
const setBackgroundColor = vi.fn(async () => {});
vi.mock("./lib/ipc/settings", () => ({ appSettingSet }));
vi.mock("@tauri-apps/api/webviewWindow", () => ({ getCurrentWebviewWindow: () => ({ setBackgroundColor }) }));

vi.stubGlobal("matchMedia", (query: string) => ({ matches: false, media: query, addEventListener: () => {}, removeEventListener: () => {} }));

afterEach(() => {
  Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  localStorage.clear();
  vi.clearAllMocks();
});

describe("the window's theme, told to the backend", () => {
  it("each choice reaches the setting the backend makes the window from", async () => {
    Reflect.set(window, "__TAURI_INTERNALS__", {});
    const { setTheme, WINDOW_THEME_KEY } = await import("./theme");
    expect(WINDOW_THEME_KEY).toBe("ui.window.theme");
    setTheme("light");
    await vi.waitFor(() => expect(appSettingSet).toHaveBeenCalledWith("ui.window.theme", "light"));
    setTheme("system");
    await vi.waitFor(() => expect(appSettingSet).toHaveBeenCalledWith("ui.window.theme", "system"));
    // The same choice again: told once.
    setTheme("system");
    await new Promise((done) => setTimeout(done, 20));
    expect(appSettingSet).toHaveBeenCalledTimes(2);
  });

  it("outside the app nothing is written", async () => {
    const { setTheme } = await import("./theme");
    setTheme("dark");
    await new Promise((done) => setTimeout(done, 20));
    expect(appSettingSet).not.toHaveBeenCalled();
  });
});
