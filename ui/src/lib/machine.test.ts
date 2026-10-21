import { describe, expect, it } from "vitest";
import { bytesWords, machineHealth, mockMachineHealth } from "./machine";

describe("machine health (AJ-E1401)", () => {
  it("serves each fixture state in the preview", async () => {
    expect((await machineHealth()).summary.words).toBe("Plenty of room");
    (window as unknown as { __AIJ_MACHINE__?: string }).__AIJ_MACHINE__ = "pressure";
    const pressure = await machineHealth();
    expect(pressure.summary.level).toBe("pressure");
    expect(pressure.summary.reasons[0].words).toBe("macOS reports critical memory pressure");
    delete (window as unknown as { __AIJ_MACHINE__?: string }).__AIJ_MACHINE__;
  });

  it("keeps five minutes of 2 s points, the first CPU unmeasured", () => {
    const tight = mockMachineHealth("tight", 1_000_000);
    expect(tight.series).toHaveLength(150);
    expect(tight.series[0].cpu_pct).toBeNull();
    expect(tight.series[149].at_ms).toBe(1_000_000);
    expect(mockMachineHealth("unsampled").sampled).toBe(false);
  });

  it("words bytes like the backend", () => {
    expect(bytesWords(3_200_000_000)).toBe("3.2 GB");
    expect(bytesWords(24_000_000_000)).toBe("24 GB");
    expect(bytesWords(640_000_000)).toBe("640 MB");
    expect(bytesWords(123_456)).toBe("under 1 MB");
  });
});

import { mockHeavyNote, mockThisMac, resourceLine } from "./machine";
import type { InstanceInfo } from "./bridge";

describe("This Mac (AJ-E1403)", () => {
  const session = (id: string, status: string, cpu: number | null, rss: number | null, helpers: number | null) =>
    ({ id, status, cpu_pct: cpu, rss_bytes: rss, child_count: helpers, account_label: "Work Max" }) as unknown as InstanceInfo;

  it("splits ours from theirs, ranks by memory and notes only heavy trees", () => {
    const info = mockThisMac(
      [
        session("a", "running", 12, 900_000_000, 2),
        session("b", "running", 96, 3_200_000_000, 5),
        session("c", "finished", 0, 0, 0),
      ],
      mockMachineHealth("plenty"),
    );
    expect(info.ours.count).toBe(2);
    expect(info.theirs.count).toBe(1);
    expect(info.top.map((tree) => tree.session_id)).toEqual(["b", null, "a"]);
    expect(info.heavy.map((tree) => tree.heavy_note)).toEqual(["CPU 96% · 3.2 GB"]);
    // (12 + 96 + 14) % of one core over 10 cores.
    expect(info.tools.cpu_pct).toBeCloseTo(12.2);
  });

  it("an unmeasured session leaves the CPU total unknown", () => {
    const info = mockThisMac([session("a", "running", null, 500_000_000, 0)], mockMachineHealth("plenty"), []);
    expect(info.tools.cpu_pct).toBeNull();
  });

  it("words a heavy tree and a session's resources", () => {
    expect(mockHeavyNote(40, 1e9)).toBeNull();
    expect(mockHeavyNote(95, 1e9)).toBe("CPU 95%");
    expect(resourceLine(12.4, 1_100_000_000, 3)).toBe("CPU 12% · 1.1 GB · 3 helpers");
    expect(resourceLine(null, null, 1)).toBe("CPU — · memory — · 1 helper");
  });
});
