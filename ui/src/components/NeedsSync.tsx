/**
 * AJ-E2502 — keeps the notification model in step with Needs you: the
 * same list Home shows (lib/needs-you.ts), sent whenever it changes, so
 * the bell, the app badge and the menu-bar item carry Home's count. Only
 * once the first loads have answered (an empty list before then would
 * end every need, and the next would tell them all again). Draws
 * nothing. Also answers the menu-bar item: a notice to open, the centre,
 * New session….
 */
import { useEffect, useRef } from "react";
import { macNeed } from "../screens/Home";
import { needInputsOf, openNowOf, useNeedsYou } from "../lib/needs-you";
import { notificationsPage, notificationsSync, onNewSessionAsked, onNoticeOpen } from "../lib/ipc/notices";
import { applySummary, setCentreOpen } from "../lib/notices-store";
import { useConversations } from "../lib/conversations";
import { readOnlyBackend } from "../lib/ipc/transport";
import type { AccountInfo, InstanceInfo, ProjectInfo } from "../lib/bridge";
import type { ThisMacInfo } from "../lib/machine";
import type { NeedInput, OpenNowInput } from "../lib/ipc/notices";
import { useNoticeActions } from "../lib/notice-actions";

/** How long a change settles before it is sent. */
const SETTLE_MS = 400;

export function NeedsSync({
  ready,
  sessions,
  accounts,
  projects,
  thisMac,
  now,
}: {
  /** The first load has answered. */
  ready: boolean;
  sessions: InstanceInfo[];
  accounts: AccountInfo[];
  projects: ProjectInfo[];
  thisMac: ThisMacInfo | null;
  now: number;
}): null {
  const needs = useNeedsYou({ sessions, accounts, projects, thisMac, now, macNeed });
  const { loaded } = useConversations();
  const actions = useNoticeActions(sessions);
  const sent = useRef("");
  const body = JSON.stringify([needInputsOf(needs), openNowOf(needs.items)]);
  useEffect(() => {
    if (!ready || !loaded || readOnlyBackend || body === sent.current) return;
    const timer = window.setTimeout(() => {
      sent.current = body;
      const [inputs, open] = JSON.parse(body) as [NeedInput[], OpenNowInput];
      void notificationsSync(inputs, open).then(applySummary, () => {
        // Sent again with the next change.
        sent.current = "";
      });
    }, SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [ready, loaded, body]);
  // The menu-bar item: a notice (its subject), the centre, or New session….
  const opener = useRef(actions.primary);
  opener.current = actions.primary;
  useEffect(() => {
    const stopOpen = onNoticeOpen((id) => {
      if (id === null) return setCentreOpen(true);
      void notificationsPage("needs", { cursor: null, limit: 200 }).then((page) => {
        const notice = page.items.find((row) => row.id === id);
        if (notice) opener.current(notice);
        else setCentreOpen(true);
      }, () => setCentreOpen(true));
    });
    const stopNew = onNewSessionAsked(() => window.dispatchEvent(new Event("aij:new-session")));
    return () => {
      stopOpen();
      stopNew();
    };
  }, []);
  return null;
}
