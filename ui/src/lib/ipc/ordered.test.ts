import { describe, expect, it } from "vitest";
import { orderedJoin, orderedLatest } from "./ordered";

/** A send whose calls finish only when the test says so. */
function gated<T>() {
  const calls: { key: string; value: T; done: () => void }[] = [];
  const send = (key: string, value: T) => new Promise<void>((resolve) => calls.push({ key, value, done: resolve }));
  return { calls, send };
}

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

describe("ordered sends (commands run off the main thread)", () => {
  it("keeps one write in flight per terminal and joins what's typed meanwhile, in order", async () => {
    const { calls, send } = gated<string>();
    const write = orderedJoin(send);
    void write("t1", "a");
    void write("t1", "b");
    void write("t1", "c");
    void write("t2", "x");
    expect(calls.map((c) => [c.key, c.value])).toEqual([
      ["t1", "a"],
      ["t2", "x"],
    ]);
    calls[0]!.done();
    await tick();
    expect(calls.map((c) => c.value)).toEqual(["a", "x", "bc"]);
    calls[2]!.done();
    await tick();
    expect(calls).toHaveLength(3);
    // Idle again: the next key goes at once.
    void write("t1", "d");
    expect(calls.map((c) => c.value)).toEqual(["a", "x", "bc", "d"]);
  });

  it("sends only the newest size after the one in flight", async () => {
    const { calls, send } = gated<number>();
    const resize = orderedLatest(send);
    void resize("t1", 80);
    void resize("t1", 90);
    void resize("t1", 100);
    expect(calls.map((c) => c.value)).toEqual([80]);
    calls[0]!.done();
    await tick();
    expect(calls.map((c) => c.value)).toEqual([80, 100]);
  });

  it("goes on after a failed send", async () => {
    const values: string[] = [];
    let fail = true;
    const write = orderedJoin(async (_key, data) => {
      values.push(data);
      if (fail) {
        fail = false;
        throw new Error("gone");
      }
    });
    void write("t1", "a");
    await write("t1", "b");
    expect(values).toEqual(["a", "b"]);
  });
});
