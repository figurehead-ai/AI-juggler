/**
 * AJ-E3508 — each area's floor logo, drawn here as the window draws marks
 * everywhere else, and kept by the backend (the "Floor logos" cache):
 *
 * - the tool's mark as `ToolMark` draws it, or a desktop App's own icon
 *   (read at run time, never bundled);
 * - the initials of the App's Account on a pill below (no Account: the
 *   App's own initials);
 * - in a ring of the App's colour.
 *
 * A logo's print says what it was drawn from; the snapshot says each
 * App's kept print, so only what's missing or changed is drawn, once.
 * Nothing runs until the Floor has been opened (its snapshot arrives).
 */
import { useEffect } from "react";

import { drawableMark } from "../components/ToolMark";
import { appIconRead } from "./ipc/appIcons";
import { floorLogoPut, onFloorChanged, type FloorApp } from "./ipc/floor";
import { markInk, slotOklch } from "./marks";
import { toolSpec } from "./tools";

/** A logo's side, pixels. */
export const LOGO_SIDE = 256;

/** FNV-1a (32-bit) of `words`, as 8 hex digits: the backend's `hash`. */
export function fnv1a(words: string): string {
  let h = 0x811c9dc5;
  for (const byte of new TextEncoder().encode(words)) {
    h ^= byte;
    h = Math.imul(h, 0x01000193) >>> 0;
  }
  return h.toString(16).padStart(8, "0");
}

/** The App's own initials ("Codex · Home" → "CH"), for an App with no
 * known Account. */
export function ownInitials(name: string): string {
  return name
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]!.toUpperCase())
    .join("");
}

/** What a logo is drawn from, in words. */
export interface LogoParts {
  /** The mark: an icon's app id, an allowed mark's id, or letters. */
  mark: { kind: "icon"; appId: string } | { kind: "path"; id: string; path: string } | { kind: "letters"; text: string };
  initials: string;
  slot: number;
}

/** The parts of `app`'s logo. */
export function logoParts(app: Pick<FloorApp, "tool" | "kind" | "host_app" | "initials" | "name" | "colour_slot">): LogoParts {
  const drawable = drawableMark(app.tool);
  const mark: LogoParts["mark"] =
    app.kind === "desktop" && app.host_app
      ? { kind: "icon", appId: app.host_app }
      : drawable
        ? { kind: "path", id: drawable.id, path: drawable.path }
        : { kind: "letters", text: ownInitials(toolSpec(app.tool).display_name || app.tool) };
  return { mark, initials: app.initials || ownInitials(app.name), slot: app.colour_slot };
}

/** The print of `parts`: a new one when the mark, the initials or the
 * colour change. */
export function logoPrint(parts: LogoParts): string {
  const mark = parts.mark.kind === "icon" ? `icon:${parts.mark.appId}` : parts.mark.kind === "path" ? `path:${parts.mark.id}` : `letters:${parts.mark.text}`;
  return fnv1a(`${mark}|${parts.initials}|${parts.slot}`);
}

function colour(slot: number, alpha = 1): string {
  const { l, c, h } = slotOklch(slot);
  return `oklch(${l} ${c} ${h} / ${alpha})`;
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("the icon didn't load"));
    img.src = url;
  });
}

/** Draws `parts` as a 256 px PNG. */
export async function drawLogo(parts: LogoParts): Promise<Uint8Array> {
  const canvas = document.createElement("canvas");
  canvas.width = LOGO_SIDE;
  canvas.height = LOGO_SIDE;
  const g = canvas.getContext("2d");
  if (!g) throw new Error("no canvas");
  const c = LOGO_SIDE / 2;
  const disc = (r: number, fill: string) => {
    g.beginPath();
    g.arc(c, c, r, 0, Math.PI * 2);
    g.fillStyle = fill;
    g.fill();
  };
  disc(124, colour(parts.slot));
  disc(110, "rgba(255,255,255,0.88)");
  disc(100, colour(parts.slot));
  const ink = markInk(parts.slot);
  const markBox = { x: c - 46, y: 44, size: 92 };
  if (parts.mark.kind === "icon") {
    const url = await appIconRead({ appId: parts.mark.appId, appPid: null }, 128).catch(() => null);
    if (url) g.drawImage(await loadImage(url), markBox.x, markBox.y, markBox.size, markBox.size);
  } else if (parts.mark.kind === "path") {
    g.save();
    g.translate(markBox.x, markBox.y);
    g.scale(markBox.size / 24, markBox.size / 24);
    g.fillStyle = ink;
    g.fill(new Path2D(parts.mark.path));
    g.restore();
  } else {
    g.fillStyle = ink;
    g.font = "700 64px -apple-system, system-ui, sans-serif";
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(parts.mark.text, c, markBox.y + markBox.size / 2);
  }
  if (parts.initials) {
    g.font = "700 34px -apple-system, system-ui, sans-serif";
    const w = Math.max(56, g.measureText(parts.initials).width + 28);
    g.fillStyle = "#ffffff";
    g.beginPath();
    g.roundRect(c - w / 2, 150, w, 46, 23);
    g.fill();
    g.fillStyle = colour(parts.slot);
    g.textAlign = "center";
    g.textBaseline = "middle";
    g.fillText(parts.initials, c, 174);
  }
  const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/png"));
  if (!blob) throw new Error("the logo couldn't be drawn");
  return new Uint8Array(await blob.arrayBuffer());
}

/** The Apps whose kept logo is missing or out of date. */
export function logosToDraw(apps: readonly FloorApp[]): Array<{ app: FloorApp; parts: LogoParts; print: string }> {
  return apps.flatMap((app) => {
    const parts = logoParts(app);
    const print = logoPrint(parts);
    return app.logo === print ? [] : [{ app, parts, print }];
  });
}

/** Draws the logos a snapshot lacks, one at a time, with `put` (draw and
 * keep one). Only a failed print is not tried again: one the cache lost
 * (cleared, trimmed) is missing from the next snapshot and drawn again. */
export function logoKeeper(
  put: (appId: string, print: string, parts: LogoParts) => Promise<void>,
): (apps: readonly FloorApp[]) => Promise<void> {
  let busy = false;
  const failed = new Set<string>();
  return async (apps) => {
    if (busy) return;
    const todo = logosToDraw(apps).filter(({ app, print }) => !failed.has(`${app.id}|${print}`));
    if (todo.length === 0) return;
    busy = true;
    try {
      for (const { app, parts, print } of todo) {
        try {
          await put(app.id, print, parts);
        } catch {
          // The Floor writes the letters itself meanwhile.
          failed.add(`${app.id}|${print}`);
        }
      }
    } finally {
      busy = false;
    }
  };
}

/** Keeps the Floor's logos drawn: each new snapshot, the missing ones, one
 * at a time (nothing happens until the Floor has been opened). */
export function useFloorLogos(): void {
  useEffect(() => {
    const keep = logoKeeper(async (appId, print, parts) => floorLogoPut(appId, print, await drawLogo(parts)));
    return onFloorChanged((snapshot) => {
      void keep(snapshot.apps);
    });
  }, []);
}
