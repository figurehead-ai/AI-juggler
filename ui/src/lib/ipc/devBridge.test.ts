/**
 * The read-only test bridge transport: the token leaves the address bar,
 * every command the bridge doesn't serve is refused before anything is
 * sent, and reads carry the bearer token.
 */
import { afterEach, describe, expect, it, vi } from "vitest";
import { connect, READ_ONLY_MESSAGE, sessionFrom } from "./devBridge";

const TOKEN = "ab".repeat(32);

function memoryStorage(): Storage {
  const items = new Map<string, string>();
  return {
    get length() {
      return items.size;
    },
    clear: () => items.clear(),
    getItem: (key) => items.get(key) ?? null,
    key: (index) => [...items.keys()][index] ?? null,
    removeItem: (key) => void items.delete(key),
    setItem: (key, value) => void items.set(key, value),
  };
}

/** The page as the browser shows it, with the bridge parameters. */
function page(query: string): URL {
  window.history.replaceState(null, "", `/?${query}`);
  return new URL(window.location.href);
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("read-only test bridge — the page's session", () => {
  it("takes the token from the query or the fragment and strips it from the URL", () => {
    const storage = memoryStorage();
    const page = new URL(`http://localhost:5173/?devBridge=4321&token=${TOKEN}&x=1`);
    expect(sessionFrom(page, storage)).toEqual({ port: 4321, token: TOKEN });
    expect(page.toString()).toBe("http://localhost:5173/?devBridge=4321&x=1");

    const fragment = new URL(`http://localhost:5173/?devBridge=4321#token=${TOKEN}`);
    expect(sessionFrom(fragment, memoryStorage())).toEqual({ port: 4321, token: TOKEN });
    expect(fragment.toString()).toBe("http://localhost:5173/?devBridge=4321");
  });

  it("a reload in the same tab finds the session; another port doesn't", () => {
    const storage = memoryStorage();
    sessionFrom(new URL(`http://localhost:5173/?devBridge=4321&token=${TOKEN}`), storage);
    expect(sessionFrom(new URL("http://localhost:5173/?devBridge=4321"), storage)).toEqual({
      port: 4321,
      token: TOKEN,
    });
    expect(sessionFrom(new URL("http://localhost:5173/?devBridge=9999"), storage)).toBeNull();
  });

  it("refuses a malformed port or token", () => {
    for (const query of [
      `devBridge=0&token=${TOKEN}`,
      `devBridge=70000&token=${TOKEN}`,
      `devBridge=abc&token=${TOKEN}`,
      "devBridge=4321&token=short",
      `devBridge=4321&token=${"G".repeat(64)}`,
    ]) {
      expect(sessionFrom(new URL(`http://localhost:5173/?${query}`), memoryStorage())).toBeNull();
    }
  });
});

describe("read-only test bridge — the transport", () => {
  it("serves reads with the token and refuses every other command unsent", async () => {
    const fetched: { url: string; init?: RequestInit }[] = [];
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string, init?: RequestInit) => {
        fetched.push({ url, init });
        if (url.endsWith("/v1/commands")) return Response.json(["app_info"]);
        if (url.endsWith("/v1/events")) return new Response(new ReadableStream());
        return Response.json({ name: "AI Juggler" });
      }),
    );
    const transport = await connect(page(`devBridge=4321&token=${TOKEN}`));
    expect(window.location.href).not.toContain(TOKEN);

    await expect(transport.invoke("app_info")).resolves.toEqual({ name: "AI Juggler" });
    const call = fetched.find((entry) => entry.url.endsWith("/v1/invoke/app_info"));
    expect(call?.url).toBe("http://127.0.0.1:4321/v1/invoke/app_info");
    expect(new Headers(call?.init?.headers).get("Authorization")).toBe(`Bearer ${TOKEN}`);

    const before = fetched.length;
    await expect(transport.invoke("launch_session", { profileId: "p" })).rejects.toEqual({
      code: "read_only_bridge",
      retryable: false,
      user_message: READ_ONLY_MESSAGE,
    });
    expect(fetched.length).toBe(before);
    expect(document.querySelector("[role=status]")?.textContent).toContain(READ_ONLY_MESSAGE);
  });

  it("a 403 from the bridge reads as the same refusal", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url.endsWith("/v1/commands")) return Response.json(["app_info"]);
        if (url.endsWith("/v1/events")) return new Response(new ReadableStream());
        return Response.json({ code: "read_only_bridge" }, { status: 403 });
      }),
    );
    const transport = await connect(page(`devBridge=4321&token=${TOKEN}`));
    await expect(transport.invoke("app_info")).rejects.toMatchObject({ user_message: READ_ONLY_MESSAGE });
  });

  it("delivers the refresh events from the stream to their listeners", async () => {
    const encoder = new TextEncoder();
    const stream: { push?: (text: string) => void } = {};
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => {
        if (url.endsWith("/v1/commands")) return Response.json([]);
        return new Response(
          new ReadableStream({
            start(controller) {
              stream.push = (text) => controller.enqueue(encoder.encode(text));
            },
          }),
        );
      }),
    );
    const transport = await connect(page(`devBridge=4321&token=${TOKEN}`));
    const seen: unknown[] = [];
    await transport.listen<number[]>("session://updated", (event) => seen.push(event.payload));
    await transport.listen("session://quota", () => seen.push("quota"));
    await vi.waitFor(() => expect(stream.push).toBeDefined());
    stream.push?.(": open\n\nevent: session://updated\ndata: [1,");
    stream.push?.("2]\n\nevent: other\ndata: 3\n\n");
    await vi.waitFor(() => expect(seen).toEqual([[1, 2]]));
  });
});
