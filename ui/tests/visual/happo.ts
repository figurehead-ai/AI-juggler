/**
 * The capture harness's `test` with Happo's `happoScreenshot` fixture
 * (AJ-E2020 R6). Outside `happo -- playwright test` (CI's happo job only)
 * the fixture does nothing, so a spec on this test runs as before; the
 * committed PNG baselines stay the gate meanwhile.
 */
import { mergeTests } from "@playwright/test";
import { test as happoTest } from "happo/playwright";
import { test as harness } from "./axe-gallery";

export { expect, gate } from "./axe-gallery";
export const test = mergeTests(harness, happoTest);
