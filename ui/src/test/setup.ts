/**
 * Every test file's setup. Async waits (waitFor, findBy…) get 10 s instead
 * of Testing Library's 1 s, and a test 30 s instead of 5: under a loaded
 * full run (the whole suite in parallel beside a Rust build) the 1 s
 * default ran out in a different settings or first-run test each time
 * (#198). A wait still returns the moment its condition holds, so a
 * passing test is no slower; only a real failure takes longer to say so.
 */
import { configure } from "@testing-library/react";

configure({ asyncUtilTimeout: 10_000 });
