/** AJ-E1612 dry run 3: a session's page had a tab "Usage" beside the
 * header's "Usage", two visible controls of one name doing different
 * things (this item's usage; the Usage screen). A tab named as a header
 * destination says whose it is; its words shown don't change. */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { PRIMARY_NAV } from "../lib/nav";
import { PageShell } from "./PageShell";
import { pageTabName, pageTabs } from "./registry";

afterEach(cleanup);

describe("page tabs named as a header destination", () => {
  it("say whose they are, starting with the words shown", () => {
    expect(pageTabName("session", "Usage")).toBe("Usage (this session)");
    expect(pageTabName("project", "Pools", 2)).toBe("Pools 2 (this project)");
    expect(pageTabName("subscription", "Projects", 0)).toBe("Projects (this account)");
    expect(pageTabName("machine", "Projects")).toBe("Projects (this Mac)");
    // Not a header destination: the words shown are its name.
    expect(pageTabName("session", "Overview")).toBeUndefined();
    expect(pageTabName("session", "Processes")).toBeUndefined();
  });

  it("leave no page tab of any kind sharing a name with the header's tabs", () => {
    const header = new Set(PRIMARY_NAV.map((item) => item.label));
    const kinds = ["session", "conversation", "project", "app", "subscription", "tool", "machine", "remote-session", "pool"] as const;
    const facts = { desktop: true, hasSubagents: true, hasMessages: true, hasTasks: true, hasHistory: true, hasUsage: true, live: true, cli: true, local: true };
    for (const kind of kinds) {
      for (const tab of pageTabs(kind, facts)) {
        const name = pageTabName(kind, tab.label) ?? tab.label;
        expect(header.has(name), `${kind}: ${tab.label}`).toBe(false);
      }
    }
  });

  it("render with that name, and the words shown unchanged", () => {
    const tabs = [
      { id: "terminal" as const, label: "Overview", source: "" },
      { id: "pools" as const, label: "Pools", source: "", count: 2 },
      { id: "usage" as const, label: "Usage", source: "" },
    ];
    const view = render(<PageShell kind="session" subject="s-1" tabs={tabs} header={<h2>s</h2>} render={(tab) => <p>{`shown: ${tab}`}</p>} />);
    expect(view.getAllByRole("tab").map((tab) => tab.textContent)).toEqual(["Overview", "Pools 2", "Usage"]);
    expect(view.getByRole("tab", { name: "Usage (this session)" })).toBeTruthy();
    expect(view.getByRole("tab", { name: "Pools 2 (this session)" })).toBeTruthy();
    expect(view.getByRole("tab", { name: "Overview" })).toBeTruthy();
    expect(view.queryByRole("tab", { name: "Usage" })).toBeNull();
  });
});
