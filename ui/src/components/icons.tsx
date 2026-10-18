import type { SVGProps } from "react";

/**
 * Inline SVG icon set (lucide-style strokes, 24-grid). No icon dependency:
 * chrome icons only, kept tiny and tree-shakeable by construction.
 */

function base(props: SVGProps<SVGSVGElement>) {
  return {
    width: 18,
    height: 18,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.75,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    ...props,
  };
}

export function IconDashboard(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="3" width="7" height="9" rx="1.5" />
      <rect x="14" y="3" width="7" height="5" rx="1.5" />
      <rect x="14" y="12" width="7" height="9" rx="1.5" />
      <rect x="3" y="16" width="7" height="5" rx="1.5" />
    </svg>
  );
}

export function IconProjects(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
      <path d="M3 13h18" />
    </svg>
  );
}

export function IconSessions(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 5h16" />
      <path d="M4 12h10" />
      <path d="M4 19h16" />
      <circle cx="18" cy="12" r="2" />
    </svg>
  );
}

export function IconProfiles(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="9" cy="8" r="3.25" />
      <path d="M3.5 19c.6-2.8 2.8-4.5 5.5-4.5s4.9 1.7 5.5 4.5" />
      <path d="M17 8.5a2.75 2.75 0 1 1 0 5.5" />
      <path d="M18 14.5c2 .4 3.4 1.9 3.8 4" />
    </svg>
  );
}

export function IconUsage(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 20V10" />
      <path d="M10 20V4" />
      <path d="M16 20v-7" />
      <path d="M21 20H3" />
    </svg>
  );
}

export function IconFleet(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="8" rx="1.5" />
      <rect x="3" y="14" width="18" height="6" rx="1.5" />
      <path d="M7 8h.01M7 17h.01" />
    </svg>
  );
}

export function IconTerminal(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m5 7 4 4-4 4" />
      <path d="M12 17h7" />
    </svg>
  );
}

export function IconJuggler(props: SVGProps<SVGSVGElement>) {
  // Mark: three balls mid-juggle over a dashed throw arc. The balls are
  // the three harnesses in flight (violet, bright violet, live cyan).
  return (
    <svg {...base(props)}>
      <path
        d="M6.8 16C8.4 7.4 15.6 7.4 17.2 16"
        stroke="currentColor"
        strokeOpacity={0.35}
        strokeWidth={1.25}
        strokeDasharray="2.2 2.6"
        fill="none"
      />
      <circle cx={6.8} cy={16} r={2.3} fill="var(--color-accent)" />
      <circle cx={12} cy={8.6} r={2.5} fill="var(--color-accent-hi)" />
      <circle cx={17.2} cy={16} r={2.3} fill="var(--color-live)" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// Provider marks (AJ-E402 I-01): filled brand-neutral glyphs per the
// pass2/04 §3 table — ◆ Claude, ⬢ Codex, ▲ ZCode, ◇ generic Tool. These
// are MARKS, not status dots: never reused for state.

/** I-01: Claude Code — solid diamond. */
export function IconProviderClaude(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)} strokeWidth={1.25}>
      <path d="M12 3.5 20.5 12 12 20.5 3.5 12Z" fill="currentColor" />
    </svg>
  );
}

/** I-01: Codex — solid hexagon. */
export function IconProviderCodex(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)} strokeWidth={1.25}>
      <path d="M12 3 20 7.75v8.5L12 21l-8-4.75v-8.5Z" fill="currentColor" />
    </svg>
  );
}

/** I-01: ZCode — solid triangle. */
export function IconProviderZCode(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)} strokeWidth={1.25}>
      <path d="M12 4 20.5 19.5h-17Z" fill="currentColor" />
    </svg>
  );
}

/** I-01: unknown Provider — hollow diamond + text fallback (`Tool`). */
export function IconProviderGeneric(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)} strokeWidth={1.75}>
      <path d="M12 3.5 20.5 12 12 20.5 3.5 12Z" />
    </svg>
  );
}

// ---------------------------------------------------------------------------
// State glyphs (AJ-E402 I-04): every state renders icon + text + colour
// (WCAG 1.4.1 — never colour alone). Stroke style, chrome-size.

/** I-04: starting — half-filled clock. */
export function IconStateStarting(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
    </svg>
  );
}

/** I-04: running — play. */
export function IconStateRunning(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M8 5.5v13l10-6.5Z" />
    </svg>
  );
}

/** I-04: waiting for the user — person with raised hand dot. */
export function IconStateWaiting(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="8" r="3.25" />
      <path d="M6 20c.7-3.3 3.1-5.2 6-5.2s5.3 1.9 6 5.2" />
    </svg>
  );
}

/** I-04: stuck (heuristic) — snail-paced progress: dashed circle arc. */
export function IconStateStuck(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M12 3.5a8.5 8.5 0 1 1-8.3 10.2" strokeDasharray="3.4 2.6" />
      <path d="M12 8v4.5l3 1.5" />
    </svg>
  );
}

/** I-04: quota blocked — circle with slash. */
export function IconStateQuota(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M6 18 18 6" />
    </svg>
  );
}

/** I-04: finished — check. */
export function IconStateFinished(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m5 12.5 4.5 4.5L19 7.5" />
    </svg>
  );
}

/** I-04: stopped — square. */
export function IconStateStopped(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="6" y="6" width="12" height="12" rx="1.5" />
    </svg>
  );
}

/** I-04: failed — cross. */
export function IconStateFailed(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m6.5 6.5 11 11M17.5 6.5l-11 11" />
    </svg>
  );
}

/** I-04: handed over — swap arrows. */
export function IconStateHandover(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 9h13l-3-3M20 15H7l3 3" />
    </svg>
  );
}

/** I-04: unknown wire value — question mark. */
export function IconStateUnknown(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M9 9a3 3 0 1 1 4.4 2.65c-.9.5-1.4 1.05-1.4 2.1" />
      <path d="M12 17.5h.01" />
    </svg>
  );
}

/** I-09: git branch. */
export function IconGitBranch(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="7" cy="6" r="2.2" />
      <circle cx="7" cy="18" r="2.2" />
      <circle cx="17" cy="9" r="2.2" />
      <path d="M7 8.2v7.6M9.2 6h4.3a1.5 1.5 0 0 1 1.5 1.5v.3M17 11.2c0 2.5-2 3.3-4.2 3.3H9.2" />
    </svg>
  );
}

/** I-08: subprocess children (fork). */
export function IconFork(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M7 4v4c0 2 1.5 3.5 3.5 3.5H14c2 0 3.5 1.5 3.5 3.5v4" />
      <path d="M4 4h6M14.5 19h6" />
    </svg>
  );
}

/** I-08: subtasks (stacked layers). */
export function IconSubtasks(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="m12 4 8 4-8 4-8-4Z" />
      <path d="m4 13 8 4 8-4" />
    </svg>
  );
}

/** I-10: needs attention — alert. */
export function IconAlert(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M12 4.5 21 19H3Z" />
      <path d="M12 10v4M12 16.8h.01" />
    </svg>
  );
}
