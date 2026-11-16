/**
 * The 0.4.6 RC walk X1: a pool whose lead had ended showed "Lead" on its
 * card and "Lead: web · started 5:16 PM (1)" in Who builds, while Who
 * leads chose nothing and read "No lead for now: nobody splits the work
 * into tasks". The picker listed only open sessions. An ended lead is
 * listed, chosen, and read back as ended.
 */
import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { LeadPicker } from "./LeadPicker";
import { NO_LEAD } from "../../../lib/pool-words";

afterEach(cleanup);

describe("Who leads, when the lead has ended", () => {
  it("lists the lead, chosen, and says it has ended", () => {
    const ended = { ref: "s1", name: "web · started 5:16 PM (1)", detail: "Ended with AI Juggler" };
    render(<LeadPicker sessions={[]} apps={[]} value={{ kind: "session", ref: "s1" }} onChange={() => {}} ended={ended} testId="pool-lead" />);
    const sentence = screen.getByTestId("pool-lead-sentence").textContent;
    expect(sentence).not.toBe(NO_LEAD);
    expect(sentence).toContain("web · started 5:16 PM (1) leads, but it has ended");
    const chosen = screen.getAllByRole("radio").find((radio) => radio instanceof HTMLInputElement && radio.checked);
    expect(chosen?.closest("[data-ref]")?.getAttribute("data-ref")).toBe("s1");
    expect(screen.getByTestId("pool-lead-ended").textContent).toContain("Ended with AI Juggler");
  });

  it("an open lead is read back as before", () => {
    const open = { ref: "s2", name: "api (2)", detail: "Working" };
    render(<LeadPicker sessions={[open]} apps={[]} value={{ kind: "session", ref: "s2" }} onChange={() => {}} ended={null} testId="pool-lead" />);
    expect(screen.getByTestId("pool-lead-sentence").textContent).toBe("api (2) leads: it plans the work and hands out tasks.");
    expect(screen.queryByTestId("pool-lead-ended")).toBeNull();
  });
});
