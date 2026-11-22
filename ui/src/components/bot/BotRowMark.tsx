/**
 * A found row's mark: a session's App logo, a task's or an Account's
 * tile, a removed record's quiet tile.
 */
import type { JSX } from "react";
import { LogoTile, ToolLogo } from "../ToolLogo";
import { IconPool } from "../icons-actions";
import { IconPerson, IconTaskBox } from "./bot-icons";
import { rowPlace } from "../../lib/bot-words";
import type { BotRowInfo } from "../../types/generated/BotRowInfo";

export function RowMarkFor({ row }: { row: BotRowInfo }): JSX.Element {
  if (row.gone) return <LogoTile icon={row.kind === "task" ? IconPool : IconTaskBox} size={24} />;
  if (row.kind === "session" && row.provider) {
    return <ToolLogo tool={row.provider} kind={row.desktop ? "desktop" : "cli"} name={rowPlace(row).split(" · ")[1]} size={24} />;
  }
  return <LogoTile icon={row.kind === "task" ? IconTaskBox : IconPerson} size={24} />;
}
