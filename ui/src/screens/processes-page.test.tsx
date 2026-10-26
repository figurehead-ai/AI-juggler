import { cleanup, fireEvent, render, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ProcessesScreen } from "./processes/ProcessesScreen";
import { forgetProcessRows, prefetchProcessRows } from "./processes/useProcessRows";

/**
 * AJ-E2404/E2405: the Processes page on the preview's made-up Mac (the
 * same answers the backend's query gives): Needs attention first, the
 * other views, search, "Include other apps" (names and memory only,
 * nothing to stop), Ports, Ended and Cleanup history.
 */

const rowKeys = (container: HTMLElement) =>
  [...container.querySelectorAll('[data-testid^="process-row-"]')].map((row) => row.getAttribute("data-testid")!.slice("process-row-".length));

function page() {
  const view = render(<ProcessesScreen />);
  return view;
}

afterEach(cleanup);
beforeEach(() => {
  forgetProcessRows();
  try {
    localStorage.removeItem("aij.processes.view");
  } catch {
    // no storage: the default view
  }
  delete (window as unknown as { __AIJ_PROCESSES__?: unknown }).__AIJ_PROCESSES__;
  delete (window as unknown as { __AIJ_PORTS__?: unknown }).__AIJ_PORTS__;
});

describe("AJ-E2404 the Processes page", () => {
  it("opens on Needs attention: what to clean up, under its session, and the tallies agree", async () => {
    const { container, getByTestId } = page();
    await waitFor(() => expect(container.querySelector('[data-testid="process-stop-8123"]')).not.toBeNull());
    expect(getByTestId("process-view-attention").getAttribute("aria-selected") ?? getByTestId("process-view-attention").getAttribute("aria-checked")).toBe("true");
    await waitFor(() => expect(getByTestId("tally-clean").textContent).toContain("7"));
    const keys = rowKeys(container);
    expect(keys).toContain("s:ours:s-pay");
    // A healthy session has nothing to show here.
    expect(keys).not.toContain("s:ours:s-web");
    // A zombie is explained in plain words.
    expect(container.textContent).toContain("only its parent can clear it");
  });

  it("opens with its rows at once when they were read after the first paint", async () => {
    await prefetchProcessRows();
    const { container } = page();
    // No wait: the kept rows paint with the page, and are read again behind it.
    expect(container.querySelector('[data-testid="processes-loading"]')).toBeNull();
    expect(rowKeys(container)).toContain("s:ours:s-pay");
  });

  it("Everything running folds a healthy session to one line until it's opened", async () => {
    const { container, getByTestId } = page();
    fireEvent.click(getByTestId("process-view-running"));
    await waitFor(() => expect(rowKeys(container)).toContain("s:ours:s-web"));
    const row = container.querySelector('[data-testid="process-row-s:ours:s-web"]') as HTMLElement;
    expect(row.getAttribute("aria-expanded")).toBe("false");
    expect(row.textContent).toMatch(/\d+ processes · .*B · /);
  });

  it("search finds a port", async () => {
    const { container, getByTestId } = page();
    fireEvent.click(getByTestId("process-view-running"));
    fireEvent.change(getByTestId("process-search"), { target: { value: "port:5173" } });
    await waitFor(() => {
      const processes = rowKeys(container).filter((key) => key.startsWith("p:"));
      expect(processes.length).toBe(1);
      expect(processes[0]).toMatch(/^p:8123:/);
    });
  });

  it("Include other apps adds the rest of this Mac: names and memory only, nothing to stop", async () => {
    const { container, getByTestId } = page();
    fireEvent.click(getByTestId("process-view-running"));
    await waitFor(() => expect(rowKeys(container)).toContain("s:ours:s-web"));
    expect(rowKeys(container)).not.toContain("a:other");
    fireEvent.click(getByTestId("process-filters"));
    fireEvent.click(getByTestId("filter-others"));
    await waitFor(() => expect(rowKeys(container)).toContain("o:Safari"));
    const heading = container.querySelector('[data-testid="process-row-a:other"]') as HTMLElement;
    expect(heading.textContent).toContain("Other apps on this Mac");
    expect(heading.textContent).toContain("names and memory only");
    const safari = container.querySelector('[data-testid="process-row-o:Safari"]') as HTMLElement;
    expect(safari.textContent).toContain("14 processes");
    expect(safari.textContent).toContain("Not in an AI session");
    expect(within(safari).queryByText("Stop…")).toBeNull();
    fireEvent.click(within(safari).getByLabelText("More for Safari"));
    await waitFor(() => expect(document.querySelector('[data-testid="menu-activity-monitor"]')).not.toBeNull());
    expect(document.querySelector('[data-testid="menu-stop"]')).toBeNull();
    expect(document.querySelector('[data-testid="menu-copy-pid"]')).toBeNull();
    expect(document.querySelector('[data-testid="menu-reveal"]')).toBeNull();
  });

  it("another app's leftover is stopped alone: no scope to widen (D-41b)", async () => {
    const { container } = page();
    await waitFor(() => expect(container.querySelector('[data-testid="process-stop-7301"]')).not.toBeNull());
    fireEvent.click(container.querySelector('[data-testid="process-stop-7301"]')!);
    await waitFor(() => expect(document.querySelector('[data-testid="kill-only-7301"]')).not.toBeNull());
    expect(document.querySelector('[data-testid="kill-only-7301"]')!.textContent).toContain("Only this process");
    expect(document.querySelector('[data-testid^="kill-scope-"]')).toBeNull();
  });

  it("Ports lists every open port with what it is", async () => {
    const { container, getByTestId } = page();
    fireEvent.click(getByTestId("process-view-ports"));
    await waitFor(() => expect(container.querySelector('[data-testid="port-row-5173"]')).not.toBeNull());
    expect(container.querySelector('[data-testid="port-row-5173"]')!.textContent).toContain("Left running");
    expect(container.querySelector('[data-testid="port-row-4321"]')!.textContent).toContain("Not from AI Juggler");
  });

  it("Ended reads how long each ran; Cleanup history names the session", async () => {
    const { container, getByTestId } = page();
    fireEvent.click(getByTestId("process-view-ended"));
    await waitFor(() => expect(rowKeys(container).some((key) => key.startsWith("e:"))).toBe(true));
    expect(container.textContent).toContain("Ran for");
    fireEvent.click(getByTestId("process-view-history"));
    await waitFor(() => expect(container.textContent).toContain("Checkout redesign"));
    expect(container.textContent).toContain("Landing page");
    expect(container.textContent).not.toContain("A session");
  });
});
