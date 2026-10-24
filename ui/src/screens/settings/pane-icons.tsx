/**
 * AJ-E2110 — each Settings pane's icon (white on its tile), and the
 * icon for each kind of desktop app. Drawn like the rest (icons.tsx).
 */
import type { JSX, SVGProps } from "react";
import { base, IconProfiles, IconSessions, IconShield, IconTerminal } from "../../components/icons";
import { IconBell, IconComputer, IconEditor, IconKey, IconSliders } from "../../components/icons-actions";
import type { DesktopAppKind } from "../../lib/desktop-apps";
import type { PaneId } from "./panes";

function IconGear(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <circle cx="12" cy="12" r="3" />
      <path d="M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1" />
    </svg>
  );
}

/** A desktop app's window. */
export function IconAppWindow(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <rect x="3" y="4" width="18" height="16" rx="2.5" />
      <path d="M3 9h18" />
      <path d="M6.5 6.5h.01M9 6.5h.01" />
    </svg>
  );
}

function IconSwitching(props: SVGProps<SVGSVGElement>) {
  return (
    <svg {...base(props)}>
      <path d="M4 8h13l-3.5-3.5" />
      <path d="M20 16H7l3.5 3.5" />
    </svg>
  );
}

const PANE_ICONS: Record<PaneId, (props: SVGProps<SVGSVGElement>) => JSX.Element> = {
  general: IconGear,
  "desktop-apps": IconAppWindow,
  accounts: IconProfiles,
  sessions: IconSessions,
  switching: IconSwitching,
  "agent-access": IconKey,
  notifications: IconBell,
  privacy: IconShield,
  macs: IconComputer,
  advanced: IconSliders,
};

export function PaneIcon({ pane }: { pane: PaneId }): JSX.Element {
  const Icon = PANE_ICONS[pane];
  return <Icon width={14} height={14} strokeWidth={2} />;
}

const KIND_ICONS: Record<DesktopAppKind, (props: SVGProps<SVGSVGElement>) => JSX.Element> = {
  desktop: IconAppWindow,
  terminal: IconTerminal,
  editor: IconEditor,
};

/** A desktop app's icon, by its kind (the app's own icon arrives with
 * the place headers). */
export function AppKindIcon({ kind }: { kind: DesktopAppKind }): JSX.Element {
  const Icon = KIND_ICONS[kind];
  return <Icon width={18} height={18} />;
}
