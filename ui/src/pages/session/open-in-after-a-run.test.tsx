/**
 * 0.4.6 (the rc3c walk): the guide says a Claude app conversation
 * continued in AI Juggler and stopped there opens again with "Open in
 * Claude app". That run's page had no such button: Resume, Relaunch, Run in
 * AI Juggler and Switch account… (other Accounts only). The run
 * stands for its conversation, so the conversation's own page, which has
 * it, wasn't reached. Now the ended run's header offers it too, in the
 * app the conversation was last open in, and not while it's open
 * somewhere now (Go to it is that door).
 */
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ResumeButton } from "./ResumeButton";
import { resetToasts } from "../../components/Toast";
import { clearSeams, instance, setSeam } from "../../test/fixtures";
import type { KnownPlaceInfo } from "../../lib/ipc/places";

const ECHO = "f162f390-d34b-43db-a171-d111e2e140e4";
const PROFILE = "app:claude-app:/Users/you/Profiles/aij-test";

const place: KnownPlaceInfo = {
  recent: [],
  id: PROFILE,
  app_id: "claude-app",
  app: "Claude app",
  instance: "aij-test",
  profile: "aij-test",
  label: "Claude app · aij-test",
  app_pid: null,
  lost: false,
  keep_alive: false,
  paused: null,
  account_id: null,
  seen_ms: 1,
  cannot_start: null,
  found: false,
};

/** The places the preview answers with: `remembered` is where each
 * conversation was last open. */
const places = (remembered: Record<string, string>) => setSeam("__AIJ_PLACES__", { places: [place], sessions: remembered });

const run = instance({ id: "sess-echo-terminal", provider: "claude-code", status: "stopped", conversation_id: ECHO, account_id: "acc-aij" });

afterEach(() => {
  cleanup();
  resetToasts();
  clearSeams("__AIJ_PLACES__");
});

describe("AI Juggler's run of a Claude app conversation, stopped", () => {
  it("offers Open in Claude app, where the conversation was last open", async () => {
    places({ [`claude-code/${ECHO}`]: PROFILE });
    render(<ResumeButton session={run} openIn={null} />);
    const open = await screen.findByTestId("header-open-in");
    expect(open.textContent).toBe("Open in Claude app");
    expect(open.getAttribute("aria-label")).toBe("Open in Claude app · aij-test");
    expect(screen.getByTestId("header-resume")).toBeTruthy();
  });

  it("offers nothing more while it's open somewhere now, or with no app to go back to", async () => {
    places({ [`claude-code/${ECHO}`]: PROFILE });
    render(<ResumeButton session={run} openIn="Claude app · aij-test" />);
    await waitFor(() => expect(screen.getByTestId("header-resume")).toBeTruthy());
    expect(screen.queryByTestId("header-open-in")).toBeNull();
    cleanup();
    places({});
    render(<ResumeButton session={run} openIn={null} />);
    await waitFor(() => expect(screen.getByTestId("header-resume")).toBeTruthy());
    expect(screen.queryByTestId("header-open-in")).toBeNull();
  });
});
