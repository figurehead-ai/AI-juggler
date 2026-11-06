/**
 * AJ-E2908: an ended session's page offers Resume (the same conversation,
 * on the same account and launch settings) before Relaunch (a fresh one).
 * When it can't run it says why, in the button's own words: no
 * conversation yet, the account signed out, out of room (with Continue on
 * another account beside it, D-45), or the conversation open in another
 * app now.
 */
import { useEffect, useState, type JSX } from "react";

import { Button } from "../../components/primitives";
import { showToast } from "../../components/Toast";
import { useTargetRoom } from "../../components/switch/TargetRoom";
import { accountsList } from "../../lib/ipc/accounts";
import { errorText, signedIn } from "../../lib/bridge";
import type { AccountInfo } from "../../types/generated/AccountInfo";
import type { InstanceInfo } from "../../types/generated/InstanceInfo";
import { outOfRoomWords } from "../../lib/room";
import { sessionResume } from "../../lib/ipc/launch";

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
  if (!session.conversation_id) return "It has no conversation to resume yet";
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
}: {
  session: InstanceInfo;
  /** The app the conversation is open in now, if any. */
  openIn: string | null;
  onOpen?: (sessionId: string) => void;
  /** Continue on another account… (shown beside it when out of room). */
  onContinue?: () => void;
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
      <Button
        size="sm"
        variant="primary"
        disabled={blocked !== null || busy}
        title={blocked ?? "Resume — same session, same account."}
        aria-describedby={blocked ? `resume-why-${session.id}` : undefined}
        onClick={() => void resume()}
        data-testid="header-resume"
      >
        {busy ? "Resuming…" : "Resume"}
      </Button>
      {blocked && (
        <span id={`resume-why-${session.id}`} className="sr-only" data-testid="header-resume-why">
          {blocked}
        </span>
      )}
      {out && onContinue && (
        <Button size="sm" variant="secondary" onClick={onContinue} data-testid="header-resume-continue">
          Continue on another account…
        </Button>
      )}
    </>
  );
}
