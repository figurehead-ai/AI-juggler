/**
 * AJ-E1532 — the colour maths the palette is built and checked with:
 * OKLCH to sRGB (Björn Ottosson's OKLab), whether a colour fits in sRGB,
 * the most chroma a hue takes at a lightness, WCAG contrast, and the
 * distance between two colours in OKLab (ΔE).
 */

export type Oklch = { l: number; c: number; h: number };

/** Linear sRGB, each channel 0–1 when the colour fits. */
export function linearRgb({ l, c, h }: Oklch): [number, number, number] {
  const a = c * Math.cos((h * Math.PI) / 180);
  const b = c * Math.sin((h * Math.PI) / 180);
  const l1 = (l + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m1 = (l - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s1 = (l - 0.0894841775 * a - 1.291485548 * b) ** 3;
  return [
    4.0767416621 * l1 - 3.3077115913 * m1 + 0.2309699292 * s1,
    -1.2684380046 * l1 + 2.6097574011 * m1 - 0.3413193965 * s1,
    -0.0041960863 * l1 - 0.7034186147 * m1 + 1.707614701 * s1,
  ];
}

/** Whether the colour fits in sRGB (a hair of rounding allowed). */
export function inGamut(colour: Oklch): boolean {
  return linearRgb(colour).every((channel) => channel >= -1e-4 && channel <= 1 + 1e-4);
}

/** The most chroma `h` takes at lightness `l` and still fits in sRGB. */
export function maxChroma(l: number, h: number): number {
  let lo = 0;
  let hi = 0.4;
  for (let step = 0; step < 32; step += 1) {
    const mid = (lo + hi) / 2;
    if (inGamut({ l, c: mid, h })) lo = mid;
    else hi = mid;
  }
  return lo;
}

/** WCAG relative luminance. */
export function luminance(colour: Oklch): number {
  const [r, g, b] = linearRgb(colour).map((channel) => Math.min(1, Math.max(0, channel)));
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

/** sRGB hex (`#rrggbb`) as linear-light luminance. */
export function hexLuminance(hex: string): number {
  const channel = (offset: number) => {
    const value = parseInt(hex.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** WCAG contrast ratio of two luminances. */
export function contrast(a: number, b: number): number {
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** ΔE in OKLab (straight-line distance; 0.02 is about a just-noticeable step). */
export function deltaE(x: Oklch, y: Oklch): number {
  const ab = ({ l, c, h }: Oklch) => [l, c * Math.cos((h * Math.PI) / 180), c * Math.sin((h * Math.PI) / 180)];
  const [p, q] = [ab(x), ab(y)];
  return Math.hypot(p[0]! - q[0]!, p[1]! - q[1]!, p[2]! - q[2]!);
}
