/**
 * AJ-E2108 — an agent's words as its own app shows them: headings, lists,
 * quotes, tables, inline code, fenced code in monospace, bold and italic.
 * Built as React elements from the text (never HTML from it, so nothing
 * in a transcript can run or style the page). Links show their words and
 * their address on hover but are never followed from here.
 */
import { Fragment, type JSX, type ReactNode } from "react";

type Block =
  | { kind: "code"; lang: string; text: string }
  | { kind: "heading"; level: number; text: string }
  | { kind: "list"; ordered: boolean; start: number; items: string[] }
  | { kind: "quote"; text: string }
  | { kind: "table"; head: string[]; rows: string[][] }
  | { kind: "rule" }
  | { kind: "para"; text: string };

const FENCE = /^\s*(```|~~~)\s*([\w+-]*)\s*$/;
const HEADING = /^(#{1,6})\s+(.*)$/;
const BULLET = /^\s*[-*+]\s+(.*)$/;
const NUMBERED = /^\s*(\d+)[.)]\s+(.*)$/;
const RULE = /^\s*([-*_])(\s*\1){2,}\s*$/;
const TABLE_SEP = /^\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*$/;

function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

/** The text's blocks, in order. */
export function blocksOf(text: string): Block[] {
  const lines = text.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index]!;
    const fence = FENCE.exec(line);
    if (fence) {
      const close = fence[1]!;
      const body: string[] = [];
      index += 1;
      while (index < lines.length && !lines[index]!.trim().startsWith(close)) {
        body.push(lines[index]!);
        index += 1;
      }
      index += 1; // the closing fence (or the end: an unclosed block)
      blocks.push({ kind: "code", lang: fence[2] ?? "", text: body.join("\n") });
      continue;
    }
    if (line.trim() === "") {
      index += 1;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", level: heading[1]!.length, text: heading[2]!.trim() });
      index += 1;
      continue;
    }
    if (RULE.test(line)) {
      blocks.push({ kind: "rule" });
      index += 1;
      continue;
    }
    if (line.includes("|") && index + 1 < lines.length && TABLE_SEP.test(lines[index + 1]!)) {
      const head = cells(line);
      const rows: string[][] = [];
      index += 2;
      while (index < lines.length && lines[index]!.includes("|") && lines[index]!.trim() !== "") {
        rows.push(cells(lines[index]!));
        index += 1;
      }
      blocks.push({ kind: "table", head, rows });
      continue;
    }
    if (line.trimStart().startsWith(">")) {
      const body: string[] = [];
      while (index < lines.length && lines[index]!.trimStart().startsWith(">")) {
        body.push(lines[index]!.trimStart().replace(/^>\s?/, ""));
        index += 1;
      }
      blocks.push({ kind: "quote", text: body.join("\n") });
      continue;
    }
    const bullet = BULLET.exec(line);
    const numbered = NUMBERED.exec(line);
    if (bullet || numbered) {
      const ordered = !bullet;
      const items: string[] = [];
      const start = numbered ? Number(numbered[1]) : 1;
      while (index < lines.length) {
        const current = lines[index]!;
        const item = ordered ? NUMBERED.exec(current) : BULLET.exec(current);
        if (item) {
          items.push((ordered ? item[2] : item[1])!.trim());
        } else if (current.trim() !== "" && /^\s{2,}\S/.test(current) && items.length > 0) {
          // A wrapped line of the item above.
          items[items.length - 1] += ` ${current.trim()}`;
        } else {
          break;
        }
        index += 1;
      }
      blocks.push({ kind: "list", ordered, start, items });
      continue;
    }
    const body: string[] = [];
    while (
      index < lines.length &&
      lines[index]!.trim() !== "" &&
      !FENCE.test(lines[index]!) &&
      !HEADING.test(lines[index]!) &&
      !BULLET.test(lines[index]!) &&
      !NUMBERED.test(lines[index]!) &&
      !lines[index]!.trimStart().startsWith(">")
    ) {
      body.push(lines[index]!);
      index += 1;
    }
    blocks.push({ kind: "para", text: body.join("\n") });
  }
  return blocks;
}

const INLINE = /(`[^`\n]+`)|(\*\*[^*\n]+\*\*|__[^_\n]+__)|(\*[^*\s][^*\n]*\*|_[^_\s][^_\n]*_)|(\[[^\]\n]+\]\([^)\s]+\))/g;

/** Inline marks: `code`, **bold**, *italic*, [a link](shown, not followed). */
export function inline(text: string, keyBase = "i"): ReactNode[] {
  const out: ReactNode[] = [];
  let last = 0;
  let n = 0;
  for (const match of text.matchAll(INLINE)) {
    const at = match.index ?? 0;
    if (at > last) out.push(text.slice(last, at));
    const [whole, code, bold, italic, link] = match;
    const key = `${keyBase}-${n++}`;
    if (code) {
      out.push(
        <code key={key} className="rounded-md border border-hairline bg-surface-2 px-1.5 py-px font-mono text-[0.86em] text-ink">
          {code.slice(1, -1)}
        </code>,
      );
    } else if (bold) {
      out.push(
        <strong key={key} className="font-semibold text-ink">
          {inline(bold.slice(2, -2), key)}
        </strong>,
      );
    } else if (italic) {
      out.push(<em key={key}>{inline(italic.slice(1, -1), key)}</em>);
    } else if (link) {
      const words = link.slice(1, link.indexOf("]("));
      const href = link.slice(link.indexOf("](") + 2, -1);
      out.push(
        <span key={key} title={href} className="text-accent-hi underline decoration-accent/40 underline-offset-2" data-testid="md-link">
          {words}
        </span>,
      );
    } else {
      out.push(whole);
    }
    last = at + whole.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** Lines kept as written inside a paragraph. */
function withBreaks(text: string, key: string): ReactNode[] {
  return text.split("\n").flatMap((line, index) => (index === 0 ? inline(line, `${key}-${index}`) : [<br key={`${key}-br-${index}`} />, ...inline(line, `${key}-${index}`)]));
}

export function Markdown({ text }: { text: string }): JSX.Element {
  const blocks = blocksOf(text);
  return (
    <div className="flex min-w-0 flex-col gap-3 text-[14px] leading-[1.6] text-ink" data-testid="md">
      {blocks.map((block, index) => {
        const key = `b${index}`;
        switch (block.kind) {
          case "code":
            return (
              <figure key={key} className="min-w-0 overflow-hidden rounded-xl border border-hairline bg-surface-2" data-testid="md-code">
                {block.lang && (
                  <figcaption className="border-b border-hairline px-3.5 py-1.5 font-mono text-[12px] text-ink/80">{block.lang}</figcaption>
                )}
                <pre className="whitespace-pre-wrap px-3.5 py-3 font-mono text-[13px] leading-[1.55] text-ink [overflow-wrap:anywhere]">
                  <code>{block.text}</code>
                </pre>
              </figure>
            );
          case "heading":
            return (
              <p key={key} className={"font-semibold text-ink " + (block.level <= 2 ? "text-[16px]" : "text-[15px]")}>
                {inline(block.text, key)}
              </p>
            );
          case "list": {
            const items = block.items.map((item, i) => (
              <li key={`${key}-${i}`} className="pl-1">
                {inline(item, `${key}-${i}`)}
              </li>
            ));
            return block.ordered ? (
              <ol key={key} start={block.start} className="flex list-decimal flex-col gap-1 pl-6 marker:text-ink/70">
                {items}
              </ol>
            ) : (
              <ul key={key} className="flex list-disc flex-col gap-1 pl-6 marker:text-ink/70">
                {items}
              </ul>
            );
          }
          case "quote":
            return (
              <blockquote key={key} className="rounded-lg bg-surface-2 px-3.5 py-2 text-ink/85">
                {withBreaks(block.text, key)}
              </blockquote>
            );
          case "table":
            return (
              <div key={key} className="min-w-0 overflow-x-auto rounded-xl border border-hairline" tabIndex={0} role="region" aria-label="Table">
                <table className="w-full border-collapse text-[13px] tabular-nums">
                  <thead className="bg-surface-2 text-left">
                    <tr>
                      {block.head.map((cell, i) => (
                        <th key={i} scope="col" className="px-3 py-1.5 font-semibold text-ink">
                          {inline(cell, `${key}-h${i}`)}
                        </th>
                      ))}
                    </tr>
                  </thead>
                  <tbody>
                    {block.rows.map((row, r) => (
                      <tr key={r} className="border-t border-hairline">
                        {row.map((cell, i) => (
                          <td key={i} className="px-3 py-1.5 text-ink">
                            {inline(cell, `${key}-${r}-${i}`)}
                          </td>
                        ))}
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            );
          case "rule":
            return <hr key={key} className="border-hairline" />;
          case "para":
            return (
              <p key={key} className="break-words">
                {withBreaks(block.text, key)}
              </p>
            );
        }
        return <Fragment key={key} />;
      })}
    </div>
  );
}

/** Plain words from markdown: marks taken off (the asks card's question). */
export function plainWords(text: string): string {
  return text
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]+)`/g, "$1")
    .replace(/\*\*([^*]+)\*\*|__([^_]+)__/g, "$1$2")
    .replace(/(^|\s)[*_]([^*_\n]+)[*_]/g, "$1$2")
    .replace(/\[([^\]]+)\]\([^)]+\)/g, "$1")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/^\s*[-*+]\s+/gm, "")
    .replace(/[ \t]+/g, " ")
    .trim();
}
