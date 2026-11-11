/**
 * AJ-E3306 — first run's found step past the setups found (the PF1
 * persona walk on rc26): the names setups come in by, each tool installed
 * with nothing set up offered as a fresh App in its own group (pass2/03 §5
 * rule 3), and the desktop apps' profiles (D-84: offered, never added by
 * themselves).
 */
import { useMemo, useState, type JSX } from "react";
import { ToolMark } from "../components/ToolMark";
import { OfferRow, usePlaceOffers } from "../components/PlaceOffers";
import { desktopAppWord } from "../lib/app-name";
import { listWords } from "../lib/account-terms";
import { providerDisplayName, toolFound, versionNotAsked, versionUnchecked } from "../lib/providers";
import { toolSpec } from "../lib/tools";
import { signedIn } from "../lib/bridge";
import { createAccount } from "../lib/ipc/accounts";
import type { ProviderInstallData, ScanReportData } from "../lib/ipc/providers";
import type { DiscoveredInfo } from "../types/generated/DiscoveredInfo";
import { colourAt, freeLabel } from "./FreshAccountStep";
import { FreshRow, type RowProgress } from "./FirstRunFoundRow";
import type { Signin } from "./FirstRun";

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
}): JSX.Element {
  return (
    <>
      {fresh.map((tool, index) => {
        const install = installRows[tool];
        const name = providerDisplayName(tool);
        return (
          <fieldset key={freshKey(tool)} className="border-b border-hairline last:border-b-0" data-testid={`first-run-group-fresh-${tool}`}>
            <legend className="sr-only">{`${name}, not set up`}</legend>
            <div aria-hidden="true" className="flex items-center justify-between gap-3 bg-surface-2 px-5 py-2.5 text-xs">
              <span className="flex items-center gap-1.5 font-semibold text-ink">
                <ToolMark tool={tool} size={14} className="text-muted" />
                <span>{name} <span className="font-normal text-muted">· not set up</span></span>
              </span>
              {install && toolFound(install) && <span className="text-muted">{cliWords(name, install)}</span>}
            </div>
            <FreshRow
              tool={tool}
              name={names[tool] ?? name}
              toolName={name}
              colour={colourAt(colourFrom + index)}
              checked={!!picked[tool]}
              disabled={busy}
              progress={progress(tool)}
              onCheck={(checked) => onPick(tool, checked)}
            />
          </fieldset>
        );
      })}
    </>
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
 * apps offers them; nothing when neither. */
export function DesktopApps({ bundles, tools }: { bundles: string[]; tools: string[] }): JSX.Element | null {
  const desktop = usePlaceOffers();
  const apps = bundles.map((bundle) => {
    const app = bundle.replace(/\.app$/, "");
    return desktopAppWord(app, tools.find((tool) => toolSpec(tool).desktop_app === app) ?? null);
  });
  const offers = desktop.offers ?? [];
  if (offers.length === 0 && apps.length === 0) return null;
  return (
    <section className="overflow-hidden rounded-2xl border border-hairline bg-surface" aria-label="Desktop apps" data-testid="first-run-desktop">
      <h2 className="bg-surface-2 px-5 py-2.5 text-xs font-semibold text-ink">Desktop apps</h2>
      {offers.length > 0 ? (
        <ul className="divide-y divide-hairline/60">
          {offers.map((offer) => (
            <OfferRow key={offer.id} offer={offer} onAnswer={(add) => desktop.answer(offer, add)} />
          ))}
        </ul>
      ) : (
        <p className="px-5 py-3 text-sm text-ink-2" data-testid="first-run-desktop-none">
          {desktopNoneWords(apps)}
        </p>
      )}
      {desktop.note && <p className="px-5 pb-3 text-xs text-danger">{desktop.note}</p>}
    </section>
  );
}
