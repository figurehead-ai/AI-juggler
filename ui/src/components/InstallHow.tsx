/**
 * AJ-E1525: how to install a tool, from its spec (first run's notes).
 */
import type { JSX } from "react";
import type { ProviderInstallData } from "../lib/bridge";
import { toolSpec } from "../lib/tools";

/** How to install a tool, from its spec: its documented command, else its
 * install page, else just the full stop. */
export function InstallHow({ tool, row }: { tool: string; row: ProviderInstallData | null }): JSX.Element {
  const spec = toolSpec(tool);
  if (spec.install_command) {
    return (
      <>
        :{" "}<span className="font-mono text-xs">{spec.install_command}</span>
      </>
    );
  }
  const url = row?.install_url || spec.install_url;
  if (url) {
    return (
      <>
        {" "}from{" "}
        <a className="underline" href={url} target="_blank" rel="noreferrer">
          its install page
        </a>
        .
      </>
    );
  }
  return <>.</>;
}
