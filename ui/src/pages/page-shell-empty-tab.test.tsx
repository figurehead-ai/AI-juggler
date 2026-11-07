/** The 0.4.1 persona walks: sessions opened on a remembered Pools tab
 * that said "In no pool now". A remembered tab this item has nothing in
 * lets go to the body; one the user picks here stays. */
import { cleanup, fireEvent, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { PageShell } from "./PageShell";
import { rememberTab } from "./registry";

const tabs = [
  { id: "overview" as const, label: "Overview", source: "" },
  { id: "pools" as const, label: "Pools", source: "" },
];

function page(subject: string, empty: boolean) {
  return <PageShell kind="session" subject={subject} tabs={tabs} emptyTabs={empty ? ["pools"] : undefined} header={<h2>s</h2>} render={(tab) => <p>{`shown: ${tab}`}</p>} />;
}

describe("a remembered tab with nothing in it", () => {
  beforeEach(() => rememberTab("session", "pools"));
  afterEach(cleanup);

  it("opens a session in no pool on its body, and one in a pool on Pools", () => {
    const none = render(page("s-none", true));
    expect(none.getByText("shown: overview")).toBeTruthy();
    none.unmount();
    expect(render(page("s-pooled", false)).getByText("shown: pools")).toBeTruthy();
  });

  it("keeps Pools when the user picks it on this session", () => {
    const view = render(page("s-none", true));
    fireEvent.click(view.getByRole("tab", { name: /Pools/ }));
    expect(view.getByText("shown: pools")).toBeTruthy();
  });
});
