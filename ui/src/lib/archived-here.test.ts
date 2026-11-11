/**
 * NY1 (rc32): archiving a session took it out of the rail at once, but
 * its need stayed in the needs list until that list was read again: for
 * about three seconds a row in the need's own words stood in its place.
 * Archived here, its needs leave at once; unarchived, they're back.
 */
import { afterEach, describe, expect, it } from "vitest";

import { resetArchived, withoutArchived } from "./archived-here";
import { instanceArchive, instanceUnarchive } from "./ipc/session-history";
function need(subject: string, sessionId: string | null): { subject: string; target: { session_id: string | null } } {
  return { subject, target: { session_id: sessionId } };
}

const ROWS = [need("s:s1", "s1"), need("s:s2", "s2"), need("room", null)];

afterEach(resetArchived);

describe("an archived session leaves Needs you at once (NY1, rc32)", () => {
  it("its needs go when it's archived and come back when it's unarchived", async () => {
    expect(withoutArchived(ROWS)).toEqual(ROWS);
    await instanceArchive("s1");
    expect(withoutArchived(ROWS).map((row) => row.subject)).toEqual(["s:s2", "room"]);
    await instanceUnarchive("s1");
    expect(withoutArchived(ROWS)).toEqual(ROWS);
  });

  it("a need named by its session's rail key goes too", async () => {
    await instanceArchive("s2");
    expect(withoutArchived([need("s:s2", null)])).toEqual([]);
  });
});
