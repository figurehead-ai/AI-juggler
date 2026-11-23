/**
 * AJ-E1546 — one short clip (`lib/clips`): muted, inline, loaded only once
 * its card is on screen, paused while it is off screen, played through
 * three times and then resting (MOT-2), with a Pause / Play button that
 * shows on hover and keyboard focus (WCAG 2.2.2). Under Reduce Motion it
 * is its still poster, never moving. In the theme the window is in.
 *
 * Decorative beside its words (the card says what it shows); a clip
 * whose file is missing draws nothing, never a broken frame.
 */
import { useEffect, useRef, useState, type JSX } from "react";
import { CLIP_PLAYS, clipFiles, type ClipId, type ClipTheme } from "../lib/clips";
import { useReducedMotion } from "../lib/reduced-motion";
import { IconPause, IconPlay } from "./icons-controls";

const readTheme = (): ClipTheme => (typeof document !== "undefined" && document.documentElement.dataset.theme === "light" ? "light" : "dark");

/** The theme the window shows now (`<html data-theme>`), kept current. */
function useThemeName(): ClipTheme {
  const [theme, setTheme] = useState<ClipTheme>(readTheme);
  useEffect(() => {
    if (typeof MutationObserver === "undefined") return undefined;
    const observer = new MutationObserver(() => setTheme(readTheme()));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, []);
  return theme;
}

const NO_OBSERVER = typeof IntersectionObserver === "undefined";

/** Whether `node` is on screen now, and whether it ever was (its file
 * then loads, and stays). Without an observer: always. */
function useOnScreen(node: HTMLElement | null): { shown: boolean; seen: boolean } {
  const [state, setState] = useState({ shown: NO_OBSERVER, seen: NO_OBSERVER });
  useEffect(() => {
    if (!node || NO_OBSERVER) return undefined;
    const observer = new IntersectionObserver(([entry]) => {
      const shown = entry?.isIntersecting ?? false;
      setState((now) => ({ shown, seen: now.seen || shown }));
    }, { threshold: 0.35 });
    observer.observe(node);
    return () => observer.disconnect();
  }, [node]);
  return state;
}

export function Clip({ id, label, className = "" }: { id: ClipId; label: string; className?: string }): JSX.Element | null {
  const theme = useThemeName();
  const reduced = useReducedMotion();
  const files = clipFiles(id, theme);
  const [frame, setFrame] = useState<HTMLDivElement | null>(null);
  const { shown: onScreen, seen } = useOnScreen(frame);
  const video = useRef<HTMLVideoElement | null>(null);
  const plays = useRef(0);
  const [wanted, setWanted] = useState(true);
  const [playing, setPlaying] = useState(false);
  const [broken, setBroken] = useState(false);
  // Loaded the first time it is on screen (or played), and kept.
  const [asked, setAsked] = useState(false);
  const loaded = seen || asked;
  useEffect(() => {
    const element = video.current;
    if (!element || !loaded) return;
    if (onScreen && wanted && plays.current < CLIP_PLAYS) void element.play?.()?.catch(() => setPlaying(false));
    else element.pause?.();
  }, [onScreen, wanted, loaded]);
  if (broken) return null;
  const look = "relative aspect-[16/10] w-full overflow-hidden rounded-xl bg-surface-2 inset-ring-1 inset-ring-hairline " + className;
  if (reduced) {
    return (
      <div className={look} data-testid={`clip-${id}`} data-still="true">
        <img src={files.poster} alt="" loading="lazy" className="size-full object-cover" onError={() => setBroken(true)} />
      </div>
    );
  }
  return (
    <div ref={setFrame} className={"group " + look} data-testid={`clip-${id}`} data-playing={playing}>
      <video
        ref={video}
        aria-hidden="true"
        className="size-full object-cover"
        muted
        playsInline
        preload="none"
        poster={files.poster}
        src={loaded ? files.video : undefined}
        onPlay={() => setPlaying(true)}
        onPause={() => setPlaying(false)}
        onEnded={(event) => {
          plays.current += 1;
          setPlaying(false);
          if (plays.current < CLIP_PLAYS && wanted) {
            event.currentTarget.currentTime = 0;
            void event.currentTarget.play?.()?.catch(() => undefined);
          }
        }}
        onError={() => setBroken(true)}
      />
      <button
        type="button"
        className="absolute right-2 bottom-2 flex size-7 items-center justify-center rounded-lg bg-surface/90 text-ink opacity-0 inset-ring-1 inset-ring-hairline outline-none transition-opacity group-hover:opacity-100 focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-accent/60"
        aria-label={playing ? `Pause the clip: ${label}` : `Play the clip: ${label}`}
        onClick={() => {
          if (playing) {
            setWanted(false);
            video.current?.pause?.();
            return;
          }
          plays.current = 0;
          setWanted(true);
          setAsked(true);
          void video.current?.play?.()?.catch(() => undefined);
        }}
        data-testid={`clip-${id}-toggle`}
      >
        {playing ? <IconPause width={14} height={14} aria-hidden="true" /> : <IconPlay width={14} height={14} aria-hidden="true" />}
      </button>
    </div>
  );
}
