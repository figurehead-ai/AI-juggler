/**
 * 0.4.6 (D-126): How to start's side, in the preview's place: what a
 * pool does, drawn (you, its lead, the team, a check) and in four lines,
 * with the guide's link. It is the pool's intro (AJ-E3011): shown in
 * full whether or not a pool exists yet.
 */
import type { JSX } from "react";

import { IconLead, IconMessage } from "../../components/icons-canvas";
import { IconProfiles, IconShield } from "../../components/icons";
import { LogoTile } from "../../components/ToolLogo";
import { POOL_DOES } from "../../lib/pool-wizard-words";
import { POOLS_GUIDE } from "../../lib/pool-words";
import { HeroArt } from "./illustrations";
import { GuideLink } from "./StepHead";

const ICONS = { you: IconProfiles, lead: IconLead, talk: IconMessage, check: IconShield } as const;

export function NewPoolStartSide({ first }: { first: boolean }): JSX.Element {
  return (
    <aside aria-labelledby="new-pool-intro-title" className="overflow-hidden rounded-card bg-surface shadow-card inset-ring-1 inset-ring-hairline/75" data-testid="new-pool-intro" data-open="true" data-first={first || undefined}>
      <div className="flex justify-center bg-canvas-hi bg-[radial-gradient(var(--color-hairline)_1px,transparent_1.2px)] bg-[length:14px_14px] pb-3.5 pt-5">
        <HeroArt />
      </div>
      <div className="flex flex-col gap-3.5 px-5 pb-5 pt-4">
        <h4 id="new-pool-intro-title" className="type-section">
          What a pool does
        </h4>
        <ul className="flex flex-col gap-3" data-testid="new-pool-intro-lines">
          {POOL_DOES.map((one) => (
            <li key={one.id} className="flex items-start gap-3">
              <LogoTile icon={ICONS[one.id]} size={24} />
              <span className="pt-0.5 text-meta text-ink-2">{one.line}</span>
            </li>
          ))}
        </ul>
        <span>
          <GuideLink guide={POOLS_GUIDE} testId="new-pool-intro-guide">
            How pools work
          </GuideLink>
        </span>
      </div>
    </aside>
  );
}
