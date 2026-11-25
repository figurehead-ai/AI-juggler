/**
 * AJ-E1553 / AJ-E1545 R8 — the sign-ins AI Juggler runs, over IPC (one
 * thin module per backend area, ENGINEERING-GUIDE U3).
 *
 * - `signInsList`: every sign-in (running, signed in, ended early), and
 *   the ended ones kept from before a restart (stale ones cleared).
 * - `signInsOpenPage(id)`: opens the page the tool showed; the backend
 *   opens the address its adapter read, never one sent from here.
 * - `signInsDismiss(id)`: takes one off the list (ends it if it runs).
 * - `onSignInsChanged`: the list, each time it changes.
 *
 * The browser preview and the tests set `window.__AIJ_SIGN_INS__`.
 */
import { call, listen, liveBackend as hasBackend } from "./transport";
import type { SignInInfo } from "../../types/generated/SignInInfo";

export type { SignInInfo };

const SIGN_INS_CHANGED = "sign-ins://changed";

declare global {
  interface Window {
    /** The browser preview's and the tests' sign-ins (none unless set). */
    __AIJ_SIGN_INS__?: SignInInfo[];
    /** The pages the preview "opened", by sign-in id. */
    __AIJ_SIGN_IN_PAGES_OPENED__?: string[];
  }
}

/** Every sign-in, oldest first. */
export async function signInsList(): Promise<SignInInfo[]> {
  if (!hasBackend) return window.__AIJ_SIGN_INS__ ?? [];
  return call("sign_ins_list");
}

/** Opens the page sign-in `id`'s tool showed, in the browser. */
export async function signInsOpenPage(id: string): Promise<void> {
  if (!hasBackend) {
    window.__AIJ_SIGN_IN_PAGES_OPENED__ = [...(window.__AIJ_SIGN_IN_PAGES_OPENED__ ?? []), id];
    return;
  }
  await call("sign_ins_open_page", { id });
}

/** Takes sign-in `id` off the list, ending it if it still runs. */
export async function signInsDismiss(id: string): Promise<void> {
  if (!hasBackend) {
    window.__AIJ_SIGN_INS__ = (window.__AIJ_SIGN_INS__ ?? []).filter((one) => one.id !== id);
    return;
  }
  await call("sign_ins_dismiss", { id });
}

/** Called with every sign-in each time the list changes. */
export function onSignInsChanged(handler: (signIns: SignInInfo[]) => void): Promise<() => void> {
  if (!hasBackend) return Promise.resolve(() => undefined);
  return listen<SignInInfo[]>(SIGN_INS_CHANGED, (event) => handler(event.payload));
}
