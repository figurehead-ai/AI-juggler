/**
 * D-62 — the two entity pages that aren't the inspector's: an App's
 * (AJ-E3204) and an Account's (AJ-E3205), lazily loaded each.
 */
import type { JSX } from "react";
import { AppPage, SubscriptionPage } from "../lazyParts";
import type { AccountInfo } from "../types/generated/AccountInfo";
import type { InstanceInfo } from "../types/generated/InstanceInfo";
import type { ProjectInfo } from "../types/generated/ProjectInfo";

export function EntityPage({ kind, ...props }: {
  kind: "app" | "subscription";
  id: string;
  sessions: InstanceInfo[];
  projects: ProjectInfo[];
  accounts: AccountInfo[];
  now: number;
  onOpenSession: (id: string) => void;
  onOpenProject: (id: string) => void;
  /** An App's folder changed (its ⋯). */
  onChanged?: () => void;
}): JSX.Element {
  return (
    <div className="min-h-0 flex-1" data-testid={kind === "app" ? "app-scroll" : "account-scroll"}>
      {kind === "app" ? <AppPage {...props} /> : <SubscriptionPage {...props} />}
    </div>
  );
}
