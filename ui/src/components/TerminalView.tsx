import { useCallback, useEffect, useId, useRef, useState } from "react";
import { FitAddon } from "@xterm/addon-fit";
import { loadGlyphRenderer, TERMINAL_FONTS } from "../lib/terminal-render";
import { SearchAddon } from "@xterm/addon-search";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { Terminal } from "@xterm/xterm";
import "@xterm/xterm/css/xterm.css";
import {
  errorText,
  hasLiveBackend,
  onTerminalExit,
  onTerminalOutput,
  terminalAttach,
  terminalResize,
  terminalSnapshot,
  terminalWrite,
} from "../lib/bridge";
import { sessionPrefsGet, sessionPrefsSet, SESSION_PREFS_DEFAULT, SESSION_PREFS_EVENT, SESSION_PREFS_RANGE, type SessionPrefsInfo } from "../lib/ipc/session-prefs";
import { followTerminalWindows, terminalOpenLink, terminalPopIn, terminalPopOut } from "../lib/ipc/terminal";
import { copyText, pathLinkProvider, terminalKey, type TerminalKey } from "../lib/terminalTools";
import { TerminalSearch } from "./TerminalSearch";
import { TerminalToolbar, keyName, type TerminalConnection } from "./TerminalToolbar";
import { Button } from "./primitives";
import { processGone } from "../lib/session-process";

const SCROLLBACK_LINES = SESSION_PREFS_DEFAULT.terminal_scrollback_lines;

/** AJ-E1505: Settings › Sessions' font size and scrollback, applied to a
 * live terminal (xterm takes both without a restart); refits after. */
export function applyTerminalPrefs(
  term: { options: { fontSize?: number; scrollback?: number } },
  prefs: Pick<SessionPrefsInfo, "terminal_font_size" | "terminal_scrollback_lines">,
  refit?: () => void,
): void {
  if (term.options.fontSize !== prefs.terminal_font_size) term.options.fontSize = prefs.terminal_font_size;
  if (term.options.scrollback !== prefs.terminal_scrollback_lines) term.options.scrollback = prefs.terminal_scrollback_lines;
  try {
    refit?.();
  } catch {
    /* a hidden panel refits when it shows */
  }
}

const TABBABLE =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

/**
 * AJ-E804 (WCAG 2.1.2, no keyboard trap): Tab and Shift+Tab belong to the
 * shell — completion, and the agent's own mode cycling — so Ctrl+Tab /
 * Ctrl+Shift+Tab leave the terminal instead. Terminals send no sequence
 * for Ctrl+Tab, so no CLI loses a key. Moves focus to the next (or
 * previous) focusable element outside `host`.
 */
function leaveTerminal(host: HTMLElement, backwards: boolean) {
  const all = Array.from(document.querySelectorAll<HTMLElement>(TABBABLE)).filter(
    (el) => el.offsetParent !== null || el === document.activeElement,
  );
  const outside = all.filter((el) => !host.contains(el));
  const firstInside = all.findIndex((el) => host.contains(el));
  const before = all.slice(0, Math.max(0, firstInside)).filter((el) => !host.contains(el));
  const after = all.slice(firstInside).filter((el) => !host.contains(el));
  const target = backwards ? (before.at(-1) ?? outside.at(-1)) : (after[0] ?? outside[0]);
  target?.focus();
}

function decodeBase64(data: string): Uint8Array {
  const binary = atob(data);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

/**
 * One xterm instance bound to a backend PTY (M1-4), embedded in the panel
 * of whatever session/login selected it: replays the retained tail on
 * (re)mount, streams live output, sends keystrokes, resizes with its
 * container. Closing and reopening a panel loses nothing.
 *
 * AJ-E1506: with the tools a terminal user expects — find, links and file
 * paths, copy, text size, jump to the bottom, a Live / Reconnecting /
 * Ended word, maximize, and its own window (`ownWindow` is that window's
 * view; the main window then says where it went).
 */

export function TerminalView({
  id,
  title,
  status,
  rule = null,
  ended = false,
  ownWindow = false,
}: {
  id: string;
  title: string;
  /** The session's status, when it is one: says whether the process ended. */
  status?: string;
  /** Its attention rule: a session at its limit may still be open. */
  rule?: string | null;
  /** A login terminal's own word that it ended. */
  ended?: boolean;
  ownWindow?: boolean;
}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const prefsRef = useRef<SessionPrefsInfo>(SESSION_PREFS_DEFAULT);
  const toolRef = useRef<(key: TerminalKey) => void>(() => {});
  const hintId = useId();
  const [search, setSearch] = useState<SearchAddon | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [maximized, setMaximized] = useState(false);
  const [scrolledUp, setScrolledUp] = useState(false);
  const [attached, setAttached] = useState(!hasLiveBackend());
  const [exited, setExited] = useState(false);
  const [poppedOut, setPoppedOut] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const [linkHint, setLinkHint] = useState<string | null>(null);
  const away = poppedOut && !ownWindow;
  const gone = ended || exited || processGone(status, rule);
  const connection: TerminalConnection = gone ? "ended" : attached ? "live" : "reconnecting";

  const say = useCallback((words: string) => {
    setNote(words);
    window.setTimeout(() => setNote((current) => (current === words ? null : current)), 3000);
  }, []);

  useEffect(() => {
    const host = hostRef.current;
    if (!host || termRef.current) return;

    const term = new Terminal({
      scrollback: SCROLLBACK_LINES,
      cursorBlink: true,
      fontSize: SESSION_PREFS_DEFAULT.terminal_font_size,
      fontFamily: TERMINAL_FONTS,
      allowProposedApi: true,
      theme: {
        background: "#05070b",
        foreground: "#cdd5e0",
        cursor: "#7c7cf4",
        selectionBackground: "#7c7cf440",
      },
    });
    const fit = new FitAddon();
    const finder = new SearchAddon();
    const openLink = (target: string) => void terminalOpenLink(id, target).catch((cause) => say(errorText(cause)));
    term.loadAddon(fit);
    term.loadAddon(finder);
    // Web addresses open on ⌘-click (Ctrl-click), as file paths do.
    term.loadAddon(new WebLinksAddon((event, uri) => (event.metaKey || event.ctrlKey) && openLink(uri), { hover: (_event, uri) => setLinkHint(uri), leave: () => setLinkHint(null) }));
    term.registerLinkProvider(pathLinkProvider(term, openLink, setLinkHint));
    term.open(host);
    loadGlyphRenderer(term, () => termRef.current === term);
    term.attachCustomKeyEventHandler((event) => {
      if (event.key === "Tab" && event.ctrlKey) {
        if (event.type === "keydown") {
          event.preventDefault();
          leaveTerminal(host, event.shiftKey);
        }
        return false;
      }
      const tool = terminalKey(event);
      if (tool) {
        if (event.type === "keydown") {
          event.preventDefault();
          event.stopPropagation();
          toolRef.current(tool);
        }
        return false;
      }
      return true;
    });
    const follow = () => setScrolledUp(term.buffer.active.viewportY < term.buffer.active.baseY);
    const scrolled = term.onScroll(follow);
    const written = term.onWriteParsed(follow);
    term.textarea?.setAttribute("aria-describedby", hintId);
    termRef.current = term;
    fitRef.current = fit;
    setSearch(finder);
    // The design preview only (no backend, never the app): the tests
    // write output and time the search.
    if (!hasLiveBackend()) {
      const seams = window as unknown as { __AIJ_TERMINALS__?: Record<string, { term: Terminal; search: SearchAddon }> };
      seams.__AIJ_TERMINALS__ = { ...seams.__AIJ_TERMINALS__, [id]: { term, search: finder } };
    }

    let alive = true;
    if (hasLiveBackend()) {
      setAttached(false);
      // Reconnecting until the retained tail is back and output streams.
      const attach = (): void => {
        void Promise.all([terminalSnapshot(id), terminalAttach(id)]).then(
          ([b64]) => {
            if (!alive) return;
            if (b64) term.write(decodeBase64(b64));
            setAttached(true);
          },
          () => alive && window.setTimeout(attach, 2000),
        );
      };
      attach();
      term.onData((data) => {
        void terminalWrite(id, data);
      });
    } else {
      term.write(
        [
          "\u001b[1;38;5;140m" + title + "\u001b[0m",
          "",
          "  ? Ask anything — this terminal is isolated to its account",
          "  and project.",
          "",
          "\u001b[38;5;245m(preview — run the app for a live terminal)\u001b[0m",
          "",
        ].join("\r\n"),
      );
    }

    const containerObserver = new ResizeObserver(() => {
      if (!host.offsetParent) return;
      try {
        const dims = fit.proposeDimensions();
        if (dims && dims.cols > 0 && dims.rows > 0) {
          term.resize(dims.cols, dims.rows);
          if (hasLiveBackend()) void terminalResize(id, dims.cols, dims.rows);
        }
      } catch {
        /* mid-teardown resizes are harmless */
      }
    });
    containerObserver.observe(host);
    // Initial fit once the host is laid out; focus, unless the user has
    // moved on within this page (the terminal's code can arrive a moment
    // after the page opened).
    requestAnimationFrame(() => {
      try {
        fitRef.current?.fit();
        if (mayTakeFocus(host)) termRef.current?.focus();
      } catch {
        /* panel mid-animation */
      }
    });

    return () => {
      alive = false;
      containerObserver.disconnect();
      scrolled.dispose();
      written.dispose();
      term.dispose();
      termRef.current = null;
      setSearch(null);
    };
  }, [id, title, hintId, away, say]);

  // AJ-E1505: the saved prefs on open, and every change while open.
  useEffect(() => {
    let alive = true;
    const apply = (prefs: SessionPrefsInfo) => {
      prefsRef.current = prefs;
      if (termRef.current) applyTerminalPrefs(termRef.current, prefs, () => fitRef.current?.fit());
    };
    void sessionPrefsGet().then((prefs) => alive && apply(prefs), () => null);
    const follow = (event: Event) => apply((event as CustomEvent<SessionPrefsInfo>).detail);
    window.addEventListener(SESSION_PREFS_EVENT, follow);
    return () => {
      alive = false;
      window.removeEventListener(SESSION_PREFS_EVENT, follow);
    };
  }, [id, away]);

  useEffect(() => {
    if (!hasLiveBackend()) return;
    const off = onTerminalOutput(id, (b64) => termRef.current?.write(decodeBase64(b64)));
    const offExit = onTerminalExit((which) => which === id && setExited(true));
    return () => {
      off();
      offExit();
    };
  }, [id]);

  useEffect(() => (ownWindow ? undefined : followTerminalWindows((ids) => setPoppedOut(ids.includes(id)))), [id, ownWindow]);

  // Esc restores a maximized terminal from anywhere but the terminal
  // itself, where Esc is the agent's (it interrupts Claude Code).
  useEffect(() => {
    if (!maximized) return;
    const restore = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !hostRef.current?.contains(event.target as Node)) setMaximized(false);
    };
    window.addEventListener("keydown", restore);
    return () => window.removeEventListener("keydown", restore);
  }, [maximized]);

  const zoom = (step: -1 | 0 | 1) => {
    const [low, high] = SESSION_PREFS_RANGE.terminal_font_size;
    const current = prefsRef.current.terminal_font_size;
    // Whole sizes from Settings' 12.5: up to 13, down to 12.
    const size = step === 0 ? SESSION_PREFS_DEFAULT.terminal_font_size : Math.min(high, Math.max(low, step > 0 ? Math.floor(current) + 1 : Math.ceil(current) - 1));
    if (size !== current) void sessionPrefsSet({ ...prefsRef.current, terminal_font_size: size }).then(() => say(`Text size ${size}`), (cause) => say(errorText(cause)));
  };
  const closeSearch = () => {
    setSearchOpen(false);
    termRef.current?.focus();
  };
  toolRef.current = (tool) => {
    if (tool === "find") setSearchOpen(true);
    else if (tool === "maximize") setMaximized((on) => !on && !ownWindow);
    else zoom(tool === "larger" ? 1 : tool === "smaller" ? -1 : 0);
  };
  const copy = () => {
    const text = termRef.current ? copyText(termRef.current) : "";
    if (!text) return say("Nothing to copy");
    void navigator.clipboard.writeText(text).then(() => say(termRef.current?.hasSelection() ? "Copied the selection" : "Copied what's on screen"), () => say("Couldn't copy"));
  };
  const toWindow = () => void (ownWindow ? terminalPopIn(id) : terminalPopOut(id, title)).catch((cause) => say(errorText(cause)));

  if (away) {
    return (
      <div className="flex h-full w-full flex-col items-center justify-center gap-3 bg-surface p-6 text-center" data-testid="terminal-away">
        <p className="text-sm text-ink">This terminal is in its own window.</p>
        <p className="max-w-sm text-xs leading-relaxed text-muted">The session keeps running and AI Juggler keeps watching it here.</p>
        <div className="flex gap-2">
          <Button size="sm" variant="secondary" onClick={() => void terminalPopOut(id, title)}>
            Show its window
          </Button>
          <Button size="sm" variant="ghost" onClick={() => void terminalPopIn(id)}>
            Bring it back here
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className={maximized ? "fixed inset-0 z-50 flex flex-col bg-[#05070b]" : "flex h-full w-full flex-col"} data-testid="terminal" data-maximized={maximized ? "true" : "false"}>
      {/* Maximized, the window's own buttons sit over this strip. */}
      {maximized && <div data-tauri-drag-region className="h-7 shrink-0 bg-surface" />}
      <TerminalToolbar
        connection={connection}
        note={note}
        searchOpen={searchOpen}
        maximized={maximized}
        ownWindow={ownWindow}
        onFind={() => (searchOpen ? closeSearch() : setSearchOpen(true))}
        onCopy={copy}
        onZoom={zoom}
        onMaximize={() => setMaximized((on) => !on)}
        onWindow={toWindow}
      />
      {searchOpen && search && <TerminalSearch search={search} onClose={closeSearch} />}
      <div className="group/term relative min-h-0 flex-1">
        <div ref={hostRef} className="h-full w-full px-3 pt-2" data-terminal-host={id} />
        {/* Shown only while the terminal has focus; always read with it. */}
        <p
          id={hintId}
          className="pointer-events-none absolute bottom-1 right-2 rounded bg-surface-3 px-1.5 py-0.5 text-label text-muted opacity-0 group-focus-within/term:opacity-100"
          data-testid="terminal-leave-hint"
        >
          ⌃Tab leaves the terminal · {keyName("F")} finds
        </p>
        {linkHint && (
          <p className="pointer-events-none absolute bottom-1 left-3 max-w-[60%] truncate rounded bg-surface-3 px-1.5 py-0.5 text-label text-muted" data-testid="terminal-link-hint">
            {/Mac/.test(navigator.platform) ? "⌘-click" : "Ctrl-click"} opens {linkHint}
          </p>
        )}
        {scrolledUp && (
          <button
            type="button"
            className="absolute bottom-7 left-1/2 -translate-x-1/2 rounded-full border border-hairline bg-surface px-3 py-1 text-label text-ink shadow-md hover:bg-surface-2 focus-visible:ring-1 focus-visible:ring-accent/60"
            onClick={() => {
              termRef.current?.scrollToBottom();
              termRef.current?.focus();
            }}
            data-testid="terminal-jump-bottom"
          >
            ↓ Jump to the bottom
          </button>
        )}
      </div>
    </div>
  );
}

/** Whether a terminal arriving now may take focus: yes from the rail, the
 * window or nothing; not from something the user has since focused in the
 * terminal's own page (Tab, a shortcut), which a late arrival would steal. */
export function mayTakeFocus(host: HTMLElement): boolean {
  // Never from a dialog over the page: it owns the keys (its focus trap).
  if (document.querySelector('[aria-modal="true"]')) return false;
  const active = document.activeElement;
  if (!active || active === document.body) return true;
  const page = host.closest("main");
  return !page || active === page || !page.contains(active) || host.contains(active);
}
