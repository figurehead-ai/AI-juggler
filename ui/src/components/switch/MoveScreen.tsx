/**
 * ADR-0031 (D-93), rebuilt to DESIGN-SYSTEM §8 (D-142, rich mode, size M
 * 680 px). The maintainer: "UI/UX for wizards, edit, add and create
 * screens should be easy to read, clear icons, tags… not a monochrome
 * hell".
 *
 * - `MoveHeader`, the dialog's fixed header: an icon tile, "Continue on
 *   <App>", the conversation and where it is now, and a from → to strip
 *   with both Apps' logos (the one picture of what happens). A wizard's
 *   step marks sit under it.
 * - `MoveChoice`, first in the body ("How it continues"): Move and Copy
 *   as RadioCards with icons and tags; one accent "Recommended" on Move
 *   when nothing blocks it. Blocked by what the user can clear (D-127):
 *   Move faded, how it clears said under it, and Check again.
 * - `MoveBody`: a needs-you Alert that the Apps restart (true whenever
 *   they do); "What happens" as a summary panel of four fact rows (icon,
 *   label, value); the project notes as a CheckRow; the conversations
 *   behind a Disclosure with their tags and the tool output left behind;
 *   and "What does this mean?" with the guide.
 * The words are the facts the backend reports, in plain words.
 */
import type { JSX, ReactNode, SVGProps } from "react";
import type { MoveOfferInfo } from "../../lib/ipc/switch-move";
import { GUIDE } from "../../lib/switch-words";
import { Alert } from "../Alert";
import { CheckRow, Disclosure, RadioCard, RadioCards, RowList } from "../controls";
import { IconCarryIn, IconCopy, IconGoTo, IconPage, IconReplay } from "../icons-actions";
import { IconMessage } from "../icons-canvas";
import { IconHistory } from "../icons-places";
import { Pill, StatePill } from "../Pill";
import { Section } from "../Section";
import { LogoTile, ToolLogo } from "../ToolLogo";
import { Explain } from "./Explain";

export type How = "move" | "copy";

const plural = (n: number, one: string, many: string) => (n === 1 ? one : many);

/** A line the backend wrote as a clause, shown as a sentence: capital
 * first, full stop last. */
export function sentence(text: string): string {
  const trimmed = text.trim();
  if (!trimmed) return trimmed;
  const capital = trimmed[0]!.toUpperCase() + trimmed.slice(1);
  return /[.!?…]$/.test(capital) ? capital : `${capital}.`;
}

/** A size in words ("1.2 GB"). */
export function size(bytes: number): string {
  if (bytes >= 1e9) return `${(bytes / 1e9).toFixed(1)} GB`;
  if (bytes >= 1e6) return `${Math.round(bytes / 1e6)} MB`;
  return `${Math.max(1, Math.round(bytes / 1e3))} KB`;
}

/** What the user can clear before Move can happen, each saying how. */
export function waits(offer: MoveOfferInfo): string[] {
  return offer.blocks.filter((block) => block.kind === "user_can_clear").map((block) => block.words);
}

/** The two Apps by name ("Claude app · Work"), where the offer names both
 * (it lists the one the conversation is on, then the other); else the
 * switch's own labels. */
export function placeNames(offer: MoveOfferInfo, source: string, target: string): { from: string; to: string } {
  const [from, to] = offer.restarts.length === 2 ? offer.restarts : [];
  return { from: from ?? source, to: to ?? target };
}

/** The header: icon tile, title, where it is now, from → to, step marks. */
export function MoveHeader({
  offer,
  source,
  target,
  title,
  tool,
  marks,
}: {
  offer: MoveOfferInfo;
  source: string;
  target: string;
  /** The conversation's title, when it has one. */
  title: string | null;
  /** The tool's catalog id (both sides are the same tool). */
  tool: string;
  /** A wizard's step marks. */
  marks?: ReactNode;
}): JSX.Element {
  const { from, to } = placeNames(offer, source, target);
  const kind = offer.app ? "desktop" : "cli";
  return (
    <div className="flex flex-col gap-3" data-testid="switch-move-header">
      <div className="flex items-start gap-3">
        <LogoTile icon={IconCarryIn} size={40} on />
        <div className="flex min-w-0 flex-col gap-0.5">
          <h2 className="type-section" data-testid="switch-move-title">
            Continue on {to}
          </h2>
          <p className="type-help truncate">
            {title ?? "This conversation"} · now on {from}
          </p>
        </div>
      </div>
      <div className="flex min-w-0 flex-wrap items-center gap-2 text-meta font-medium text-ink" data-testid="switch-move-strip">
        <span className="inline-flex min-w-0 items-center gap-2 rounded-full bg-surface-2 py-1 pr-3 pl-1">
          <ToolLogo tool={tool} kind={kind} size={24} />
          <span className="truncate">{from}</span>
        </span>
        <svg aria-hidden="true" width="40" height="12" viewBox="0 0 40 12" className="shrink-0 text-faint">
          <path d="M1 6h36M32 1.5 37 6l-5 4.5" fill="none" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
        <span className="sr-only">to</span>
        <span className="inline-flex min-w-0 items-center gap-2 rounded-full bg-accent-soft py-1 pr-3 pl-1">
          <ToolLogo tool={tool} kind={kind} size={24} />
          <span className="truncate">{to}</span>
        </span>
      </div>
      {marks}
    </div>
  );
}

/** "How it continues": Move or Copy, first in the body. */
export function MoveChoice({
  offer,
  value,
  onChange,
  target,
  source,
  onCheckAgain,
}: {
  offer: MoveOfferInfo;
  value: How;
  onChange: (how: How) => void;
  target: string;
  source: string;
  /** Reads the offer again (Move waits on something that may be done now). */
  onCheckAgain?: () => void;
}): JSX.Element {
  const ready = offer.blocks.length === 0;
  const why = waits(offer);
  return (
    <Section title="How it continues" testId="switch-how">
      <RadioCards label="How it continues" value={value} onChange={(how) => onChange(how === "move" ? "move" : "copy")} columns={2} testId="switch-how-cards">
        <RadioCard
          value="move"
          logo={<LogoTile icon={IconCarryIn} size={32} on={value === "move"} />}
          title={
            <span className="inline-flex flex-wrap items-center gap-x-2 gap-y-1">
              Move
              {ready && <Pill tone="accent">Recommended</Pill>}
            </span>
          }
          sub={`The same conversation continues on ${target}. ${source} no longer lists it.`}
          disabled={!ready}
          reason={why[0] && sentence(why[0])}
          testId="switch-how-move"
        >
          {ready ? (
            <span className="mt-1.5 flex flex-wrap gap-1.5">
              <StatePill variant="neutral">No extra space</StatePill>
              <StatePill variant="neutral">Undo available</StatePill>
            </span>
          ) : (
            why[0] && <span className="mt-1 text-meta text-ink-2">{sentence(why[0])}</span>
          )}
        </RadioCard>
        <RadioCard
          value="copy"
          logo={<LogoTile icon={IconCopy} size={32} on={value === "copy"} />}
          title="Copy"
          // 0.4.1 (the rc6 walk): the original is said once, by the copy's
          // own screen below, so this says only where the copy goes.
          sub={`A copy continues on ${target}.`}
          testId="switch-how-copy"
        >
          <span className="mt-1.5 flex flex-wrap gap-1.5">
            <StatePill variant="neutral">Uses {size(offer.bytes)}</StatePill>
          </span>
        </RadioCard>
      </RadioCards>
      {!ready && why.length > 1 && (
        <ul className="ml-4 list-disc text-meta text-ink-2" data-testid="switch-how-why">
          {why.slice(1).map((line) => (
            <li key={line}>{sentence(line)}</li>
          ))}
        </ul>
      )}
      {!ready && onCheckAgain && (
        <button type="button" onClick={onCheckAgain} className="w-fit text-meta font-medium text-accent-hi underline-offset-2 hover:underline" data-testid="switch-how-again">
          Check again
        </button>
      )}
    </Section>
  );
}

type Icon = (props: SVGProps<SVGSVGElement>) => JSX.Element;

/** One fact row of the summary panel: icon, label, value. */
function Fact({ icon: Glyph, label, children, testId }: { icon: Icon; label: string; children: ReactNode; testId: string }): JSX.Element {
  return (
    <div className="grid grid-cols-[16px_5.5rem_minmax(0,1fr)] items-start gap-x-3 py-2" data-testid={testId}>
      <Glyph aria-hidden="true" width={16} height={16} className="mt-0.5 text-ink-2" />
      <dt className="text-meta text-muted">{label}</dt>
      <dd className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-body text-ink">{children}</dd>
    </div>
  );
}

/** A conversation's tags: what is special about it, in a word or two. */
function tags(member: MoveOfferInfo["members"][number], target: string): string[] {
  const said: string[] = [];
  if (member.side === "relinked") said.push(`Already on ${target}`);
  if (member.archived) said.push("Archived");
  if (member.folder_missing) said.push("Folder gone");
  return said;
}

const TIPS = [
  "Move: the conversation itself goes to the other App, with every message and sub-session. Nothing is copied, and Undo puts both back.",
  "Copy: a new copy continues there and the original stays where it is.",
  "Both Apps quit and reopen during a move, so anything running in them stops.",
];

/** What Move does, under the choice. */
export function MoveBody({
  offer,
  target,
  source,
  tool,
  takeNotes,
  onTakeNotes,
}: {
  offer: MoveOfferInfo;
  target: string;
  source: string;
  tool: string;
  takeNotes: boolean;
  onTakeNotes: (take: boolean) => void;
}): JSX.Element {
  const moving = offer.members.filter((m) => m.side === "moves");
  const others = moving.length - 1;
  const restarts = offer.app ? offer.restarts.join(" and ") : "";
  const archived = moving.filter((m) => m.archived).length;
  const gone = moving.filter((m) => m.folder_missing).length;
  return (
    <div className="flex flex-col gap-5" data-testid="switch-move">
      {restarts && (
        <Alert level="needs-you" title={offer.restarts.length === 1 ? `${restarts} restarts.` : "Both Apps restart."} testId="switch-move-restart" live={null}>
          {restarts} quit and start again. Anything running in {offer.restarts.length === 1 ? "it" : "them"} stops, including other conversations&apos; work: start it again after the move.
        </Alert>
      )}
      <Section title="What happens">
        <dl className="flex flex-col divide-y divide-hairline rounded-card bg-surface-2 px-4 py-1">
          <Fact icon={IconMessage} label="Moves" testId="switch-move-what">
            <span>
              {others > 0 ? `The conversation and its ${others} ${plural(others, "session", "sessions")}` : "The conversation"} · {size(offer.bytes)}, with every message, sub-session and rewind point
            </span>
            {archived > 0 && <StatePill variant="neutral">{archived} archived</StatePill>}
            {gone > 0 && <StatePill variant="neutral">{gone} folder{gone === 1 ? "" : "s"} gone</StatePill>}
          </Fact>
          <Fact icon={IconHistory} label="Original" testId="switch-move-original">
            {source} no longer lists it; {target} does, as the same conversation. Undo puts both back exactly.
          </Fact>
          {restarts && (
            <Fact icon={IconReplay} label="Restarts" testId="switch-move-restarts">
              {restarts}
            </Fact>
          )}
          <Fact icon={IconGoTo} label="After" testId="switch-move-model">
            It opens in {target}. AI Juggler sends no message; send one to check it answers on {target}.
          </Fact>
        </dl>
      </Section>
      {offer.notes.length > 0 && (
        <Alert level="info" live={null} testId="switch-move-notes">
          {offer.notes.map((note) => (
            <p key={note}>{sentence(note)}</p>
          ))}
        </Alert>
      )}
      {offer.notes_conflict && (
        <RowList>
          <CheckRow
            icon={IconPage}
            title={`Use this conversation's project notes on ${target}`}
            help={`Every conversation of this project there reads them. Off: ${target} keeps its own; either way, Undo restores them.`}
            checked={takeNotes}
            onChange={onTakeNotes}
            testId="switch-move-take-notes"
          />
        </RowList>
      )}
      {(offer.members.length > 1 || offer.kept_outputs > 0) && (
        <Disclosure testId="switch-move-family" summary={`${moving.length} ${plural(moving.length, "conversation moves", "conversations move")}`}>
          <ul className="mt-2 flex flex-col gap-1">
            {offer.members.map((member, index) => (
              <li key={`${member.title}-${index}`} className="flex min-w-0 flex-wrap items-center gap-x-2.5 gap-y-1 rounded-lg px-2 py-1.5 hover:bg-surface-2" data-testid="switch-move-member">
                <ToolLogo tool={tool} kind={offer.app ? "desktop" : "cli"} size={24} />
                <span className="min-w-0 flex-1 truncate text-body text-ink" title={member.title}>
                  {member.title}
                </span>
                {tags(member, target).map((tag) => (
                  <StatePill key={tag} variant="neutral">
                    {tag}
                  </StatePill>
                ))}
              </li>
            ))}
          </ul>
          {offer.kept_outputs > 0 && (
            <p className="mt-2 px-2 text-meta text-muted" data-testid="switch-move-outputs">
              {offer.kept_outputs} {plural(offer.kept_outputs, "folder", "folders")} of tool output {plural(offer.kept_outputs, "stays", "stay")} on {source}, where old messages point to {plural(offer.kept_outputs, "it", "them")}. Cleanup keeps {plural(offer.kept_outputs, "it", "them")}.
            </p>
          )}
        </Disclosure>
      )}
      <Explain tips={TIPS} guide={GUIDE.move} testId="switch-move-explain" />
    </div>
  );
}
