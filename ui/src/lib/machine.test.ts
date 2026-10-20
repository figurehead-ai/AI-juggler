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
  });
});
