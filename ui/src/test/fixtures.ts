/**
 * Whole, typed fixtures for tests: every field of the backend's types
 * filled with a plain value, each overridable, so a test names only what
 * it is about (and never casts a part-filled object to the type). The
 * preview's seams on `window` are set, read and cleared here by name.
 */
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { LaunchRecipeInfo } from "../types/generated/LaunchRecipeInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";
import type { SubscriptionInfo } from "../types/generated/SubscriptionInfo";

/** A session of AI Juggler's, running in a terminal. */
export function instance(over: Partial<InstanceInfo> = {}): InstanceInfo {
  return {
    id: "sess-1",
    launch_recipe_id: "p-1",
    provider: "claude-code",
    account_id: "work",
    account_label: "Work Max",
    model: null,
    launch_model: null,
    running_model: null,
    model_source: "launch",
    status: "running",
    quota: null,
    tokens: null,
    cpu_pct: null,
    rss_bytes: null,
    child_count: null,
    subtask_count: null,
    git_branch: null,
    git_dirty_count: null,
    needs_attention: null,
    attention: null,
    started_at_ms: Date.now() - 3_600_000,
    last_activity_ms: Date.now() - 60_000,
    machine_id: "this-mac",
    cwd: "/w/alpha",
    kind: "terminal",
    adopted: false,
    project_id: null,
    conversation_id: null,
    isolation_label: null,
    ...over,
  };
}

/** A folder (an App's sign-in), signed in. */
export function account(over: Partial<AccountInfo> = {}): AccountInfo {
  return { id: "work", provider: "claude-code", label: "Work", plan: null, login_state: "logged_in", cleanup_opt_in: false, renewal_date: null, preview_opt_in: false, in_place: false, ...over };
}

/** A project. */
export function project(over: Partial<ProjectInfo> = {}): ProjectInfo {
  return { id: "p1", root: "/w/alpha", name: "alpha", last_activity_ms: null, active_instance: null, accounts_used: [], instance_count: 0, history_sessions: 0, history_last_used_ms: null, repo: null, missing: false, ...over };
}

/** Launch settings. */
export function recipe(over: Partial<LaunchRecipeInfo> = {}): LaunchRecipeInfo {
  return { id: "rec-1", name: "Main", account_id: "work", account_label: "Work", provider: "claude-code", model: null, default_cwd: null, restart_on_crash: null, ...over };
}

/** An Account (a subscription) over one folder of the same id. */
export function subscription(over: Partial<SubscriptionInfo> & { folder?: string } = {}): SubscriptionInfo {
  const { folder = "work", ...rest } = over;
  return {
    key: `k-${folder}`,
    account_id: null,
    name: null,
    display_name: folder,
    tool: "claude-code",
    state: "signed_in",
    email: null,
    organisation: null,
    reason: null,
    profiles: [{ id: folder, label: folder, folder: `~/.claude-${folder}`, plan: null, renewal_date: null, fee_usd: null }],
    plan: null,
    renewal_date: null,
    fee_usd: null,
    differs: [],
    choices: [],
    ...rest,
  };
}

/** Sets the preview's seam `name` (`__AIJ_…__`). */
export function setSeam(name: string, value: unknown): void {
  Reflect.set(window, name, value);
}

/** The seam's value now. */
function seamOf(name: string): unknown {
  return Reflect.get(window, name);
}

/** Clears seams. */
export function clearSeams(...names: string[]): void {
  for (const name of names) Reflect.deleteProperty(window, name);
}

/** The strings a list-shaped seam holds (calls recorded by the preview). */
export function seamCalls(name: string): string[] {
  const value = seamOf(name);
  return Array.isArray(value) ? value.filter((entry): entry is string => typeof entry === "string") : [];
}

/** `element`, known to be a button (else the test fails here). */
export function asButton(element: Element | null | undefined): HTMLButtonElement {
  if (!(element instanceof HTMLButtonElement)) throw new Error("not a button");
  return element;
}

/** `element`, known to be an HTML element. */
export function asElement(element: Element | null | undefined): HTMLElement {
  if (!(element instanceof HTMLElement)) throw new Error("not an element");
  return element;
}

/** A CustomEvent's detail (else the test fails here). */
export function detailOf(event: Event): unknown {
  if (!(event instanceof CustomEvent)) throw new Error("not a CustomEvent");
  return event.detail;
}
