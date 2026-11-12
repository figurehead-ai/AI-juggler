/**
 * D-97 (1) — Home's "Next steps", while nothing has run yet: what setup
 * left to do, each one click from its flow. After first run Home read
 * "Ready when you are · Not measured yet", and a tool left unticked just
 * disappeared (the AJ-E3307 persona round). It lists:
 *
 * - each tool first run's scan found that has no App yet ("Codex is
 *   installed: sign in to add it"), opening Add an app for that tool —
 *   its found step, or a fresh App and its sign-in (lib/found-tools);
 * - each signed-in Account whose limits aren't measured, with "Measure
 *   its limits", which opens the Account, where they're measured (the
 *   Runway card's own door).
 *
 * Rows as Home's other lists draw them (the Parked card's); the card
 * goes once a session has run or nothing is left on it.
 */
import { useMemo, type JSX } from "react";
import { AccountAvatar } from "./Avatar";
import { Button } from "./primitives";
import { SectionTitle } from "./SectionTitle";
import { MeasureLimits } from "./MeasureLimits";
import { ToolMark } from "./ToolMark";
import { accountGroups, groupRunway } from "./HomeRunway";
import { signedIn } from "../lib/bridge";
import { addAppFor, notBroughtIn, rememberedFound } from "../lib/found-tools";
import type { QuotaDisplayData } from "../lib/ipc/quota";
import { providerDisplayName } from "../lib/providers";
import type { SubscriptionInfo } from "../lib/subscriptions";
import { firstRunToolIds } from "../lib/tools";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** What a found tool's row says. */
function foundToolWords(tool: string, setUp: boolean): string {
  const name = providerDisplayName(tool);
  return setUp ? `${name} has a setup on this computer: bring it in` : `${name} is installed: sign in to add it`;
}

const ROW = "flex flex-wrap items-center gap-x-3.5 gap-y-2 border-t border-hairline/60 px-4 py-2.5 first:border-t-0";

export function HomeNext({
  accounts,
  subscriptions,
  quota,
  quotaKnown,
  sessions,
}: {
  accounts: AccountInfo[];
  subscriptions: SubscriptionInfo[] | null;
  quota: QuotaDisplayData[];
  quotaKnown: boolean;
  sessions: InstanceInfo[];
}): JSX.Element | null {
  const appTools = accounts.map((account) => account.provider);
  const toolsKey = [...new Set(appTools)].sort().join(",");
  // Read again when an App of a new tool comes in (its row then goes).
  // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed by the tools
  const tools = useMemo(() => notBroughtIn(rememberedFound(), appTools, firstRunToolIds()), [toolsKey]);
  // Only once the readings have answered: never "not measured" before.
  const unmeasured = quotaKnown
    ? accountGroups(accounts.filter((account) => signedIn(account.login_state)), subscriptions).filter((group) => {
        const run = groupRunway(group, quota, sessions);
        return run.left === null && !run.credits && run.words.startsWith("Not measured");
      })
    : [];
  if (sessions.length > 0 || tools.length + unmeasured.length === 0) return null;
  return (
    <section aria-labelledby="home-next" className="flex flex-col gap-2.5" data-testid="home-next">
      <SectionTitle id="home-next">Next steps</SectionTitle>
      <ul className="overflow-hidden rounded-xl border border-hairline bg-surface">
        {tools.map(({ tool, setUp }) => (
          <li key={`tool-${tool}`} className={ROW} data-testid="home-next-tool" data-tool={tool}>
            <ToolMark tool={tool} size={16} className="shrink-0 text-muted" />
            <span className="min-w-0 flex-1 basis-48 text-body text-ink">{foundToolWords(tool, setUp)}</span>
            <Button size="sm" variant="secondary" className="ml-auto" aria-label={`${setUp ? "Bring in" : "Add"} ${providerDisplayName(tool)}`} onClick={() => addAppFor(tool)}>
              {setUp ? "Bring in" : "Add"}
            </Button>
          </li>
        ))}
        {unmeasured.map((group) => {
          const first = group.profiles[0]!;
          return (
            <li key={`measure-${group.key}`} className={ROW} data-testid="home-next-measure">
              <AccountAvatar tone={group.subscription?.state === "signed_in" ? "colour" : "neutral"} account={group.subscription ?? { folder: first.id }} label={group.name} size={16} />
              <span className="min-w-0 flex-1 basis-48 text-body text-ink">
                {group.name}: <span className="text-ink-2">its limits aren't measured yet</span>
              </span>
              {/* 0.4.4: it measures here (the tool's own check), not a page. */}
              <span className="ml-auto">
                <MeasureLimits look="button" folders={group.profiles.map((profile) => profile.id)} tool={first.provider} name={group.name} />
              </span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
