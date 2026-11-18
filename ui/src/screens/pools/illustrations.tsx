/**
 * 0.4.6 (D-126): New pool's line art — one small drawing per step, ink
 * greys and one accent (the kit: "making adds … one small line
 * illustration per step"; monitoring screens keep none). Drawn from the
 * theme's tokens, so both themes hold; decorative (`aria-hidden`), the
 * words beside them say it all.
 */
import type { JSX, ReactNode } from "react";

const LINE = { stroke: "var(--color-faint)", strokeOpacity: 0.7, strokeWidth: 1.5, fill: "none" } as const;
const EDGE = "var(--color-hairline)";
const ACCENT = "var(--color-accent)";
const NODE = "var(--color-surface-3)";

function Art({ width, height, children, testId, fluid = false }: { width: number; height: number; children: ReactNode; testId?: string; fluid?: boolean }): JSX.Element {
  return (
    <svg aria-hidden="true" width={fluid ? "100%" : width} height={height} viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMidYMid meet" fill="none" data-testid={testId}>
      {children}
    </svg>
  );
}

function Box({ x, y, w, h, on = false }: { x: number; y: number; w: number; h: number; on?: boolean }): JSX.Element {
  return <rect x={x} y={y} width={w} height={h} rx={Math.min(10, h / 2.6)} fill={NODE} stroke={on ? ACCENT : EDGE} strokeWidth={on ? 1.5 : 1} />;
}

function Bars({ x, y, w }: { x: number; y: number; w: number }): JSX.Element {
  return (
    <>
      <rect x={x} y={y} width={w} height={4} rx={2} fill="var(--color-faint)" opacity={0.55} />
      <rect x={x} y={y + 8} width={w * 0.62} height={4} rx={2} fill="var(--color-faint)" opacity={0.35} />
    </>
  );
}

function Task({ x, y }: { x: number; y: number }): JSX.Element {
  return (
    <g>
      <rect x={x} y={y} width={22} height={16} rx={4} fill="var(--color-surface)" stroke="var(--color-faint)" strokeOpacity={0.7} />
      <path d={`M${x + 5} ${y + 6}h12M${x + 5} ${y + 10.5}h8`} stroke="var(--color-faint)" strokeWidth={1.4} strokeLinecap="round" />
    </g>
  );
}

function Envelope({ x, y, on = false }: { x: number; y: number; on?: boolean }): JSX.Element {
  const c = on ? ACCENT : "var(--color-faint)";
  return (
    <g>
      <rect x={x} y={y} width={22} height={15} rx={3} fill="var(--color-surface)" stroke={c} strokeWidth={1.4} />
      <path d={`m${x + 2} ${y + 2.5} 9 6.5 9-6.5`} fill="none" stroke={c} strokeWidth={1.4} strokeLinejoin="round" />
    </g>
  );
}

/** A 24-box glyph, placed and scaled. */
function Glyph({ x, y, size, d, color = "var(--color-ink-2)", width = 1.8 }: { x: number; y: number; size: number; d: string; color?: string; width?: number }): JSX.Element {
  return (
    <g transform={`translate(${x} ${y}) scale(${size / 24})`}>
      <path d={d} stroke={color} strokeWidth={width} strokeLinecap="round" strokeLinejoin="round" fill="none" />
    </g>
  );
}

const CROWN = "M4 17.5 3 7.5l5 4 4-6 4 6 5-4-1 10ZM5 20.5h14";
const CHECK = "M20 6 9 17l-5-5";
const PERSON = "M12 4.5a3.5 3.5 0 1 1 0 7 3.5 3.5 0 0 1 0-7ZM5 20c.8-3.6 3.6-5.5 7-5.5s6.2 1.9 7 5.5";
const EYE = "M2.5 12s3.5-6.5 9.5-6.5 9.5 6.5 9.5 6.5-3.5 6.5-9.5 6.5S2.5 12 2.5 12ZM12 9.2a2.8 2.8 0 1 1 0 5.6 2.8 2.8 0 0 1 0-5.6Z";
const ZAP = "M13 3 5 13.5h6L10 21l8-10.5h-6Z";

/** The start step's side: you, the lead, the team, a check. */
export function HeroArt(): JSX.Element {
  return (
    <Art width={320} height={150} testId="new-pool-art-hero">
      <path d="M58 75H112M178 66C210 66 214 34 246 34M178 84C210 84 214 116 246 116M268 50v48" {...LINE} />
      <circle cx={36} cy={75} r={22} fill={NODE} stroke={EDGE} />
      <Glyph x={25} y={64} size={22} d={PERSON} />
      <Box x={112} y={58} w={66} h={34} on />
      <Glyph x={119} y={66} size={18} d={CROWN} color={ACCENT} />
      <Bars x={141} y={69} w={28} />
      <Box x={246} y={18} w={66} h={32} />
      <Bars x={256} y={28} w={46} />
      <Box x={246} y={100} w={66} h={32} />
      <Bars x={256} y={110} w={46} />
      <circle cx={268} cy={75} r={11} fill="var(--color-surface)" stroke="var(--color-faint)" strokeOpacity={0.7} />
      <Glyph x={261} y={68} size={14} d={CHECK} width={2.2} />
      <Task x={196} y={40} />
    </Art>
  );
}

/** Who leads: the lead hands tasks to two others. */
export function LeadArt(): JSX.Element {
  return (
    <Art width={300} height={132} testId="new-pool-art-lead">
      <path d="M150 44C150 70 70 66 70 92M150 44C150 70 230 66 230 92" {...LINE} />
      <Box x={108} y={12} w={84} h={32} on />
      <Glyph x={118} y={19} size={18} d={CROWN} color={ACCENT} />
      <Bars x={142} y={22} w={38} />
      <Box x={28} y={92} w={84} h={30} />
      <Bars x={40} y={101} w={58} />
      <Box x={188} y={92} w={84} h={30} />
      <Bars x={200} y={101} w={58} />
      <Task x={96} y={58} />
      <Task x={182} y={58} />
    </Art>
  );
}

/** On their own: Look only, Work together, Work together and wake each other. */
export function OwnArt({ level, on }: { level: string; on: boolean }): JSX.Element {
  const ends = (
    <>
      <Box x={14} y={30} w={48} h={36} />
      <Bars x={24} y={42} w={28} />
      <Box x={158} y={30} w={48} h={36} />
      <Bars x={168} y={42} w={28} />
    </>
  );
  if (level === "read")
    return (
      <Art width={220} height={96} fluid>
        <path d="M62 48H158" {...LINE} />
        {ends}
        <circle cx={110} cy={48} r={15} fill="var(--color-surface)" stroke="var(--color-faint)" strokeOpacity={0.7} />
        <Glyph x={100} y={38} size={20} d={EYE} color={on ? ACCENT : "var(--color-ink-2)"} />
      </Art>
    );
  if (level === "full")
    return (
      <Art width={220} height={96} fluid>
        <path d="M62 48H158" {...LINE} />
        {ends}
        <Envelope x={96} y={30} />
        <circle cx={200} cy={28} r={12} fill="var(--color-surface)" stroke={on ? ACCENT : "var(--color-faint)"} strokeWidth={1.4} />
        <Glyph x={193} y={21} size={14} d={ZAP} color={on ? ACCENT : "var(--color-ink-2)"} />
      </Art>
    );
  return (
    <Art width={220} height={96} fluid>
      <path d="M62 40H158M62 56H158" {...LINE} />
      {ends}
      <Envelope x={84} y={21} on={on} />
      <Task x={116} y={50} />
    </Art>
  );
}

/** Messages: three dots, the lead's in the accent; "ask" a waiting mark
 * between the members, "lead" no line between them. */
export function TalkArt({ preset }: { preset: string }): JSX.Element {
  return (
    <Art width={120} height={48}>
      <path d={`M60 10 30 38M60 10 90 38${preset !== "lead" ? "M30 38H90" : ""}`} {...LINE} strokeWidth={1.4} />
      {preset === "ask" && (
        <>
          <circle cx={60} cy={38} r={7} fill="var(--color-canvas-hi)" stroke="var(--color-warning)" strokeWidth={1.4} />
          <circle cx={60} cy={38} r={2.4} fill="var(--color-warning)" />
        </>
      )}
      <circle cx={60} cy={10} r={6} fill={ACCENT} />
      <circle cx={30} cy={38} r={5.5} fill={NODE} stroke="var(--color-faint)" strokeOpacity={0.7} />
      <circle cx={90} cy={38} r={5.5} fill={NODE} stroke="var(--color-faint)" strokeOpacity={0.7} />
    </Art>
  );
}

/** A template's team in small: role dots, the lead in the accent. */
export function TemplateArt({ shape }: { shape: string }): JSX.Element {
  const dot = (x: number, y: number, lead = false) => <circle key={`${x}-${y}`} cx={x} cy={y} r={lead ? 6 : 5} fill={lead ? ACCENT : NODE} stroke={lead ? "none" : "var(--color-faint)"} strokeOpacity={0.7} />;
  const line = (d: string) => <path key={d} d={d} {...LINE} strokeWidth={1.3} />;
  const frame = (x: number, w: number, rx = 5) => <rect key={x} x={x} y={6} width={w} height={28} rx={rx} fill="none" stroke="var(--color-faint)" strokeOpacity={0.7} />;
  const shapes: Record<string, ReactNode> = {
    pair: [line("M16 20H48"), dot(16, 20), dot(48, 20), <Glyph key="c" x={41} y={6} size={11} d={CHECK} width={2.5} />],
    team: [line("M32 10 12 30M32 10V30M32 10 52 30"), dot(32, 10, true), dot(12, 30), dot(32, 30), dot(52, 30)],
    repos: [frame(4, 24), frame(36, 24), line("M22 20H42"), dot(16, 20), dot(48, 20)],
    reviewer: [line("M32 12 12 30M32 12V30M32 12 52 30"), dot(12, 30), dot(32, 30), dot(52, 30), <circle key="r" cx={32} cy={12} r={6} fill={NODE} stroke="var(--color-faint)" strokeOpacity={0.7} />],
    account: [<rect key="a" x={6} y={4} width={52} height={32} rx={6} fill="none" stroke="var(--color-faint)" strokeOpacity={0.7} />, dot(20, 20), dot(32, 20), dot(44, 20)],
    mac: [<rect key="m" x={34} y={8} width={24} height={18} rx={3} fill="none" stroke="var(--color-faint)" strokeOpacity={0.7} />, line("M40 31h12"), line("M14 18H34"), dot(14, 18)],
    sealed: [<rect key="s" x={6} y={4} width={52} height={32} rx={8} fill="none" stroke="var(--color-faint)" strokeOpacity={0.7} strokeWidth={1.6} />, line("M22 20H42"), dot(22, 20), dot(42, 20)],
    scouts: [dot(12, 10), dot(32, 30), dot(52, 10), dot(22, 30), dot(44, 30)],
  };
  return (
    <Art width={64} height={40} testId="new-pool-art-template">
      {shapes[shape] ?? shapes.team}
    </Art>
  );
}

/** Team tools: two apps' sessions talking through AI Juggler. `left`
 * and `right` are their logos, drawn by the caller over the ends. */
export function ToolsArt(): JSX.Element {
  return (
    <Art width={420} height={110} testId="new-pool-art-tools">
      <path d="M96 52H178M242 52H324" {...LINE} />
      <circle cx={210} cy={52} r={32} fill="color-mix(in srgb, var(--color-accent) 20%, var(--color-surface))" stroke={ACCENT} strokeWidth={1.5} />
      <Envelope x={126} y={26} on />
      <Task x={272} y={62} />
    </Art>
  );
}
