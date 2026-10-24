/**
 * AJ-E2108 — a session's last messages, on its page only (never in rows,
 * the tray or notifications). Read from the end of its transcript when
 * the page is first shown, with keys and tokens taken out; kept nowhere,
 * not even here once the page closes. A chat its app keeps on its
 * servers has no transcript on this Mac: its title is all there is.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { Card } from "../../components/primitives";
import { conversationsLastMessages, type LastMessagesInfo } from "../../lib/ipc/conversations";
import { errorText } from "../../lib/bridge";
import { appWords, isDesktopApp, speakerName, type SessionModel } from "./model";

/** Why there's nothing to show, in words. */
const NONE: Record<string, string> = {
  none: "There's no transcript of it on this Mac to read, so its title is all AI Juggler shows.",
  compressed: "Its transcript is stored compressed now that it has ended, so it isn't read here.",
  unreadable: "Its transcript couldn't be read just now.",
};

function openSettings(): void {
  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: { section: "sessions" } }));
}

export function LastMessages({ model, visible, remoteMac }: { model: SessionModel; visible: boolean; remoteMac?: string | null }): JSX.Element {
  const [info, setInfo] = useState<LastMessagesInfo | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const { profileId, conversationId } = model;
  const key = `${profileId}\u0000${conversationId ?? ""}`;
  const read = useRef<string | null>(null);
  // Read once the page is shown, not before; again only for another
  // session (or when a read was cut short by leaving the page).
  useEffect(() => {
    if (!visible || remoteMac || conversationId === null || read.current === key) return;
    read.current = key;
    setInfo(null);
    setFailed(null);
    let alive = true;
    let answered = false;
    conversationsLastMessages(profileId, conversationId).then(
      (next) => {
        answered = true;
        if (alive) setInfo(next);
      },
      (cause) => {
        answered = true;
        if (alive) setFailed(errorText(cause));
      },
    );
    return () => {
      alive = false;
      if (!answered) read.current = null;
    };
  }, [visible, remoteMac, key, profileId, conversationId]);

  const speaker = speakerName(model.tool);
  const place = model.places[0];
  const app = appWords(place, model.tool);
  return (
    <Card className="flex flex-col gap-3 p-4" data-testid="last-messages">
      <div className="flex items-baseline justify-between gap-2">
        <h3 className="text-[11px] font-medium uppercase tracking-wider text-muted">Last messages</h3>
        {info?.state === "shown" && (
          <span className="text-[10.5px] text-muted" data-testid="last-messages-note">
            read from the session's transcript when you open it · nothing kept
            {info.redacted > 0 && ` · ${info.redacted === 1 ? "1 key" : `${info.redacted} keys`} taken out`}
          </span>
        )}
      </div>
      {remoteMac ? (
        <p className="text-[12px] text-muted" data-testid="last-messages-remote">
          Open on {remoteMac} to read: its messages stay on that Mac.
        </p>
      ) : conversationId === null ? (
        <p className="text-[12px] text-muted">It hasn't written anything yet.</p>
      ) : failed ? (
        <p className="text-[12px] text-muted" role="status">
          Couldn't read them: {failed}
        </p>
      ) : info === null ? (
        <div className="flex flex-col gap-2" aria-label="Loading the last messages" role="status">
          <span className="h-10 w-2/3 rounded-xl bg-surface-3 motion-safe:animate-pulse" />
          <span className="ml-auto h-8 w-1/2 rounded-xl bg-surface-3 motion-safe:animate-pulse" />
        </div>
      ) : info.state === "off" ? (
        <p className="text-[12px] text-muted" data-testid="last-messages-off">
          Last messages are off.{" "}
          <button type="button" onClick={openSettings} className="text-ink underline decoration-hairline underline-offset-2 outline-none hover:decoration-ink focus-visible:ring-1 focus-visible:ring-accent/60">
            Turn them on in Settings
          </button>
          .
        </p>
      ) : info.state !== "shown" || info.turns.length === 0 ? (
        <p className="text-[12px] text-muted" data-testid="last-messages-none" data-state={info.state}>
          {NONE[info.state] ?? "The end of its transcript holds no messages."}
        </p>
      ) : (
        <ol className="flex flex-col gap-2.5" data-testid="last-messages-turns">
          {info.turns.map((turn, index) => {
            const you = turn.speaker === "you";
            const asks = !you && model.state.asks && index === info.turns.length - 1;
            return (
              <li key={index} className={"flex max-w-[88%] flex-col gap-1 " + (you ? "ml-auto items-end" : "items-start")} data-testid="last-turn" data-speaker={turn.speaker} data-asks={asks ? "true" : undefined}>
                <span className={"text-[10px] font-semibold uppercase tracking-wider " + (asks ? "text-warning" : "text-muted")}>
                  {you ? "You" : asks ? `${speaker} asks` : speaker}
                </span>
                <p
                  className={
                    "whitespace-pre-wrap break-words rounded-2xl border px-3 py-2 text-[12.5px] leading-relaxed text-ink " +
                    (you ? "border-accent/25 bg-accent-soft" : asks ? "border-warning/40 bg-warning-soft" : "border-hairline bg-surface-2")
                  }
                >
                  {turn.text}
                </p>
              </li>
            );
          })}
        </ol>
      )}
      {model.live && app && !remoteMac && (
        <p className="border-t border-hairline/60 pt-2.5 text-[11.5px] text-muted" data-testid="last-messages-answer">
          Answer it in {app}. Go to it brings {place?.label || app} to the front
          {isDesktopApp(place, model.tool) ? "; if another session is showing there, pick this one in its sidebar." : "."}
        </p>
      )}
    </Card>
  );
}
