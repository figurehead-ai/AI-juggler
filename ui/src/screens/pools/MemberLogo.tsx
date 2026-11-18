/**
 * 0.4.6 (D-126): a pool member's logo on the making screens and the
 * Pools card. An App or a session shows its tool's logo (ToolLogo: the
 * app's own icon, read at run time from this Mac, D-38; a CLI its
 * vendor's app icon with a terminal badge); what has no logo of its own
 * (everyone in the projects, an Account, a Mac) a tile with its icon.
 * A small mark at the corner says a role: the lead's crown in the
 * accent, a reviewer's shield. Decorative: the member's name is always
 * beside it in words.
 */
import type { JSX } from "react";

import type { ControlIcon } from "../../components/controls";
import { IconComputer } from "../../components/icons-actions";
import { IconLead, IconTasks } from "../../components/icons-canvas";
import { IconView } from "../../components/icons-places";
import { IconProfiles, IconSessions, IconShield, IconTerminal } from "../../components/icons";
import { LogoTile, ToolLogo, type ToolLogoSize } from "../../components/ToolLogo";
import type { AppModel } from "../../lib/apps";
import type { PoolRole } from "../../lib/ipc/pools";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";

/** Each role's icon on the making screens (the roles' tiles, the role
 * control, the mark on a logo). */
export const ROLE_ICONS: Record<PoolRole, ControlIcon> = { lead: IconLead, member: IconTasks, reviewer: IconShield, observer: IconView };

/** What a member's logo is drawn from. */
export type LogoSpec = { kind: "tool"; tool: string; look: "desktop" | "cli"; appId?: string | null; name: string } | { kind: "tile"; icon: ControlIcon };

/** Where to look a member's App and session up. */
export interface LogoSources {
  apps: readonly AppModel[];
  sessions: readonly InstanceInfo[];
}

const appLogo = (app: AppModel): LogoSpec =>
  app.kind === "cli" ? { kind: "tool", tool: app.tool ?? "", look: "cli", name: app.app } : { kind: "tool", tool: app.tool ?? "", look: "desktop", appId: app.appId, name: app.app };

/** A member's logo, from its ref (`app:…`, `session:…`, `tool:…`,
 * `account:…`, `mac:…`, `all`). */
export function logoOf(ref: string, { apps, sessions }: LogoSources): LogoSpec {
  const at = ref.indexOf(":");
  const kind = at < 0 ? ref : ref.slice(0, at);
  const id = at < 0 ? "" : ref.slice(at + 1);
  switch (kind) {
    case "app": {
      const app = apps.find((one) => one.id === id);
      if (app) return appLogo(app);
      return { kind: "tile", icon: id.startsWith("cli:") ? IconTerminal : IconComputer };
    }
    case "session": {
      const session = sessions.find((one) => one.id === id);
      if (!session) return { kind: "tile", icon: IconSessions };
      const app = session.app_id ? apps.find((one) => one.id === session.app_id) : undefined;
      if (app) return app.kind === "desktop" ? { kind: "tool", tool: session.provider, look: "desktop", appId: app.appId, name: app.app } : { kind: "tool", tool: session.provider, look: "cli", name: app.app };
      return { kind: "tool", tool: session.provider, look: "cli", name: session.provider };
    }
    case "tool":
      return { kind: "tool", tool: id, look: "desktop", name: id };
    case "account":
      return { kind: "tile", icon: IconProfiles };
    case "mac":
      return { kind: "tile", icon: IconComputer };
    default:
      return { kind: "tile", icon: IconSessions };
  }
}

/** The mark at a logo's corner: the lead's crown, a reviewer's shield. */
function RoleMark({ of: role, size }: { of: PoolRole; size: number }): JSX.Element | null {
  if (role !== "lead" && role !== "reviewer") return null;
  const Icon = ROLE_ICONS[role];
  const side = size >= 32 ? 18 : 16;
  return (
    <span
      aria-hidden="true"
      className={"absolute -bottom-1 -right-1 flex items-center justify-center rounded-full shadow-[0_0_0_2px_var(--color-surface)] " + (role === "lead" ? "bg-accent-fill text-white" : "bg-surface-3 text-ink")}
      style={{ width: side, height: side }}
      data-testid="member-logo-role"
      data-role={role}
    >
      <Icon width={side - 6} height={side - 6} strokeWidth={2.2} />
    </span>
  );
}

export function MemberLogo({ spec, size = 24, badge, on = false }: { spec: LogoSpec; size?: ToolLogoSize; /** Its role mark (a lead's, a reviewer's). */ badge?: PoolRole; on?: boolean }): JSX.Element {
  return (
    <span className="relative inline-flex shrink-0" data-testid="member-logo" data-logo={spec.kind === "tool" ? spec.tool || "app" : "tile"}>
      {spec.kind === "tool" ? <ToolLogo tool={spec.tool} kind={spec.look} appId={spec.appId} name={spec.name} size={size} /> : <LogoTile icon={spec.icon} size={size} on={on} />}
      {badge && <RoleMark of={badge} size={size} />}
    </span>
  );
}
