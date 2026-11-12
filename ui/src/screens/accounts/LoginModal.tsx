/**
 * Sign in to one App's folder (moved out of Profiles.tsx, its size
 * budget): the one shared sign-in queue card, opened in a terminal.
 */
import { Modal } from "../../components/form";
import { LoginQueueCard } from "../../components/LoginQueueCard";
import { cliAppName } from "../../lib/app-name";
import { ownSignIn, providerDisplayName } from "../../lib/providers";
import type { AccountInfo } from "../../types/generated/AccountInfo";

export function LoginModal({
  account,
  openTerminal,
  onClose,
}: {
  account: AccountInfo;
  openTerminal: (id: string, title: string) => void;
  onClose: () => void;
}) {
  return (
    <Modal
      title={`Log in · ${cliAppName(account.provider, account.label)}`}
      subtitle={`${ownSignIn(account.provider)}, opened for you — AI Juggler never sees your credentials.`}
      onClose={onClose}
    >
      {/* AJ-E308 Req.3: the ONE shared queue card — no alerts, no
          unhandled rejections; polls and auto-detects. */}
      <LoginQueueCard
        accountId={account.id}
        label={account.label}
        providerName={providerDisplayName(account.provider)}
        initialState={account.login_state}
        onSpawned={(terminalId) => {
          openTerminal(terminalId, `Sign in · ${cliAppName(account.provider, account.label)}`);
          onClose();
        }}
        onDone={onClose}
      />
    </Modal>
  );
}

