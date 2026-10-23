/**
 * AJ-E1518 — the Privacy page: what AI Juggler reads, changes and sends,
 * and what it never does. Every line comes from `lib/trust-claims.json`,
 * the same list `docs/guide/trust.md` carries, and names the tests that
 * prove it (`crates/juggler-app/tests/trust.rs` checks each one exists).
 * The tests stay behind each line's disclosure (D-25). Below, what each
 * tool's own terms say about several accounts, from its adapter.
 */
import type { JSX, ReactNode } from "react";
import { SectionTitle } from "../components/SectionTitle";
import { ToolMark } from "../components/ToolMark";
import { useTools } from "../lib/tools";
import claims from "../lib/trust-claims.json";

/** The day, in the user's own words ("Oct 23, 2026"), for the `YYYY-MM-DD` a terms line was checked on. */
function checkedWords(day: string): string {
  const date = new Date(`${day}T12:00:00`);
  return Number.isNaN(date.getTime()) ? day : date.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" });
}

/** Where a test's file can be read. */
const SOURCE = "https://github.com/figurehead-ai/AI-juggler/blob/main/";

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }): JSX.Element {
  return (
    <section aria-labelledby={`${id}-title`} className="flex flex-col gap-2.5" data-testid={`privacy-${id}`}>
      <SectionTitle id={`${id}-title`}>{title}</SectionTitle>
      <ul className="flex flex-col rounded-xl border border-hairline bg-surface px-4 py-1.5">{children}</ul>
    </section>
  );
}

function ClaimRow({ text, tests }: { text: string; tests: string[] }): JSX.Element {
  return (
    <li className="border-t border-hairline/60 py-2.5 first:border-t-0" data-testid="privacy-claim">
      <details className="group">
        <summary className="flex cursor-pointer list-none items-start gap-2.5 rounded-lg text-sm text-ink outline-none focus-visible:ring-1 focus-visible:ring-accent/60">
          {/* The rail's fold chevron: one look for every disclosure. */}
          <svg aria-hidden="true" width="10" height="10" viewBox="0 0 10 10" className="mt-1.5 shrink-0 text-faint transition-transform group-open:rotate-90 motion-reduce:transition-none">
            <path d="M3.5 2 L7 5 L3.5 8" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
          <span className="flex-1">{text}</span>
        </summary>
        <div className="ml-5 mt-1.5 flex flex-col gap-1 text-xs text-muted" data-testid="privacy-tests">
          <span>{tests.length === 1 ? "Checked by this test:" : `Checked by these ${tests.length} tests:`}</span>
          <ul className="flex flex-col gap-0.5">
            {tests.map((reference) => {
              const [path, name] = reference.split("::");
              return (
                <li key={reference}>
                  <a href={`${SOURCE}${path}`} target="_blank" rel="noreferrer" className="font-mono text-[11px] text-accent underline-offset-2 hover:underline">
                    {name}
                  </a>
                  <span className="text-faint"> · {path}</span>
                </li>
              );
            })}
          </ul>
        </div>
      </details>
    </li>
  );
}

export function PrivacyScreen(): JSX.Element {
  const tools = useTools().filter((tool) => tool.terms !== null);
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-7 py-2" data-testid="privacy-page">
      <header className="flex flex-col gap-1.5">
        <h1 className="text-2xl font-semibold tracking-tight">Privacy</h1>
        <p className="text-sm text-muted">
          What AI Juggler reads, changes and sends, and what it never does. {claims.checked}
        </p>
      </header>
      {claims.groups.map((group) => (
        <Section key={group.id} id={group.id} title={group.title}>
          {group.claims.map((claim) => (
            <ClaimRow key={claim.id} text={claim.text} tests={claim.tests} />
          ))}
        </Section>
      ))}
      {tools.length > 0 && (
        <Section id="terms" title="Your accounts and the tools' terms">
          {tools.map((tool) => (
            <li key={tool.id} className="flex items-start gap-2.5 border-t border-hairline/60 py-2.5 text-sm first:border-t-0" data-testid="privacy-terms-row">
              <ToolMark tool={tool.id} size={14} className="mt-0.5 shrink-0 text-muted" />
              <span className="flex flex-col gap-0.5">
                <span className="text-ink">{tool.terms!.accounts}</span>
                <span className="text-xs text-muted">
                  <a href={tool.terms!.url} target="_blank" rel="noreferrer" className="text-accent underline underline-offset-2">
                    {tool.terms!.name} ↗
                  </a>{" "}
                  · checked {checkedWords(tool.terms!.checked)}
                </span>
              </span>
            </li>
          ))}
        </Section>
      )}
      <p className="text-xs text-muted">
        Use only accounts that are yours: AI Juggler never shares, pools or rotates them. Found a line that isn't true? Please{" "}
        <a
          href="https://github.com/figurehead-ai/AI-juggler/security/advisories/new"
          target="_blank"
          rel="noreferrer"
          className="text-accent underline underline-offset-2"
        >
          report it privately
        </a>
        : it's a security bug.
      </p>
    </div>
  );
}
