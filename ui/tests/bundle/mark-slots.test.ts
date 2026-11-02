// @vitest-environment node
/**
 * AJ-E3206 — the backend gives an App or an Account one of
 * `COLOUR_SLOTS` colour slots (schema v42, in its trigger too); the UI
 * draws slot n in `PROFILE_HUES[n]`. The two counts are one.
 */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { PROFILE_HUES } from "../../src/lib/mark-hues";

const rust = readFileSync(fileURLToPath(new URL("../../../crates/juggler-core/src/storage/entities.rs", import.meta.url)), "utf8");

describe("AJ-E3206 the colour slots", () => {
  it("are as many in the backend as the UI has hues", () => {
    expect(Number(/pub const COLOUR_SLOTS: i64 = (\d+);/.exec(rust)?.[1])).toBe(PROFILE_HUES.length);
    // The migration's own copy: its backfill's modulus and its trigger's slots.
    const migration = rust.slice(rust.indexOf("COLOUR_SLOTS_MIGRATION"));
    expect(migration).toContain(`% ${PROFILE_HUES.length};`);
    expect(migration).toContain(`SELECT ${PROFILE_HUES.length - 1}) AS slots`);
  });
});
