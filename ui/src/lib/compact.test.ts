import { describe, expect, it } from "vitest";
import { byteSize, memorySize } from "./compact";
import { bytesWords } from "./machine";

// 0.1.7: eleven byte formatters became two rules on one implementation:
// files and disks in decimal units (Finder), memory as macOS counts it.
describe("byte sizes", () => {
  it("bytes, KB and MB are whole; GB one decimal under 10", () => {
    expect(byteSize(0)).toBe("0 B");
    expect(byteSize(512)).toBe("512 B");
    expect(byteSize(2_000)).toBe("2 KB");
    expect(byteSize(640_400_000)).toBe("640 MB");
    expect(byteSize(3_200_000_000)).toBe("3.2 GB");
    expect(byteSize(24_000_000_000)).toBe("24 GB");
    expect(byteSize(2_500_000_000_000)).toBe("2.5 TB");
  });

  it("a size that rounds up to the next unit reads in that unit", () => {
    expect(byteSize(999.6)).toBe("1 KB");
    expect(byteSize(999_600)).toBe("1 MB");
    expect(byteSize(999_600_000)).toBe("1.0 GB");
    expect(byteSize(9_960_000_000)).toBe("10 GB");
  });

  it("memory is counted as macOS counts it", () => {
    expect(memorySize(32 * 1024 ** 3)).toBe("32 GB");
    expect(memorySize(103_079_215_104)).toBe("96 GB");
    expect(memorySize(8.6 * 1024 ** 3)).toBe("8.6 GB");
    expect(memorySize(640 * 1024 ** 2)).toBe("640 MB");
    expect(memorySize(1_000 * 1024 ** 2)).toBe("1.0 GB");
  });

  it("This Mac keeps its floor for memory", () => {
    expect(bytesWords(123_456)).toBe("under 1 MB");
    expect(bytesWords(32 * 1024 ** 3)).toBe("32 GB");
  });
});
