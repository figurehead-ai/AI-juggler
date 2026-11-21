/**
 * SK-23 — a terminal's view on its backend PTY: what the session printed
 * before the view opened (the output AI Juggler keeps while it runs),
 * then what it prints, each byte once (lib/terminal-replay joins them by
 * place). The kept output, the live listener and the backend's forwarding
 * are asked for at once, so the first paint waits on one answer; once the
 * listener and the forwarding are in place, whatever came before them is
 * asked for too. A session that printed its question in between and then
 * waited showed a blank terminal.
 */
import { onTerminalOutput, terminalAttach, terminalSnapshot } from "./ipc/terminal-io";
import { replay } from "./terminal-replay";

function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/** Output the backend sent (kept or live), as bytes and where they end. */
const spanOf = (sent: { data: string; end: number }) => ({ bytes: decodeBase64(sent.data), end: sent.end });

/** What a view shows the output with: xterm's `write`, whose `done` runs
 * once what came before it is drawn into the terminal's rows. */
export type TerminalSink = { write(data: string | Uint8Array, done?: () => void): void };

/** What a view hears of the attach: `replayed` once what the session
 * printed before is in the rows (the view says "Live" then, never over a
 * pane still blank); `missed` each time a try fails (it tries again). */
export type AttachEvents = { replayed: () => void; missed: () => void };

/** Attaches terminal `id`'s output to `term`. Until its kept output comes
 * back it tries again, each wait longer, up to 30 s (the rc5 walk: a
 * session never supervised was asked for every 2 s, for ever). Returns
 * the stop. */
export function attachTerminal(id: string, term: TerminalSink, events: AttachEvents): () => void {
  let alive = true;
  const join = replay(
    (bytes) => term.write(bytes),
    (from) => void terminalSnapshot(id, from).then((kept) => alive && join.kept(spanOf(kept)), () => alive && join.lost()),
  );
  let hearing = false;
  let forwarding = false;
  const inPlace = () => void (hearing && forwarding && join.listening());
  const stopHearing = onTerminalOutput(
    id,
    (chunk) => alive && join.heard(spanOf(chunk)),
    () => ((hearing = true), inPlace()),
  );
  let wait = 2000;
  const attach = (): void => {
    void Promise.all([terminalSnapshot(id), terminalAttach(id)]).then(
      ([kept]) => {
        if (!alive) return;
        join.kept(spanOf(kept));
        forwarding = true;
        inPlace();
        term.write("", () => alive && events.replayed());
      },
      () => {
        if (!alive) return;
        events.missed();
        window.setTimeout(attach, wait);
        wait = Math.min(wait * 2, 30_000);
      },
    );
  };
  attach();
  return () => {
    alive = false;
    stopHearing();
  };
}
