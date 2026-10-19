/**
 * AJ-E1010 — release notes as published (GitHub Release body, Markdown),
 * rendered as plain React elements: `##` headings, `-` bullets and
 * `**bold**`. Nothing is ever injected as HTML; anything else shows as
 * text.
 */
import type { ReactNode } from "react";

function inline(text: string): ReactNode[] {
  return text.split(/(\*\*[^*]+\*\*)/g).map((part, index) =>
    part.startsWith("**") && part.endsWith("**") && part.length > 4 ? (
      <strong key={index} className="font-semibold text-ink">
        {part.slice(2, -2)}
      </strong>
    ) : (
      part
    ),
  );
}

export function ReleaseNotes({ text }: { text: string }): JSX.Element {
  const blocks: ReactNode[] = [];
  let bullets: string[] = [];
  const flush = () => {
    if (bullets.length === 0) return;
    blocks.push(
      <ul key={`ul-${blocks.length}`} className="ml-4 list-disc">
        {bullets.map((item, index) => (
          <li key={index}>{inline(item)}</li>
        ))}
      </ul>,
    );
    bullets = [];
  };
  for (const raw of text.split("\n")) {
    const line = raw.trimEnd();
    if (/^\s*[-*] /.test(line)) {
      bullets.push(line.replace(/^\s*[-*] /, ""));
      continue;
    }
    flush();
    if (line.startsWith("#")) {
      blocks.push(
        <p key={`h-${blocks.length}`} className="mt-1 font-semibold text-ink">
          {line.replace(/^#+\s*/, "")}
        </p>,
      );
    } else if (line.trim()) {
      blocks.push(<p key={`p-${blocks.length}`}>{inline(line)}</p>);
    }
  }
  flush();
  return <div className="flex flex-col gap-1">{blocks}</div>;
}
