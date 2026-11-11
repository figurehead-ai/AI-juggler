/** The 0.4.1 persona walk (Priya, R4): a project's menu has a door to
 * keeping it apart, and it opens the sealed template on that project. */
import { describe, expect, it, vi } from "vitest";

import { PROJECT } from "./action-specs-setup";
import { onNewPoolRequest, requestNewPool, takeNewPoolRequest } from "./pools";
import { project as fixtureProject } from "../test/fixtures";

describe("Keep it apart…", () => {
  it("is on a project's menu where the app can open the Pools screen", () => {
    const spec = PROJECT.find((one) => one.id === "keep-apart");
    expect(spec?.label).toBe("Keep it apart…");
    const keep = vi.fn();
    const project = fixtureProject({ id: "p1", name: "client-a" });
    expect(spec?.applies(project, {})).toBe(false);
    expect(spec?.applies(project, { keepProjectApart: keep })).toBe(true);
    void spec?.run(project, { keepProjectApart: keep });
    expect(keep).toHaveBeenCalledWith(project);
  });

  it("hands the Pools screen one request, once", () => {
    const heard = vi.fn();
    const stop = onNewPoolRequest(heard);
    requestNewPool({ template: "sealed-client", project: "p1" });
    expect(heard).toHaveBeenCalledOnce();
    expect(takeNewPoolRequest()).toEqual({ template: "sealed-client", project: "p1" });
    expect(takeNewPoolRequest()).toBeNull();
    stop();
  });
});
