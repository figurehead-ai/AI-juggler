/**
 * D-97 (1) — Home's "Next steps", while nothing has run yet: each tool
 * first run's scan found that has no App yet ("Codex is installed: sign in
 * to add it"), one click to Add an app for that tool — its found step, or
 * a fresh App and its sign-in (lib/found-tools). A tool left unticked
 * never just disappears (the AJ-E3307 persona round).
 *
 * D-117, D-118's mock: a setup prompt never sits above what needs you. It
 * is the right column's last section, under Your apps and Runway by
 * account, folded like them. An Account whose limits aren't measured says
 * so on its own Runway row, with "Measure its limits ›" (or "How it's
 * measured ›" where its tool has no check, D-103.4) measuring in place:
 * the mock's place, said once.
 *
 * The section goes once a session has run or nothing is left on it.
 */
import { useMemo, type JSX } from "react";
import { Button } from "./primitives";
import { HomeSection } from "./HomeSection";
import { ToolMark } from "./ToolMark";
import { addAppFor, notBroughtIn, rememberedFound } from "../lib/found-tools";
import { providerDisplayName } from "../lib/providers";
import { firstRunToolIds } from "../lib/tools";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";

/** What a found tool's row says. */
function foundToolWords(tool: string, setUp: boolean): string {
  const name = providerDisplayName(tool);
  return setUp ? `${name} has a setup on this computer: bring it in` : `${name} is installed: sign in to add it`;
}

export function HomeNext({ accounts, sessions }: { accounts: AccountInfo[]; sessions: InstanceInfo[] }): JSX.Element | null {
  const appTools = accounts.map((account) => account.provider);
  const toolsKey = [...new Set(appTools)].sort().join(",");
  // Read again when an App of a new tool comes in (its row then goes).
  // eslint-disable-next-line react-hooks/exhaustive-deps -- keyed by the tools
  const tools = useMemo(() => notBroughtIn(rememberedFound(), appTools, firstRunToolIds()), [toolsKey]);
  if (sessions.length > 0 || tools.length === 0) return null;
  return (
    <HomeSection id="home-next" label="Next steps" count={tools.length}>
      <ul className="flex flex-col rounded-xl bg-surface px-4 py-1.5" data-testid="home-next">
        {tools.map(({ tool, setUp }) => (
          <li key={`tool-${tool}`} className="flex flex-wrap items-center gap-x-3 gap-y-1.5 border-t border-hairline/60 py-2 first:border-t-0" data-testid="home-next-tool" data-tool={tool}>
            <ToolMark tool={tool} size={16} className="shrink-0 text-muted" />
            <span className="min-w-0 flex-1 basis-40 text-meta text-ink">{foundToolWords(tool, setUp)}</span>
            <Button size="sm" variant="secondary" className="ml-auto" aria-label={`${setUp ? "Bring in" : "Add"} ${providerDisplayName(tool)}`} onClick={() => addAppFor(tool)}>
              {setUp ? "Bring in" : "Add"}
            </Button>
          </li>
        ))}
      </ul>
    </HomeSection>
  );
}
