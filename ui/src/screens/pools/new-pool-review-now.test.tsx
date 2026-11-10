/**
 * The rc27 P1 walk: Review promised an App's sessions "now and later",
 * and the pool's card then counted every ended session as "now". Review
 * says each broad member's open sessions here now, in the card's words.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { draftOf } from "../../lib/new-pool";
import { templateById } from "../../lib/pool-templates";
import { NewPoolReview } from "./NewPoolReview";

afterEach(cleanup);

describe("Review says what each member brings now (the rc27 P1 walk)", () => {
  it("an App's chip counts its open sessions here, as its canvas card will", () => {
    const members = [
      { ref: "session:lead", kind: "session" as const, label: "web · Lab", role: "lead" as const, level: null, reach: "full" as const },
      { ref: "app:cli:lab", kind: "app" as const, label: "Claude Code · Lab", role: "member" as const, level: null, reach: "later" as const },
      { ref: "app:cli:codex", kind: "app" as const, label: "Codex · Account G", role: "reviewer" as const, level: null, reach: "later" as const },
    ];
    const draft = { ...draftOf(templateById("writer-reviewer"), "web"), members };
    const openNow = new Map([["session:lead", 1], ["app:cli:lab", 1], ["app:cli:codex", 0]]);
    const review = render(<NewPoolReview draft={draft} onName={() => undefined} projectName={(id) => id ?? "No project"} openNow={openNow} />);
    const rows = [...review.container.querySelectorAll("li")].map((row) => row.textContent);
    expect(rows[1]).toContain("1 open now · new ones join");
    expect(rows[2]).toContain("none open now · new ones join");
    expect(rows.join(" ")).not.toContain("now and later");
  });
});
