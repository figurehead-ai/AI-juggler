/**
 * AJ-E1520 (0.3) — a project's subagents: the Subagents view over its
 * newest sessions together (each group says which session it is in),
 * and a subagent's own page from it. For the project page to mount:
 * `<ProjectSubagents projectId={…} />` (lazily, like the session tabs).
 */
import { useCallback, useMemo, useRef, useState, type JSX } from "react";
import type { SubagentRowInfo } from "../../lib/ipc/subagents";
import { SubagentDetail, type DetailTarget } from "./SubagentDetail";
import { SubagentsView } from "./SubagentsView";
import type { SubagentsSource } from "./useSubagentRows";

export function ProjectSubagents({ projectId, visible = true, now = Date.now() }: { projectId: string; visible?: boolean; now?: number }): JSX.Element {
  const [open, setOpen] = useState<(DetailTarget & { title: string }) | null>(null);
  const keep = useRef(0);
  const source = useMemo<SubagentsSource>(() => ({ kind: "project", projectId }), [projectId]);
  const onOpen = useCallback((row: SubagentRowInfo) => {
    const agent = row.agent;
    if (!agent?.session) return;
    setOpen({ profileId: agent.session.account_id, conversationId: agent.session.conversation_id, subagentId: agent.id, title: agent.session.title ?? "Its session" });
  }, []);
  if (open) {
    return (
      <div className="w-full min-w-0 max-w-[1280px]" data-testid="project-subagents">
        <SubagentDetail
          target={open}
          sessionTitle={open.title}
          visible={visible}
          onBack={() => setOpen(null)}
          onOpen={(id) => setOpen({ ...open, subagentId: id })}
          onConversation={null}
        />
      </div>
    );
  }
  return (
    <div className="w-full min-w-0 max-w-[1280px]" data-testid="project-subagents">
      <SubagentsView source={source} visible={visible} now={now} onOpen={onOpen} firstView="all" whose="these sessions'" keep={keep} />
    </div>
  );
}
