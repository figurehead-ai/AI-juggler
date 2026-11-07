/**
 * The go sitting's 7b walk (2026-11-07): New pool's "Add a project" listed
 * "web" twice, nothing telling them apart. Same-named projects show their
 * folder under the name; a name of its own shows none.
 */
import { describe, expect, it } from "vitest";
import { projectChoices } from "./project-words";

describe("projects in a picker", () => {
  it("show their folder only where two share a name", () => {
    expect(
      projectChoices([
        { id: "a", name: "web", root: "/Users/you/lab/web" },
        { id: "b", name: "web", root: "/Users/you/clients/acme/web" },
        { id: "c", name: "team", root: "/Users/you/lab/team" },
      ]),
    ).toEqual([
      { value: "a", label: "web", hint: "~/lab/web" },
      { value: "b", label: "web", hint: "~/clients/acme/web" },
      { value: "c", label: "team" },
    ]);
  });
});
