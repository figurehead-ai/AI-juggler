/**
 * AJ-E1545 step 3, "Meet Bot" (the maintainer's amendment): what Bot does
 * in plain words — it answers about your sessions, usage and apps, cites
 * every answer, and never acts — then where Bot's engine is set up.
 *
 * The engine setup is the one shared component (AJ-E3823: Apple's AI,
 * the local model, the side-by-side choice), as in Bot's panel and
 * Settings › AI › Models, with its Skip for now; this step never offers
 * an engine or a download of its own. With Bot ready, one line says so.
 */
import type { JSX } from "react";
import { EngineSetup } from "../ai/EngineSetup";
import { Pill } from "../Pill";
import { IconBot } from "../bot/bot-icons";
import { IconCheck } from "../icons-controls";
import { IconSessions, IconShield, IconUsage } from "../icons";
import { useAiJob, isShown, type AiJobInfo } from "../../lib/ai-availability";

const DOES = [
  { icon: IconSessions, title: "Answers about your work", text: "What needs you, what changed in a project, which sessions ran today, how much each account has left." },
  { icon: IconUsage, title: "Cites every answer", text: "Each answer links the sessions, tasks and accounts it comes from, so you can check it in one click." },
  { icon: IconShield, title: "Never acts", text: "Bot reads and answers. It doesn't start, stop or send anything; you do." },
] as const;

/** Where Bot stands, in the step's words (and its value beside it). */
export function botStanding(chat: AiJobInfo | null): "ready" | "waiting" | "setup" {
  if (chat?.state === "ready") return "ready";
  if (isShown(chat)) return "waiting";
  return "setup";
}

/** Bot ready: said once; else the shared engine setup (AJ-E3823). */
function BotEngine({ onSkip, onChosen }: { onSkip: () => void; onChosen: () => void }): JSX.Element {
  const chat = useAiJob("chat");
  const standing = botStanding(chat);
  if (standing === "ready") {
    return (
      <output className="flex items-center gap-2 rounded-[14px] bg-surface p-5 text-body font-medium text-ink inset-ring-1 inset-ring-hairline" data-testid="setup-bot-engine" data-standing={standing}>
        <IconCheck width={16} height={16} strokeWidth={2.4} aria-hidden="true" className="text-success" />
        Bot is ready{chat?.model_name ? `, on ${chat.model_name}` : ""}.
      </output>
    );
  }
  return (
    <div className="rounded-[14px] bg-surface p-5 inset-ring-1 inset-ring-hairline" data-testid="setup-bot-engine" data-standing={standing}>
      <EngineSetup place="wizard" onSkip={onSkip} onChosen={onChosen} />
    </div>
  );
}

export function MeetBotStep({ onSkip, onChosen }: { onSkip: () => void; onChosen: () => void }): JSX.Element {
  return (
    <div className="flex flex-col gap-5" data-testid="setup-bot">
      <div className="flex items-start gap-3">
        <span aria-hidden="true" className="flex size-10 shrink-0 items-center justify-center rounded-[10px] bg-accent-soft text-accent-hi">
          <IconBot width={22} height={22} />
        </span>
        <p className="text-body text-ink-2">
          Ask Bot in plain words about your sessions, your usage and your apps, from anywhere with ⌘J. To find a session or a task, Search (⌘K) is the quicker way.{" "}
          <Pill tone="neutral">Beta</Pill>
        </p>
      </div>
      <ul className="grid gap-3 sm:grid-cols-3">
        {DOES.map(({ icon: Icon, title, text }) => (
          <li key={title} className="flex flex-col gap-2 rounded-[14px] bg-surface p-4">
            <span aria-hidden="true" className="flex size-[30px] items-center justify-center rounded-lg bg-surface-3 text-ink-2">
              <Icon width={16} height={16} />
            </span>
            <span className="text-body font-semibold text-ink">{title}</span>
            <span className="text-meta text-muted">{text}</span>
          </li>
        ))}
      </ul>
      <BotEngine onSkip={onSkip} onChosen={onChosen} />
    </div>
  );
}
