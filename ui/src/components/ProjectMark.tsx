/**
 * AJ-E1532 — a project's mark, a rounded square: its own icon, else its
 * letters (or the user's emoji) on its colour. Clones of one repository
 * share the mark and carry their number. The backend decides it
 * (`ProjectInfo.mark`); this only draws it, the same on the rail, Home,
 * History, the pages' headers and the inspector (D-25). Decorative:
 * the project's name is always beside it.
 */
import { useEffect, useState, type JSX } from "react";
import { markOf, swatchColor } from "../lib/palette";
import type { ProjectMarkInfo } from "../types/generated/ProjectMarkInfo";
import { projectIconRead } from "../lib/ipc/marks";
import { IconProjects } from "./icons";

/** Icons already read, by project id and version. */
const icons = new Map<string, string | null>();

/** The project's icon as a `data:` URL, once read (`null`: none, or not
 * yet). */
export function useProjectIcon(projectId: string | null, version: string | undefined): string | null {
  const key = projectId && version ? `${projectId}@${version}` : null;
  const [url, setUrl] = useState<string | null>(key ? (icons.get(key) ?? null) : null);
  useEffect(() => {
    if (!key || !projectId) return setUrl(null);
    if (icons.has(key)) return setUrl(icons.get(key) ?? null);
    let alive = true;
    void projectIconRead(projectId).then(
      (read) => {
        icons.set(key, read);
        if (alive) setUrl(read);
      },
      () => icons.set(key, null),
    );
    return () => {
      alive = false;
    };
  }, [key, projectId]);
  return url;
}

export function ProjectMark({ projectId, size = 20 }: { projectId: string | null; size?: number }): JSX.Element {
  const mark = markOf(projectId);
  const icon = useProjectIcon(projectId, mark?.shows === "icon" ? mark.icon : undefined);
  return <MarkTile mark={mark} icon={icon} size={size} />;
}

/** A mark as drawn: `icon` (a `data:` URL) over the letters when there is
 * one; a neutral folder when the mark isn't known (a project the list
 * hasn't loaded). The mark sheet draws its preview with it. */
export function MarkTile({ mark, icon, size }: { mark: ProjectMarkInfo | null; icon: string | null; size: number }): JSX.Element {
  const radius = Math.max(3, Math.round(size * 0.28));
  if (!mark) {
    const glyph = Math.round(size * 0.6);
    return (
      <span aria-hidden="true" className="flex shrink-0 items-center justify-center bg-surface-3 text-muted" style={{ width: size, height: size, borderRadius: radius }} data-testid="mark-project">
        <IconProjects width={glyph} height={glyph} />
      </span>
    );
  }
  const color = swatchColor(mark.swatch);
  return (
    <span
      aria-hidden="true"
      className="relative flex shrink-0 select-none items-center justify-center font-semibold leading-none text-white"
      style={{
        width: size,
        height: size,
        borderRadius: radius,
        background: icon ? "var(--color-surface-3)" : color,
        boxShadow: icon ? `inset 0 0 0 1.5px ${color}` : undefined,
        fontSize: Math.max(8, Math.round(size * (mark.emoji ? 0.62 : 0.46))),
      }}
      data-testid="mark-project"
      data-swatch={mark.swatch}
      data-kind={icon ? "icon" : mark.emoji ? "emoji" : "letters"}
    >
      {icon ? (
        <img src={icon} alt="" draggable={false} className="object-contain" style={{ width: size - 6, height: size - 6 }} />
      ) : (
        (mark.emoji ?? mark.letters)
      )}
      {mark.clone !== undefined && size >= 14 && (
        <span
          className="absolute -bottom-1 -right-1 flex items-center justify-center rounded-full border border-hairline bg-surface-3 font-semibold tabular-nums text-ink"
          style={{ minWidth: Math.round(size * 0.55), height: Math.round(size * 0.55), fontSize: Math.max(7, Math.round(size * 0.36)) }}
          data-testid="mark-clone"
        >
          {mark.clone}
        </span>
      )}
    </span>
  );
}
