/**
 * AJ-E1534: a project's mark, a rounded square in its colour (AJ-E1532
 * gives it a monogram). One component for the page header and the
 * session lists, so a project looks the same wherever it shows.
 */
import type { JSX } from "react";
import { hueOf } from "./indicator";
import { IconProjects } from "./icons";

export function ProjectMark({ projectId, size = 20 }: { projectId: string; size?: number }): JSX.Element {
  const icon = Math.round(size * 0.6);
  return (
    <span
      aria-hidden="true"
      className="flex shrink-0 items-center justify-center rounded-md"
      style={{
        width: size,
        height: size,
        background: `hsl(${hueOf(projectId)} 60% 50% / 0.18)`,
        color: `hsl(${hueOf(projectId)} 60% 66%)`,
      }}
      data-testid="mark-project"
    >
      <IconProjects width={icon} height={icon} />
    </span>
  );
}
