/**
 * AJ-E1534 — orientation: you always know where you are. The top bar's
 * title and highlight come from the page shown, the open item's rail
 * row is selected (filled, the accent bar, `aria-current`) whatever its
 * type, and each type of page has its own mark.
 */
import { cleanup, render, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import App from "./App";
import { useProjectLens } from "./components/project-lens.test-kit";

// AJ-E2104: the project heading is the Project lens's.
useProjectLens();

vi.mock("./components/TerminalView", () => ({
  TerminalView: ({ id }: { id: string }) => <div data-testid={`terminal-${id}`} />,
}));

beforeEach(() => {
  window.matchMedia ??= ((query: string) =>
    ({ matches: false, media: query, onchange: null, addListener() {}, removeListener() {}, addEventListener() {}, removeEventListener() {}, dispatchEvent: () => false }) as MediaQueryList);
  localStorage.setItem("aij.onboarding.dismissed", "1");
  sessionStorage.setItem("aij.ready.skipped", "1");
});
afterEach(() => {
  cleanup();
  localStorage.clear();
  sessionStorage.clear();
});

async function openApp() {
  const view = render(<App />);
  const rail = await waitFor(() => {
    const found = view.container.querySelector<HTMLElement>("[data-testid='rail']");
    expect(found).not.toBeNull();
    return found!;
  });
  return { view, rail };
}

const navItem = (view: ReturnType<typeof render>, name: string) =>
  within(view.getByRole("navigation", { name: "Main" })).getByRole("button", { name });

it("Home lights only on Home; a session's page lights nothing and is titled by its project", async () => {
  const user = userEvent.setup();
  const { view, rail } = await openApp();
  expect(navItem(view, "Home").getAttribute("aria-current")).toBe("page");
  const row = await waitFor(() => {
    const found = [...rail.querySelectorAll<HTMLButtonElement>("button")].find((b) => /feat\/read-model/.test(b.textContent ?? ""));
    expect(found).toBeTruthy();
    return found!;
  });
  await user.click(row);
  await view.findByTestId("right-inspector");
  expect(navItem(view, "Home").getAttribute("aria-current")).toBeNull();
  expect(view.getByRole("heading", { level: 1 }).textContent).toBe("ai-juggler");
  // Its rail row is the selected one, and its page's mark is the session's.
  expect(row.getAttribute("aria-current")).toBe("true");
  expect(view.getByTestId("session-mark")).toBeTruthy();
});

it("an account's page is under Accounts; the open project's rail heading is selected", async () => {
  const user = userEvent.setup();
  const { view, rail } = await openApp();
  await user.click(navItem(view, "Accounts"));
  const name = (await view.findAllByTestId("account-label"))[0]!;
  await user.click(name);
  await waitFor(() => expect(view.getByTestId("page-type").textContent).toBe("Account"));
  expect(navItem(view, "Accounts").getAttribute("aria-current")).toBe("page");

  const heading = within(rail).getAllByTestId("rail-project")[0]!;
  await user.click(heading);
  // 0.3: a project's page is under Projects (its lit tab), its crumb
  // "Projects › its name".
  await waitFor(() => expect(view.getByTestId("crumb-projects").textContent).toBe("Projects"));
  expect(view.getByTestId("page-type").textContent).toBe(view.getByTestId("page-name").textContent);
  expect(heading.getAttribute("aria-current")).toBe("true");
  expect(navItem(view, "Projects").getAttribute("aria-current")).toBe("page");
  expect(navItem(view, "Accounts").getAttribute("aria-current")).toBeNull();
  expect(within(view.getByTestId("page-header")).getByTestId("mark-project")).toBeTruthy();
});
