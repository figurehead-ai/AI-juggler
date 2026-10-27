/**
 * AJ-E2507 — your menu, behind the avatar (the approved design of
 * 2026-10-27; no spec had one). What isn't daily work: who you are (your
 * name as this computer keeps it, or your optional Figurehead account —
 * D-18, D-46: one quiet "Connect", never a reminder), Settings (⌘, as
 * before), the screens that left the header's More (This Mac, Processes;
 * My Macs only with a Figurehead account — the maintainer's ruling,
 * 2026-10-27, D-49), Privacy, Appearance inline, keyboard shortcuts,
 * help, Report a problem…, and About and updates with the version and
 * channel. The menu rule (AJ-E2106): icons on every line, three groups,
 * getting there first, the one destructive line last.
 */
import { useEffect, useState, type JSX, type MouseEvent, type SVGProps } from "react";
import { MenuItem, MenuSeparator, RowMenu } from "./RowMenu";
import { IconShield } from "./icons";
import { IconBroom, IconComputer } from "./icons-actions";
import { IconAppearance, IconCloud, IconConnect, IconGear, IconHelp, IconKeyboard, IconLeave, IconUpdate, IconAsk } from "./icons-menu";
import { channelLabel } from "./UpdatesCard";
import { cloudBackupStatus, fetchAppInfo, figureheadStatus, fleetPeers, updateStatus, type FigureheadStatusInfo } from "../lib/bridge";
import { appHelpOpen, appReportBugOpen, appUserGet } from "../lib/ipc/app";
import { agoWords } from "../lib/time-words";
import { GLOBAL_NAV } from "../lib/nav";
import { setTheme, storedTheme, THEME_EVENT, type ThemeChoice } from "../theme";
import type { AppUserInfo } from "../types/generated/AppUserInfo";

/** "f…@example.com": an address, shortened as elsewhere. */
export function shortEmail(email: string): string {
  const at = email.indexOf("@");
  return at > 0 ? `${email[0]}…${email.slice(at)}` : email;
}

function screenLabel(id: string): string {
  return GLOBAL_NAV.find((entry) => entry.id === id)?.label ?? id;
}

function go(id: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-screen", { detail: { id } }));
}

function settings(section?: string): void {
  window.dispatchEvent(new CustomEvent("aij:open-settings", { detail: section ? { section } : undefined }));
}

function Line({ icon: Icon, label, hint, danger }: { icon: (props: SVGProps<SVGSVGElement>) => JSX.Element; label: string; hint?: string; danger?: boolean }): JSX.Element {
  return (
    <span className="flex items-center gap-2.5">
      <Icon width={15} height={15} aria-hidden="true" className={"shrink-0 " + (danger ? "text-danger" : "text-muted")} />
      <span data-label className="min-w-0 flex-1 truncate text-body">
        {label}
      </span>
      {hint && <span className="shrink-0 text-label text-muted">{hint}</span>}
    </span>
  );
}

function Avatar({ initial, size }: { initial: string; size: number }): JSX.Element {
  return (
    <span aria-hidden="true" style={{ width: size, height: size }} className="flex shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-accent to-live text-meta font-semibold text-canvas">
      {initial}
    </span>
  );
}

const THEMES: { id: ThemeChoice; label: string }[] = [
  { id: "light", label: "Light" },
  { id: "dark", label: "Dark" },
  { id: "system", label: "Auto" },
];

/** What the menu shows, read when it opens (never a network call). */
interface Facts {
  account: FigureheadStatusInfo | null;
  backupAt: number | null;
  computers: number;
  version: string;
  channel: string | null;
}

export function UserMenu(): JSX.Element {
  const [user, setUser] = useState<AppUserInfo | null>(null);
  const [at, setAt] = useState<{ x: number; y: number } | null>(null);
  const [facts, setFacts] = useState<Facts>({ account: null, backupAt: null, computers: 0, version: "", channel: null });
  const [theme, setThemeChoice] = useState<ThemeChoice>(() => storedTheme());
  useEffect(() => {
    void appUserGet().then(setUser, () => setUser(null));
    const follow = () => setThemeChoice(storedTheme());
    window.addEventListener(THEME_EVENT, follow);
    return () => window.removeEventListener(THEME_EVENT, follow);
  }, []);
  const open = (event: MouseEvent<HTMLButtonElement>) => {
    if (at) return setAt(null);
    // Under the avatar, its right edge with the avatar's (the window keeps it in).
    const box = event.currentTarget.getBoundingClientRect();
    setAt({ x: box.right - 320, y: box.bottom + 8 });
    void Promise.allSettled([figureheadStatus(), cloudBackupStatus(), fleetPeers(), fetchAppInfo(), updateStatus()]).then(([account, backup, peers, info, update]) => {
      const value = <T,>(result: PromiseSettledResult<T>): T | null => (result.status === "fulfilled" ? result.value : null);
      const status = value(account);
      setFacts({
        account: status?.state === "linked" ? status : null,
        backupAt: value(backup)?.last?.at_ms ?? null,
        computers: value(peers)?.length ?? 0,
        version: value(info)?.version ?? "",
        channel: value(update)?.channel ?? null,
      });
    });
  };
  const run = (action: () => void) => () => {
    setAt(null);
    action();
  };
  const initial = user?.initial ?? "?";
  const name = user?.name ?? "You";
  const computer = user?.computer ?? "computer";
  const computers = user?.computers ?? "computers";
  const linked = facts.account;
  const myComputers = (
    <MenuItem onClick={run(() => go("fleet"))} testId="user-menu-computers">
      <Line icon={IconComputer} label={`My ${computers}`} hint={facts.computers > 0 ? String(facts.computers) : undefined} />
    </MenuItem>
  );
  return (
    <>
      <button type="button" onClick={open} aria-haspopup="menu" aria-expanded={at !== null} aria-label={`Your menu: ${name}`} title={name} className="rounded-full outline-none focus-visible:ring-2 focus-visible:ring-accent/70" data-testid="user-menu-button">
        <Avatar initial={initial} size={30} />
      </button>
      {at && (
        <RowMenu heading="" ariaLabel="Your menu" at={at} onClose={() => setAt(null)} wide>
          <div role="none" className="flex items-center gap-2.5 px-2 pb-2 pt-1.5" data-testid="user-menu-who">
            <Avatar initial={initial} size={38} />
            <span className="min-w-0 flex-1">
              <span className="block truncate text-body font-semibold text-ink">{name}</span>
              <span className="block truncate text-label text-muted">{linked?.email ? `${shortEmail(linked.email)} · Figurehead` : `On this ${computer} · no Figurehead account`}</span>
            </span>
            {linked?.plan && <span className="shrink-0 rounded-full border border-hairline px-2 py-0.5 text-label text-muted">{linked.plan}</span>}
          </div>
          {linked ? (
            <>
              <MenuItem onClick={run(() => settings("account"))} testId="user-menu-backup">
                <Line icon={IconCloud} label="Cloud backup" hint={facts.backupAt ? agoWords(facts.backupAt, Date.now()) : undefined} />
              </MenuItem>
              {myComputers}
            </>
          ) : (
            <MenuItem onClick={run(() => settings("account"))} testId="user-menu-connect">
              <span className="flex items-center justify-center gap-2 rounded-lg border border-hairline bg-surface-2 px-2 py-1.5 text-meta text-ink">
                <IconConnect width={14} height={14} aria-hidden="true" className="shrink-0 text-muted" />
                <span data-label>Connect a Figurehead account (optional)</span>
              </span>
            </MenuItem>
          )}
          <MenuSeparator />
          <MenuItem onClick={run(() => settings())} testId="user-menu-settings">
            <Line icon={IconGear} label="Settings" hint="⌘," />
          </MenuItem>
          <MenuItem onClick={run(() => go("thismac"))}>
            <Line icon={IconComputer} label={screenLabel("thismac")} />
          </MenuItem>
          <MenuItem onClick={run(() => go("cleanup"))}>
            <Line icon={IconBroom} label={screenLabel("cleanup")} />
          </MenuItem>
          {/* D-49: My Macs only with a Figurehead account (the account group). */}
          <MenuItem onClick={run(() => settings("privacy"))}>
            <Line icon={IconShield} label="Privacy" />
          </MenuItem>
          <div role="group" aria-label="Appearance" className="flex items-center gap-2.5 px-2 py-1" data-testid="user-menu-appearance">
            <IconAppearance width={15} height={15} aria-hidden="true" className="shrink-0 text-muted" />
            <span className="flex-1 text-body text-ink">Appearance</span>
            <span className="flex overflow-hidden rounded-lg border border-hairline">
              {THEMES.map((choice) => (
                <button
                  key={choice.id}
                  type="button"
                  role="menuitemradio"
                  aria-checked={theme === choice.id}
                  onClick={() => {
                    setTheme(choice.id);
                    setThemeChoice(choice.id);
                  }}
                  className={"px-2 py-0.5 text-label outline-none focus-visible:ring-1 focus-visible:ring-accent/60 " + (theme === choice.id ? "bg-surface-3 font-medium text-ink" : "text-muted hover:text-ink")}
                >
                  {choice.label}
                </button>
              ))}
            </span>
          </div>
          <MenuItem onClick={run(() => window.dispatchEvent(new Event("aij:open-shortcuts")))}>
            <Line icon={IconKeyboard} label="Keyboard shortcuts" hint="?" />
          </MenuItem>
          <MenuSeparator />
          <MenuItem onClick={run(() => void appHelpOpen())}>
            <Line icon={IconHelp} label="Help and guides" />
          </MenuItem>
          <MenuItem onClick={run(() => void appReportBugOpen())}>
            <Line icon={IconAsk} label="Report a problem…" />
          </MenuItem>
          <MenuItem onClick={run(() => settings("updates"))} testId="user-menu-about">
            <Line icon={IconUpdate} label="About and updates" hint={[facts.version, facts.channel ? channelLabel(facts.channel) : null].filter(Boolean).join(" · ") || undefined} />
          </MenuItem>
          {linked && (
            <MenuItem danger onClick={run(() => settings("account"))} testId="user-menu-disconnect">
              <Line icon={IconLeave} label="Disconnect Figurehead…" danger />
            </MenuItem>
          )}
        </RowMenu>
      )}
    </>
  );
}

