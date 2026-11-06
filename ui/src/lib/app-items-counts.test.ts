/**
 * Walk K1: an App's and an Account's Projects count each project's
 * sessions as the backend does, not from the window's newest few hundred.
 */
import { expect, it } from "vitest";
import { withProjectCounts } from "./app-items";
import { project } from "../test/fixtures";

it("takes each project's count from the backend, and adds the ones the sample missed", () => {
  const base = [{ id: "p-web", name: "web", sessions: 35, lastMs: 2_000 }];
  const projects = [project({ id: "p-web", name: "web" }), project({ id: "p-api", name: "api", last_activity_ms: 1_000 })];
  const rows = withProjectCounts(base, new Map([["p-web", 168], ["p-api", 167]]), projects);
  expect(rows).toEqual([
    { id: "p-web", name: "web", sessions: 168, lastMs: 2_000 },
    { id: "p-api", name: "api", sessions: 167, lastMs: 1_000 },
  ]);
  // Not read yet: the sample as it is.
  expect(withProjectCounts(base, undefined, projects)).toEqual(base);
});
