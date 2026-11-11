/**
 * D-62 (AJ-E3202 req. 3) — the preview's stand-in for the backend's
 * Account name rule (`subscriptions::name_all`, which fills
 * `display_name` where the list is built), so the preview, the galleries
 * and the visual tests name each Account as the app would. The screens
 * never run it: they read `display_name` (`accountName`). Mirrors the
 * Rust rule; its tests are there.
 */
import type { SubscriptionInfo } from "../types/generated/SubscriptionInfo";
import { planWords } from "../lib/subscriptions";
import { toolSpec } from "../lib/tools";

/** A row as the preview's fixture gives it (its name, if any, is redone). */
export type UnnamedSubscription = Omit<SubscriptionInfo, "display_name"> & { display_name?: string };

/** Some, and not blank. */
function given(value: string | null | undefined): string | null {
  return value && value.trim() ? value : null;
}

function toolWords(tool: string): string {
  return toolSpec(tool).short_name || toolSpec(tool).display_name;
}

/** AJ-E3202 req. 3: the one name all its Apps were accepted under at
 * import, said by its folders (a Claude app profile on an accepted folder
 * keeps it); none when a folder wasn't accepted or they differ
 * (`subscriptions::accepted_name`). */
function acceptedName(row: UnnamedSubscription): string | null {
  const names = row.profiles.map((profile) => given(profile.accepted_name)?.trim() ?? null);
  const first = names[0] ?? null;
  return first !== null && names.every((name) => name === first) ? first : null;
}

function defaultName(row: UnnamedSubscription): string {
  const organisation = given(row.organisation);
  if (organisation) return organisation;
  const plan = given(row.plan);
  if (plan) return `${toolWords(row.tool)} ${planWords(plan)}`;
  return acceptedName(row) ?? given(row.email) ?? `${toolWords(row.tool)} account`;
}

function nameOf(row: UnnamedSubscription, all: readonly UnnamedSubscription[], defaults: readonly (string | null)[]): string {
  if (row.state === "signed_out") return "Not signed in";
  const name = given(row.name);
  if (name) return name.trim();
  if (row.service) return row.service.name;
  if (row.state === "unverified") return "Account unknown";
  const base = defaultName(row);
  const twins = all.filter((_, at) => defaults[at] === base);
  if (!twins.some((other) => other.key !== row.key)) return base;
  const email = given(row.email);
  if (email && email !== base) return `${base} · ${email}`;
  if (new Set(twins.map((other) => other.tool)).size === twins.length) return `${toolWords(row.tool)} · ${base}`;
  const order = twins.map((other) => other.account_id ?? other.key).sort();
  return `${base} (${order.indexOf(row.account_id ?? row.key) + 1})`;
}

/** The rows, each with the name the backend would give it. */
export function previewNamed(rows: readonly UnnamedSubscription[]): SubscriptionInfo[] {
  const defaults = rows.map((row) => (row.state === "signed_in" && !given(row.name) && !row.service ? defaultName(row) : null));
  return rows.map((row) => ({ ...row, display_name: nameOf(row, rows, defaults) }));
}
