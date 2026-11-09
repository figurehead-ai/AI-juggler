/**
 * AJ-E2908: an ended session's page offers Resume (the same conversation,
 * on the same account and launch settings) before Relaunch (a fresh one).
 * When it can't run it says why, in the button's own words: no
 * conversation yet, the account signed out, out of room (with Continue on
 * another account beside it, D-45), the conversation open in another
 * app now, or (rc21) its tool won't open it by its id, with the way out
 * (Open the original) beside it. Greyed, it says why on hover and on
 * keyboard focus (`WhyButton`): the rc21 walk found it greyed in silence.
 */
import { useEffect, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { OpenOriginal } from "../../components/OpenOriginal";
import { WhyButton } from "../../components/WhyTip";
import { resumeRefusalWords } from "../../lib/resume-refusal";
import { Hinted } from "../Hinted";
import { showToast } from "../../components/Toast";
import { useTargetRoom } from "../../components/switch/TargetRoom";
import { accountsList } from "../../lib/ipc/accounts";
import { errorText, signedIn } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { outOfRoomWords } from "../../lib/room";
import { sessionResume } from "../../lib/ipc/launch";
import { runtimeHandover } from "../../lib/ipc/runtime";
import { toolSpec } from "../../lib/tools";

/** Why Resume can't run now, or null. */
export function resumeBlocked({
  session,
  account,
  out,
  openIn,
}: {
  session: InstanceInfo;
  account: AccountInfo | null;
  out: string | null;
  openIn: string | null;
}): string | null {
  // AJ-E2908 R2: whether its tool can resume is the tool's own spec.
  const tool = toolSpec(session.provider);
  if (!tool.supports.resume) return `${tool.display_name} can't resume a conversation from AI Juggler`;
  if (!session.conversation_id) return "It has no conversation to resume yet";
  // rc21: its tool's words, from the backend's one check every door asks.
  const refused = resumeRefusalWords(session.resume_refusal);
  if (refused) return refused;
  if (account && !signedIn(account.login_state)) return `Sign in to ${account.label} first`;
  if (out) return out;
  if (openIn) return `It's open in ${openIn} now. Go to it there`;
  return null;
}

export function ResumeButton({
  session,
  openIn,
  onOpen,
  onContinue,
  onOpenOriginal,
  hint = null,
}: {
  session: InstanceInfo;
  /** The app the conversation is open in now, if any. */
  openIn: string | null;
  onOpen?: (sessionId: string) => void;
  /** Continue on another account… (shown beside it when out of room). */
  onContinue?: () => void;
  /** Opens a conversation by its account and id (Open the original). */
  onOpenOriginal?: (accountId: string, id: string, projectId: string | null) => void;
  /** The line under Resume alone ("Same conversation"): Run in AI
   * Juggler and Continue on another account stay beside it on the
   * header's row (CI #218: in the hint's column they stacked under it). */
  hint?: string | null;
}): JSX.Element {
  const [account, setAccount] = useState<AccountInfo | null>(null);
  const [busy, setBusy] = useState(false);
  useEffect(() => {
    let live = true;
    accountsList().then(
      (all) => live && setAccount(all.find((a) => a.id === session.account_id) ?? null),
      () => undefined,
    );
    return () => {
      live = false;
    };
  }, [session.account_id]);
  const room = useTargetRoom(session.account_id ?? undefined);
  const out = room?.kind === "out" ? `${room.account ?? account?.label ?? "This account"} is ${outOfRoomWords(room.resetsAtMs).replace(/^Out/, "out")}` : null;
  const blocked = resumeBlocked({ session, account, out, openIn });
  // The RC walk (runtime lab step 4): "Open in terminal" had no way back.
  // Once its terminal has ended, a tool with a run-without-a-terminal
  // runtime continues the same conversation here (one writer at a time).
  const back = toolSpec(session.provider).runtime ?? null;
  const runHere = async () => {
    setBusy(true);
    try {
      const started = await runtimeHandover(session.id, "driven");
      onOpen?.(started.session_id);
    } catch (cause) {
      showToast({ tone: "error", message: `Couldn't run it in AI Juggler: ${errorText(cause)}` });
    } finally {
      setBusy(false);
    }
  };
  const resume = async () => {
    setBusy(true);
    try {
      const started = await sessionResume(session.id);
      onOpen?.(started.session_id);
    } catch (cause) {
      showToast({ tone: "error", message: `Couldn't resume it: ${errorText(cause)}` });
    } finally {
      setBusy(false);
    }
  };
  return (
    <>
      <Hinted hint={hint} testId="header-resume-hint">
        <WhyButton
          size="sm"
          variant="primary"
          why={blocked}
          disabled={busy}
          title={blocked ? undefined : "Resume — same session, same account."}
          onClick={() => void resume()}
          data-testid="header-resume"
        >
          {busy ? "Resuming…" : "Resume"}
        </WhyButton>
      </Hinted>
      <OpenOriginal refusal={session.resume_refusal} onOpen={onOpenOriginal} />
      {back && session.conversation_id && blocked === null && (
        <Button size="sm" variant="secondary" disabled={busy} title={`The same conversation, here without a terminal: ${back.label}${back.beta ? " (Beta)" : ""}.`} onClick={() => void runHere()} data-testid="header-run-here">
          Run in AI Juggler
        </Button>
      )}
      {out && onContinue && (
        <Button size="sm" variant="secondary" onClick={onContinue} data-testid="header-resume-continue">
          Continue on another account…
        </Button>
      )}
    </>
  );
}
