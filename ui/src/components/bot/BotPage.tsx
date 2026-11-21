/**
 * D-136 point 4 — the Bot page, a destination like Watches: the same one
 * conversation, larger, with Search in its head and "Back to the panel"
 * (the same place in the conversation); What Bot remembers, a tab of the
 * page (ADR-0037 §9 as amended); and a side column with a short memory
 * summary and the watches Bot set. The approved mock-up's dashed "Later ·
 * Longer tasks" card is left out by the maintainer's one change: it drew
 * a feature that doesn't exist yet (D-127); it returns with 0.6's.
 */
import { useEffect, useState, type JSX } from "react";
import { IconChevronRight } from "../icons-controls";
import { IconMore } from "../icons-canvas";
import { Segmented } from "../controls";
import { BotConversation } from "./BotConversation";
import { BotComposer } from "./BotComposer";
import { BotEmpty, BotNoModel } from "./BotEmpty";
import { BotFindBar, BotMenu } from "./BotHead";
import { BotMemories } from "./BotMemories";
import { BotClearSheet, BotForgetHost } from "./BotSheets";
import { IconBookmark, IconBot, IconCollapse, IconEye, IconFolder, IconSearch } from "./bot-icons";
import { WORDS } from "../../lib/bot-words";
import { backToPanel, leftPage, openBot, openFind, useBot } from "../../lib/bot-store";
import { useMemories } from "../../lib/bot-memory";
import { useAiJob } from "../../lib/ai-availability";
import { watchesNewest, type WatchLine } from "../../lib/ipc/watches";

type Tab = "conversation" | "memory";

function SideCard({ title, link, onLink, children }: { title: string; link?: string; onLink?: () => void; children: JSX.Element | JSX.Element[] | null }): JSX.Element {
  return (
    <section className="flex flex-col gap-2.5 rounded-[14px] bg-surface p-3.5 inset-ring-1 inset-ring-hairline/75" aria-label={title}>
      <div className="flex items-center">
        <h3 className="grow text-section font-semibold text-ink">{title}</h3>
        {link && onLink && (
          <button type="button" className="inline-flex items-center gap-1 rounded-md text-meta font-medium text-accent-hi outline-none hover:underline focus-visible:ring-2 focus-visible:ring-accent/60" onClick={onLink}>
            {link}
            <IconChevronRight width={12} height={12} aria-hidden="true" />
          </button>
        )}
      </div>
      {children}
    </section>
  );
}

function Side({ onMemories }: { onMemories: () => void }): JSX.Element {
  const { info } = useMemories();
  const [watches, setWatches] = useState<WatchLine[] | null>(null);
  useEffect(() => {
    void watchesNewest(3).then(setWatches).catch(() => setWatches([]));
  }, []);
  const items = info?.items ?? [];
  return (
    <aside className="flex min-h-0 flex-col gap-3.5" aria-label="Beside the conversation">
      <SideCard title={WORDS.whatBotRemembers} link={items.length > 0 ? WORDS.all(items.length) : undefined} onLink={onMemories}>
        {items.length === 0 ? (
          <span className="text-meta text-muted">Nothing yet. Say “Remember: …” to keep a name or a note.</span>
        ) : (
          items.slice(0, 3).map((m) => (
            <div key={m.key} className="flex items-start gap-2.5 text-meta leading-snug text-ink-2">
              {m.slot === "focus" ? <IconFolder width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" /> : <IconBookmark width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" />}
              <span>
                {m.slot === "alias" && m.about ? (
                  <>
                    You call “{m.about}” <b className="font-medium text-ink">{m.text}</b>
                  </>
                ) : m.slot === "focus" && m.about ? (
                  `“${m.about}” matters most`
                ) : (
                  m.text
                )}
              </span>
            </div>
          ))
        )}
      </SideCard>
      {watches && watches.length > 0 && (
        <SideCard title={WORDS.watchesBotSet}>
          {watches.map((w) => (
            <div key={w.id} className="flex items-start gap-2.5 text-meta leading-snug text-ink-2">
              <IconEye width={14} height={14} aria-hidden="true" className="mt-px shrink-0 text-faint" />
              <span>{w.words}</span>
            </div>
          ))}
        </SideCard>
      )}
    </aside>
  );
}

export function BotPage({ onBack }: { onBack: () => void }): JSX.Element {
  const bot = useBot();
  const job = useAiJob("chat");
  const [tab, setTab] = useState<Tab>("conversation");
  const [now] = useState(() => Date.now());
  const [menu, setMenu] = useState(false);
  const [clearing, setClearing] = useState(false);
  const { info } = useMemories();
  const noModel = job?.state === "no_engine";
  useEffect(() => {
    openBot();
    const toMemory = () => setTab("memory");
    window.addEventListener("aij:bot-memories", toMemory);
    return () => {
      window.removeEventListener("aij:bot-memories", toMemory);
      leftPage();
    };
  }, []);
  const back = () => {
    backToPanel();
    onBack();
  };
  return (
    <div className="flex h-full min-h-0 flex-col" data-testid="bot-page">
      <div className="relative flex shrink-0 flex-wrap items-center gap-2.5 pb-3.5">
        <span className="grid size-10 place-items-center rounded-xl bg-accent-soft text-accent-hi">
          <IconBot width={20} height={20} aria-hidden="true" />
        </span>
        <div className="flex flex-col gap-0.5">
          <h2 className="text-title text-ink">{WORDS.title}</h2>
          <span className="text-meta whitespace-nowrap text-muted max-[1100px]:hidden">{WORDS.pageLead}</span>
        </div>
        <Segmented
          className="ml-[18px]"
          label="Bot"
          value={tab}
          onChange={setTab}
          segments={[
            { value: "conversation", label: WORDS.tabConversation },
            { value: "memory", label: `${WORDS.tabMemory} ${info?.items.length ?? ""}`.trim() },
          ]}
          testId="bot-tabs"
        />
        <span className="grow" />
        {tab === "conversation" && (
          <button type="button" className="flex h-8 w-[190px] items-center gap-2 rounded-[10px] max-[1100px]:hidden bg-fill px-2.5 text-meta text-faint inset-ring-1 inset-ring-hairline/75 outline-none focus-visible:ring-2 focus-visible:ring-accent/60" onClick={openFind} data-testid="bot-page-search">
            <IconSearch width={14} height={14} aria-hidden="true" />
            Search
          </button>
        )}
        <button type="button" className="inline-flex h-7 items-center gap-1.5 rounded-lg bg-surface-2 px-2.5 text-meta font-medium text-ink inset-ring-1 inset-ring-hairline outline-none hover:bg-surface-3 focus-visible:ring-2 focus-visible:ring-accent/60" onClick={back} data-testid="bot-back-to-panel">
          <IconCollapse width={14} height={14} aria-hidden="true" />
          {WORDS.backToPanel}
        </button>
        <button type="button" className="grid size-[30px] place-items-center rounded-lg text-muted outline-none hover:bg-surface-2 hover:text-ink focus-visible:ring-2 focus-visible:ring-accent/60" aria-label={WORDS.moreLabel} aria-haspopup="menu" aria-expanded={menu} onClick={() => setMenu((m) => !m)}>
          <IconMore width={16} height={16} aria-hidden="true" />
        </button>
        {menu && <BotMenu job={job} at="right-0 top-12" onClose={() => setMenu(false)} onClear={() => setClearing(true)} onMemories={() => setTab("memory")} />}
      </div>
      {tab === "memory" ? (
        <BotMemories />
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_308px] gap-x-7 max-[1180px]:grid-cols-1">
          <section className="flex min-h-0 flex-col overflow-hidden rounded-2xl bg-surface inset-ring-1 inset-ring-hairline/75" aria-label={WORDS.tabConversation}>
            {bot.find && <BotFindBar />}
            <BotConversation now={now} wide empty={noModel ? <BotNoModel /> : <BotEmpty job={job} />} />
            <div className="px-3 pb-1">
              <BotComposer />
            </div>
          </section>
          <div className="max-[1180px]:hidden">
            <Side onMemories={() => setTab("memory")} />
          </div>
        </div>
      )}
      <BotForgetHost />
      {clearing && <BotClearSheet onClose={() => setClearing(false)} onMemories={() => setTab("memory")} />}
    </div>
  );
}
