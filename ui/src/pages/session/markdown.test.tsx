/**
 * AJ-E2108 vitest — an agent's markdown as elements, never as HTML:
 * headings, lists, quotes, tables, fenced code, inline marks; links shown
 * but not followed; a tag in the text stays text.
 */
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Markdown, blocksOf, plainWords } from "./Markdown";

afterEach(cleanup);

describe("AJ-E2108 markdown", () => {
  it("splits text into its blocks", () => {
    const kinds = blocksOf("# Title\n\nA para\nstill it.\n\n- one\n- two\n\n1. first\n2. second\n\n```ts\nconst a = 1;\n```\n\n> quoted\n\n| a | b |\n|---|---|\n| 1 | 2 |").map((block) => block.kind);
    expect(kinds).toEqual(["heading", "para", "list", "list", "code", "quote", "table"]);
  });

  it("renders code in monospace, marks inline, and never follows a link", () => {
    const { container } = render(<Markdown text={"Use `pnpm test` and **read** the [docs](https://example.test/x).\n\n```sh\nrm -rf <dir>\n```"} />);
    expect(container.querySelector("code")?.textContent).toBe("pnpm test");
    expect(container.querySelector("strong")?.textContent).toBe("read");
    expect(container.querySelector("a")).toBeNull();
    const link = container.querySelector("[data-testid='md-link']")!;
    expect(link.textContent).toBe("docs");
    expect(link.getAttribute("title")).toBe("https://example.test/x");
    expect(container.querySelector("pre")?.textContent).toBe("rm -rf <dir>");
    expect(container.querySelector("figcaption")?.textContent).toBe("sh");
  });

  it("keeps HTML in a message as text", () => {
    const { container } = render(<Markdown text={'<img src=x onerror="alert(1)"> <b>bold?</b>'} />);
    expect(container.querySelector("img")).toBeNull();
    expect(container.querySelector("b")).toBeNull();
    expect(container.textContent).toContain("<img src=x");
  });

  it("an unclosed fence still shows its code", () => {
    const { container } = render(<Markdown text={"```\nhalf written"} />);
    expect(container.querySelector("pre")?.textContent).toBe("half written");
  });

  it("plain words take the marks off", () => {
    expect(plainWords("Should I **rebase** `main` or [open](u) it?")).toBe("Should I rebase main or open it?");
  });
});
