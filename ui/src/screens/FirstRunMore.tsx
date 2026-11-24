/**
 * AJ-E3306 — first run's found step past the setups found (the PF1
 * persona walk on rc26): the names setups come in by, each tool installed
 * with nothing set up offered as a fresh App in its own group (pass2/03 §5
 * rule 3), Add an app's desktop profiles (D-84: offered, never added by
 * themselves), "Where we looked" (F33) and the choice of how they come
 * in (ADR-0018).
 */
import { useMemo, useState, type JSX } from "react";
import { Disclosure, Radio } from "../components/controls";
import { SELECTED_CARD } from "../components/controls/selected";
import { OfferRow, usePlaceOffers } from "../components/PlaceOffers";
import { listWords } from "../lib/account-terms";
import { providerDisplayName, sameAsWords, toolFound, versionNotAsked, versionUnchecked } from "../lib/providers";
import { toolSpec } from "../lib/tools";
import { signedIn } from "../lib/bridge";
import { createAccount } from "../lib/ipc/accounts";
import type { ProbeRowData, ProviderInstallData, ScanReportData } from "../lib/ipc/providers";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import { colourAt, freeLabel } from "./FreshAccountStep";
import { FreshRow, type RowProgress } from "./FirstRunFoundRow";
import type { Placement, Signin } from "./FirstRun";

/** A group's tool, in its header: its version, or that it couldn't be
 * read (0.0.11) or wasn't asked (the bridge), never "not installed". */
export function cliWords(name: string, row: ProviderInstallData): string {
  return versionNotAsked(row) ? `${name} installed; its version isn't known` : versionUnchecked(row) ? `${name} found; couldn't check its version` : `${name} ${row.version ?? ""} installed`;
}

/** A path's last folder. */
export const folderOf = (path: string) => path.replace(/\/+$/, "").split("/").pop() ?? path;

/** The name it's brought in as (changeable later in Accounts): the
 * tool's own for its usual folder; the word a folder is named for when
 * that is all its name says beyond the tool's (`.claude-work` → "Work",
 * pass2/03 §5 rule 4, labels by inferred purpose); else the folder's
 * name, which says more than a guess (`.claude-client-acme`). */
export function labelOf(found: DiscoveredInfo): string {
  const defaultLabel = toolSpec(found.harness).default_label;
  if (found.origin === "vendor-default" && defaultLabel) return defaultLabel;
  const folder = folderOf(found.path).replace(/^\./, "");
  const purpose = found.inference?.purpose ?? "";
  const stem = /^[a-z0-9]+[-_](.+)$/i.exec(folder)?.[1];
  if (found.inference?.purpose_rule === "name-word" && purpose && stem?.toLowerCase() === purpose.toLowerCase()) return purpose;
  return folder;
}

/** Each setup's name, two of one tool that would read the same told
 * apart in list order ("Work", "Work 2"; pass2/03 §5 rule 4), and none
 * the same as an App's already in AI Juggler (`taken`). */
export function setupNames(installs: DiscoveredInfo[], taken: string[]): Map<string, string> {
  const out = new Map<string, string>();
  const used: Record<string, string[]> = {};
  for (const found of installs) {
    const own = (used[found.harness] ??= found.managed ? [] : [...taken]);
    const name = found.managed ? labelOf(found) : freeLabel(labelOf(found), own);
    own.push(name);
    out.set(found.path, name);
  }
  return out;
}

/** The progress key of a tool set up fresh. */
export const freshKey = (tool: string) => `fresh:${tool}`;

/** First run's tools installed with nothing set up (the scan's
 * `not_set_up`), the names they'd get, and which are ticked: none until
 * the user ticks one (each needs a sign-in; nothing is made before Bring
 * in). Adding for one tool lists none. */
export function useFresh(report: ScanReportData | null, tools: string[], taken: string[] | null, names: Map<string, string>) {
  const fresh = useMemo(
    () => (tools.length > 1 ? (report?.not_set_up ?? []).filter((tool) => tools.includes(tool)) : []),
    [report, tools],
  );
  const freshNames = useMemo(() => {
    const used = [...(taken ?? []), ...names.values()];
    return Object.fromEntries(
      fresh.map((tool) => {
        const name = freeLabel(providerDisplayName(tool), used);
        used.push(name);
        return [tool, name];
      }),
    );
  }, [fresh, names, taken]);
  const [freshPicked, setFreshPicked] = useState<Record<string, boolean>>({});
  return { fresh, freshNames, freshPicked, setFreshPicked };
}

/** One ticked tool set up fresh, in AI Juggler's own folder: its sign-in
 * for the queue, or `null` when it is signed in already. */
export async function setUpFresh(tool: string, name: string, colour: string): Promise<Signin | null> {
  const account = await createAccount(tool, name, null);
  return signedIn(account.login_state) ? null : { accountId: account.id, label: account.label, loginNote: null, provider: tool, colour };
}

/** Each tool with nothing set up, in its own group under the setups. */
export function FreshGroups({
  fresh,
  names,
  picked,
  onPick,
  installRows,
  colourFrom,
  busy,
  progress,
}: {
  fresh: string[];
  names: Record<string, string>;
  picked: Record<string, boolean>;
  onPick: (tool: string, on: boolean) => void;
  installRows: Record<string, ProviderInstallData>;
  /** The colour slot of the first (after the setups'). */
  colourFrom: number;
  busy: boolean;
  progress: (tool: string) => RowProgress;
}): JSX.Element | null {
  if (fresh.length === 0) return null;
  // AJ-E1545: installed but not set up is one closed group, nothing ticked;
  // ticking one sets it up in its own folder.
  return (
    <div className="border-b border-hairline px-5 py-3 last:border-b-0" data-testid="first-run-not-set-up">
      <Disclosure summary={<span className="font-medium">Installed, not set up yet · {fresh.length}</span>}>
        <p className="mt-2 text-meta text-muted">Ticking one sets it up in a folder of its own, and you sign in to it after. Nothing is made until you bring it in.</p>
        {fresh.map((tool, index) => {
          const install = installRows[tool];
          const name = providerDisplayName(tool);
          return (
            <fieldset key={freshKey(tool)} className="mt-2" data-testid={`first-run-group-fresh-${tool}`}>
              <legend className="sr-only">{`${name}, not set up`}</legend>
              <FreshRow
                tool={tool}
                name={names[tool] ?? name}
                installed={install && toolFound(install) ? cliWords(name, install) : `${name} is installed`}
                colour={colourAt(colourFrom + index)}
                checked={!!picked[tool]}
                disabled={busy}
                progress={progress(tool)}
                onCheck={(checked) => onPick(tool, checked)}
              />
            </fieldset>
          );
        })}
      </Disclosure>
    </div>
  );
}

/** AJ-E3306 (D-84): the desktop apps here with none of their profiles
 * seen open yet: where those profiles come from ("The Claude app is on
 * this computer. …"). */
export function desktopNoneWords(apps: string[]): string {
  const named = apps.map((app) => (app.endsWith(" app") ? `the ${app}` : app));
  const list = listWords(named);
  return `${list.charAt(0).toUpperCase()}${list.slice(1)} ${apps.length === 1 ? "is" : "are"} on this computer. Open a profile you use and it shows here to add, or add one any time in Settings › Desktop apps.`;
}

/** The desktop apps here (S10, in the words every screen uses) and their
 * profiles seen open but not added, each offered (Add / Not now) as Add
 * apps offers them; nothing when neither. `only`: one tool's app (its
 * host app id), for one tool's Add an app (F19). */
export function DesktopApps({ only = null }: { only?: string | null } = {}): JSX.Element | null {
  const desktop = usePlaceOffers(only);
  const offers = desktop.offers ?? [];
  // AJ-E1545: Add an app's (one tool) only; Your apps shows each profile
  // as a row in its tool's group (0.5.1 F35). Only profiles seen open and
  // not added yet are offered.
  if (offers.length === 0) return null;
  return (
    <section className="overflow-hidden rounded-2xl border border-hairline bg-surface" aria-label="Desktop apps" data-testid="first-run-desktop">
      <h2 className="bg-surface-2 px-5 py-2.5 text-meta font-semibold text-ink">Open now, not added yet</h2>
      <ul className="divide-y divide-hairline/60">
        {offers.map((offer) => (
          <OfferRow key={offer.id} offer={offer} onAnswer={(add) => void desktop.answer(offer, add)} />
        ))}
      </ul>
      {desktop.note && <p className="px-5 pb-3 text-xs text-danger">{desktop.note}</p>}
    </section>
  );
}

/** F33 (0.5.1 PL1 C3): "Where we looked", in the page under Your apps'
 * rows, closed until opened (PAT-SER-7): each place the scan checked, and
 * plainly what it couldn't fully check. Nothing when the scan names none. */
export function WhereWeLooked({ probes }: { probes: ProbeRowData[] }): JSX.Element | null {
  if (probes.length === 0) return null;
  return (
    <Disclosure summary="Where we looked" testId="first-run-looked">
      <ul className="mt-2 flex flex-col gap-1.5 rounded-xl bg-surface px-4 py-3 text-xs" data-testid="first-run-looked-list">
        {probes.map((probe) => (
          <li key={probe.label} className="leading-relaxed">
            <span className="text-ink">{probe.label}</span>
            <span className="text-muted">: {probe.detail}</span>
            {probe.health !== "ok" && (
              <span className={"block " + (probe.health === "partial" ? "text-warning" : "text-danger")}>
                {probe.health === "partial" ? "Partly checked" : "Couldn't check"}
                {probe.reason ? `: ${probe.reason}` : ""}
              </span>
            )}
          </li>
        ))}
      </ul>
    </Disclosure>
  );
}

/** The two ways in; `tools` are the ones that can be used where they
 * are (ADR-0018), which name what "the same" is the same as. */
function placements(tools: string[], one: boolean): { id: Placement; title: string; detail: string }[] {
  return [
    {
      id: "in_place",
      title: one ? "Use it where it is" : "Use them where they are",
      detail: `Same sign-in, settings and history ${sameAsWords(tools)}. AI Juggler adds nothing of its own to ${one ? "it" : "them"}.`,
    },
    {
      id: "copy",
      title: one ? "Make a separate copy" : "Make separate copies",
      detail: one
        ? "A fresh copy for AI Juggler only; your original stays as it is. The copy needs one sign-in."
        : "A fresh copy for AI Juggler only; your originals stay as they are. Each copy needs one sign-in.",
    },
  ];
}

/** ADR-0018's choice, under More options (AJ-E1545: "Use them where they
 * are" is the default, said once in the subtitle). */
export function PlacementChoice({ tools, one, value, busy, open, onChange }: { tools: string[]; one: boolean; value: Placement; busy: boolean; open?: boolean; onChange: (placement: Placement) => void }): JSX.Element {
  return (
    <Disclosure summary={<span className="font-medium">More options</span>} open={open} testId="first-run-more-options">
      <div role="radiogroup" aria-label="How AI Juggler uses them" className="grid gap-2 sm:grid-cols-2" data-testid="first-run-placement">
        {placements(tools, one).map((option) => (
          <label
            key={option.id}
            htmlFor={`first-run-placement-${option.id}-radio`}
            className={"flex cursor-pointer gap-3 rounded-xl border px-4 py-3 " + (value === option.id ? SELECTED_CARD : "border-hairline bg-surface hover:bg-surface-2/60")}
            data-testid={`first-run-placement-${option.id}`}
          >
            <Radio
              id={`first-run-placement-${option.id}-radio`}
              name="first-run-placement"
              value={option.id}
              aria-labelledby={`first-run-placement-${option.id}-title`}
              aria-describedby={`first-run-placement-${option.id}-detail`}
              checked={value === option.id}
              disabled={busy}
              onChange={() => onChange(option.id)}
              className="mt-0.5 shrink-0"
            />
            <span className="flex flex-col gap-0.5">
              <span id={`first-run-placement-${option.id}-title`} className="text-sm font-medium text-ink">
                {option.title}
                {option.id === "in_place" && <span className="ml-1.5 text-xs font-normal text-muted">Recommended</span>}
              </span>
              <span id={`first-run-placement-${option.id}-detail`} className="text-xs leading-relaxed text-muted">{option.detail}</span>
            </span>
          </label>
        ))}
      </div>
    </Disclosure>
  );
}
