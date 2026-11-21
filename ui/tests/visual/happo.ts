/**
 * The capture harness's `test` with Happo's `happoScreenshot` fixture
 * (AJ-E2020 R6). Outside `happo -- playwright test` (CI's happo job only)
 * the fixture does nothing, so a spec on this test runs as before; the
 * committed PNG baselines stay the gate meanwhile.
 *
 * **Keep every `component` and `variant` name stable.** Happo pairs an
 * example with its baseline by those names only: a renamed one shows as
 * one example added and one deleted, compared with nothing, so that
 * screen silently has no visual regression check until a new baseline is
 * approved. (0.5's release PR: all 24 Home examples gained the target's
 * name, `…-chrome-large`, and Happo compared none of them.) Rename only
 * on purpose, say so in the pull request, and check the new examples by
 * eye before approving.
 */
import { mergeTests } from "@playwright/test";
import { test as happoTest } from "happo/playwright";
import { test as harness } from "./axe-gallery";

export { expect, gate } from "./axe-gallery";
export const test = mergeTests(harness, happoTest);
