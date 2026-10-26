/**
 * D-38 — an app's own icon, as installed on this Mac: the Claude app,
 * ChatGPT, Terminal, an editor, AI Juggler itself. One component wherever
 * an app or a place is named (the rail's rows and place headers, Home,
 * History, a project's sessions, the palette).
 *
 * - The icon is read by the backend from the app's bundle, once it is on
 *   screen, and kept in memory for every row that shows the same app.
 * - Until it arrives, a quiet square holds its place.
 * - When the app isn't installed or its icon can't be read, a clean one-
 *   letter mark stands in at every size, centred and never cut off, at
 *   12 px or more.
 * - The profile is never drawn on it (`ProfileBadge`, beside it).
 *
 * Decorative (`alt=""`): the app's name is always beside it in words.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { appIconRead, type AppIconAsk, type AppIconPixels } from "../lib/ipc/appIcons";
import { IconJuggler } from "./icons";
import { OWN_PLACE } from "../lib/rail-items";

/** The sizes an app's icon is drawn at. */
export type AppIconSize = 16 | 20 | 24 | 32;

/** The id AI Juggler's own place asks with (its own bundle). */
export const OWN_APP_ID = OWN_PLACE;

/** Icons read, by app and pixel size (`null`: none to be had). */
const icons = new Map<string, string | null>();
/** Reads under way, so twenty rows of one app ask once. */
const reading = new Map<string, Promise<string | null>>();

/** Test seam: forget every icon read. */
export function forgetAppIconsForTests(): void {
  icons.clear();
  reading.clear();
}

function keyOf(ask: AppIconAsk, px: AppIconPixels): string | null {
  if (ask.appId) return `${ask.appId}@${px}`;
  return ask.appPid !== null ? `pid:${ask.appPid}@${px}` : null;
}

/** The pixels a `size` px icon needs on this screen. */
export function pixelsFor(size: number): AppIconPixels {
  const ratio = typeof window !== "undefined" && window.devicePixelRatio > 0 ? window.devicePixelRatio : 1;
  const need = size * Math.max(1, ratio);
  return need <= 32 ? 32 : need <= 64 ? 64 : 128;
}

/** The fallback's letter: the name's first, at every size and in every
 * place (the review of 0.1.4: "C" in the rail and "Ch" on Home read as
 * two different marks). `_size` stays for callers that pass it. */
export function appLetters(name: string, _size?: number): string {
  const words = name.replace(/[^\p{L}\p{N}\s]/gu, " ").trim().split(/\s+/).filter(Boolean);
  const first = [...(words[0] ?? "?")];
  return (first[0] ?? "?").toUpperCase();
}

/** The fallback's type size: 12 px or more, small enough that its
 * letters fit inside the square with room to spare. */
export function letterSize(size: number): number {
  return Math.max(12, Math.round(size * 0.44));
}

type Read = { url: string | null; done: boolean };

/** The icon for `ask` at `px`, read once `visible`. */
export function useAppIcon(ask: AppIconAsk, px: AppIconPixels, visible: boolean): Read {
  const key = keyOf(ask, px);
  const [read, setRead] = useState<Read>(() => (key === null ? { url: null, done: true } : icons.has(key) ? { url: icons.get(key) ?? null, done: true } : { url: null, done: false }));
  const { appId, appPid } = ask;
  useEffect(() => {
    if (key === null) return setRead({ url: null, done: true });
    if (icons.has(key)) return setRead({ url: icons.get(key) ?? null, done: true });
    setRead({ url: null, done: false });
    if (!visible) return;
    let alive = true;
    let pending = reading.get(key);
    if (!pending) {
      pending = appIconRead({ appId, appPid }, px).then(
        (url) => url,
        () => null,
      );
      reading.set(key, pending);
      void pending.then((url) => {
        icons.set(key, url);
        reading.delete(key);
      });
    }
    void pending.then((url) => alive && setRead({ url, done: true }));
    return () => {
      alive = false;
    };
  }, [key, appId, appPid, px, visible]);
  return read;
}

/** Whether `element` is on screen yet (always, without an observer). */
function useOnScreen(element: React.RefObject<HTMLElement | null>, skip: boolean): boolean {
  const [seen, setSeen] = useState(skip || typeof IntersectionObserver === "undefined");
  useEffect(() => {
    if (seen || !element.current) return;
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        setSeen(true);
        observer.disconnect();
      }
    }, { rootMargin: "120px" });
    observer.observe(element.current);
    return () => observer.disconnect();
  }, [element, seen]);
  return seen;
}

/** macOS rounds an app's square at about 22.5 % of its side. */
export function macRadius(size: number): number {
  return Math.round(size * 0.225 * 10) / 10;
}

export interface AppIconProps {
  /** The host registry's id (`PlaceInfo.app_id`, a tool's `host_app`),
   * or `OWN_APP_ID`. */
  appId: string | null;
  /** The app's process, when a place knows it (an app the registry
   * doesn't list). */
  appPid?: number | null;
  /** The app's name, for the fallback's letters and the tooltip. */
  name: string;
  size?: AppIconSize;
  /** Words for the image, only where no name is beside it. */
  alt?: string;
  className?: string;
}

/** Any size: `AppIcon` keeps callers to the four sizes; a page header
 * that predates them (the session page's) draws larger through
 * `PlaceMark`. */
export function AppIconTile({ appId, appPid = null, name, size, alt = "", className = "" }: Omit<AppIconProps, "size"> & { size: number }): JSX.Element {
  const box = useRef<HTMLSpanElement | null>(null);
  const px = pixelsFor(size);
  const cached = keyOf({ appId, appPid }, px);
  const visible = useOnScreen(box, cached !== null && icons.has(cached));
  const { url, done } = useAppIcon({ appId, appPid }, px, visible);
  const [broken, setBroken] = useState(false);
  useEffect(() => setBroken(false), [url]);
  const radius = macRadius(size);
  const decorative = alt === "";
  const frame = { width: size, height: size };
  if (url && !broken) {
    return (
      <span ref={box} className={`inline-flex shrink-0 ${className}`} style={frame} title={name} data-testid="app-icon" data-app={appId ?? undefined} data-state="icon" aria-hidden={decorative ? "true" : undefined}>
        <img src={url} alt={alt} width={size} height={size} draggable={false} decoding="async" className="block select-none" style={frame} onError={() => setBroken(true)} />
      </span>
    );
  }
  if (!done) {
    return <span ref={box} aria-hidden="true" className={`inline-flex shrink-0 bg-surface-3 motion-safe:animate-pulse ${className}`} style={{ ...frame, borderRadius: radius }} data-testid="app-icon" data-app={appId ?? undefined} data-state="loading" />;
  }
  const own = appId === OWN_APP_ID;
  const glyph = Math.round(size * 0.7);
  // The letters are drawn from `data-letters` by CSS, not written as
  // text: a picture beside the name, never part of a control's label
  // (WCAG 2.5.3).
  return (
    <span
      ref={box}
      role={decorative ? undefined : "img"}
      aria-label={decorative ? undefined : alt}
      aria-hidden={decorative ? "true" : undefined}
      title={name}
      data-testid="app-icon"
      data-app={appId ?? undefined}
      data-state="letters"
      data-letters={own ? undefined : appLetters(name, size)}
      className={
        "inline-flex shrink-0 select-none items-center justify-center overflow-visible whitespace-nowrap border border-hairline bg-surface-3 font-semibold leading-none tracking-tight text-ink " +
        (own ? "" : "before:content-[attr(data-letters)] ") +
        className
      }
      style={{ ...frame, borderRadius: radius, fontSize: letterSize(size) }}
    >
      {own && <IconJuggler width={glyph} height={glyph} className="text-accent-hi" />}
    </span>
  );
}

export function AppIcon({ size = 20, ...props }: AppIconProps): JSX.Element {
  return <AppIconTile {...props} size={size} />;
}
