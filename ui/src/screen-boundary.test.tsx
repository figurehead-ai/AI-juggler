/**
 * The 0.0.12 report: one screen's error (Usage) blanked the whole window.
 * A throwing screen now shows its own panel; the header and the rail keep
 * working, and the error goes to the crash-report path.
 */
import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ScreenBoundary, errorDetails } from "./components/ScreenBoundary";

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));
// Usage throws, as it did on 0.0.12 (null.toFixed).
vi.mock("./screens/Usage", () => ({
  UsageScreen: () => {
    throw new TypeError("Cannot read properties of null (reading 'toFixed')");
  },
}));

const w = () => window as unknown as Record<string, unknown>;

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
  delete w().__AIJ_UI_ERRORS__;
  // React logs a caught error to the console; the test expects it.
  vi.spyOn(console, "error").mockImplementation(() => {});
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
  vi.restoreAllMocks();
});

it("a throwing screen shows its panel; the header and the rail stay", async () => {
  const { default: App } = await import("./App");
  const user = userEvent.setup();
  const view = render(<App />);
  await user.click(await view.findByRole("button", { name: "More" }));
  await user.click(await view.findByRole("button", { name: /^Usage/ }));
  const panel = await view.findByTestId("screen-problem");
  expect(panel.getAttribute("data-screen")).toBe("usage");
  expect(panel.textContent).toContain("This screen hit a problem");
  expect(panel.textContent).toContain("Usage couldn't be shown");
  // Not a blank window: the header's navigation and the rail are there.
  expect(view.getByRole("button", { name: "Home" })).toBeTruthy();
  expect(view.getByTestId("rail")).toBeTruthy();
  // And the rest works: Home opens.
  await user.click(view.getByRole("button", { name: "Home" }));
  expect(view.queryByTestId("screen-problem")).toBeNull();
  // The error went to the crash-report path: the screen and the error only.
  expect(w().__AIJ_UI_ERRORS__).toEqual([{ screen: "usage", message: "TypeError: Cannot read properties of null (reading 'toFixed')" }]);
});

it("Try again re-renders the screen; Copy details copies the error, not the screen", async () => {
  let fail = true;
  const Flaky = () => {
    if (fail) throw new Error("boom");
    return <p>Back</p>;
  };
  const writeText = vi.fn(async () => undefined);
  Object.defineProperty(navigator, "clipboard", { value: { writeText }, configurable: true });
  render(
    <ScreenBoundary screen="history" label="History">
      <Flaky />
    </ScreenBoundary>,
  );
  const panel = screen.getByTestId("screen-problem");
  fireEvent.click(within(panel).getByRole("button", { name: "Copy details" }));
  expect(writeText).toHaveBeenCalledTimes(1);
  expect(String((writeText.mock.calls[0] as unknown[])[0])).toMatch(/^Screen: history\nError: boom/);
  fail = false;
  fireEvent.click(within(panel).getByRole("button", { name: "Try again" }));
  expect(screen.getByText("Back")).toBeTruthy();
});

it("the details are the error and its place in the code", () => {
  const error = new TypeError("x is null");
  error.stack = "TypeError: x is null\n    at CostMatrix (index.js:1:2)";
  expect(errorDetails("usage", error)).toBe("Screen: usage\nTypeError: x is null\nTypeError: x is null\n    at CostMatrix (index.js:1:2)");
});
