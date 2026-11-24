import { describe, expect, it } from "vitest";
import { CLIP_IDS, permissionClip } from "./clips";

describe("permission cards' clips (AJ-E1545 R3)", () => {
  it("each card the setup offers shows its effect: Notifications has its own clip", () => {
    expect(permissionClip("notifications")).toBe("perm-notify");
    expect(permissionClip("accessibility")).toBe("perm-focus");
    expect(permissionClip("screen-recording")).toBe("perm-preview");
    expect(CLIP_IDS).toContain("perm-notify");
  });
});
