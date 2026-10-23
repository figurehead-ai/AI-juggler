/**
 * AJ-E1506 vitest — the terminal tools' pure parts: which text is a file
 * path, what Copy takes, and the tools' keys (⌘ on a Mac, Ctrl+Shift
 * elsewhere, never a key the shell owns).
 */
import { describe, expect, it } from "vitest";
import type { Terminal } from "@xterm/xterm";
import { findPathLinks } from "./terminalLinks";
import { copyText, terminalKey } from "./terminalTools";

const paths = (line: string) => findPathLinks(line).map((link) => link.text);

describe("file paths in terminal output", () => {
  it("finds what agents and compilers print, with line and column", () => {
    expect(paths("● Edit src/auth/session.test.ts")).toEqual(["src/auth/session.test.ts"]);
    expect(paths("error: crates/juggler-app/src/lib.rs:770:5: mismatched")).toEqual(["crates/juggler-app/src/lib.rs:770:5"]);
    expect(paths("see ./README.md, and ../other/x.ts.")).toEqual(["./README.md", "../other/x.ts"]);
    expect(paths("wrote ~/notes/todo.md (2 KB)")).toEqual(["~/notes/todo.md"]);
    expect(paths("(see src/a.tsx)")).toEqual(["src/a.tsx"]);
  });

  it("leaves web addresses to the web-links addon and prose alone", () => {
    expect(paths("open https://example.com/a/b.ts or /Users/me/app/main.go:3:7")).toEqual(["/Users/me/app/main.go:3:7"]);
    expect(paths("http://localhost:5173/index.html")).toEqual([]);
    expect(paths("1/2 and/or 2026/10/22 TCP/IP e.g. v1.2")).toEqual([]);
    expect(paths("cd / && ls ~/")).toEqual([]);
  });

  it("says where each path is in the line", () => {
    const [link] = findPathLinks("  at src/a.ts:3");
    expect(link).toEqual({ start: 5, end: 15, text: "src/a.ts:3" });
  });
});

describe("the terminal tools' keys", () => {
  const key = (init: KeyboardEventInit) => terminalKey(new KeyboardEvent("keydown", init));

  it("⌘ on a Mac", () => {
    expect(key({ metaKey: true, key: "f", code: "KeyF" })).toBe("find");
    expect(key({ metaKey: true, key: "=", code: "Equal" })).toBe("larger");
    expect(key({ metaKey: true, key: "-", code: "Minus" })).toBe("smaller");
    expect(key({ metaKey: true, key: "0", code: "Digit0" })).toBe("reset");
    expect(key({ metaKey: true, key: "Enter", code: "Enter" })).toBe("maximize");
  });

  it("Ctrl+Shift elsewhere; plain Ctrl stays the shell's", () => {
    expect(key({ ctrlKey: true, shiftKey: true, key: "F", code: "KeyF" })).toBe("find");
    expect(key({ ctrlKey: true, shiftKey: true, key: "+", code: "Equal" })).toBe("larger");
    for (const shell of [{ key: "f", code: "KeyF" }, { key: "-", code: "Minus" }, { key: "Enter", code: "Enter" }]) {
      expect(key({ ctrlKey: true, ...shell })).toBeNull();
    }
    expect(key({ key: "f", code: "KeyF" })).toBeNull();
    expect(key({ altKey: true, metaKey: true, key: "f", code: "KeyF" })).toBeNull();
    // Plain Ctrl+0 is the shell's too.
    expect(key({ ctrlKey: true, key: "0", code: "Digit0" })).toBeNull();
  });
});

describe("Copy", () => {
  const term = (selection: string, screen: string[]) =>
    ({
      rows: 4,
      getSelection: () => selection,
      buffer: { active: { viewportY: 0, getLine: (y: number) => (y < screen.length ? { translateToString: () => screen[y] } : undefined) } },
    }) as unknown as Terminal;

  it("takes the selection, or else what's on screen without trailing blank lines", () => {
    expect(copyText(term("npm test", ["a", "b"]))).toBe("npm test");
    expect(copyText(term("", ["$ npm test", "ok", "", ""]))).toBe("$ npm test\nok");
  });
});
