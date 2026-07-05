import { describe, it, expect } from "vitest";
import {
  formatTokens,
  formatRequests,
  formatNumber,
  formatLatency,
  formatSpeed,
  formatPercent,
} from "@/lib/format";

describe("formatTokens", () => {
  it("returns 0 for falsy input", () => {
    expect(formatTokens(0)).toBe("0");
  });
  it("formats thousands with K suffix", () => {
    expect(formatTokens(1_500)).toBe("1.5K");
    expect(formatTokens(999_999)).toBe("1000.0K");
  });
  it("formats millions with M suffix", () => {
    expect(formatTokens(1_500_000)).toBe("1.50M");
  });
  it("formats billions with B suffix", () => {
    expect(formatTokens(2_500_000_000)).toBe("2.50B");
  });
});

describe("formatRequests", () => {
  it("returns 0 for falsy input", () => {
    expect(formatRequests(0)).toBe("0");
  });
  it("formats thousands with K suffix", () => {
    expect(formatRequests(1_500)).toBe("1.5K");
  });
  it("formats millions with M suffix", () => {
    expect(formatRequests(1_500_000)).toBe("1.5M");
  });
});

describe("formatNumber", () => {
  it("formats with locale group separators", () => {
    expect(formatNumber(1234567)).toBe("1,234,567");
  });
});

describe("formatLatency", () => {
  it("returns em-dash for falsy/zero input", () => {
    expect(formatLatency(0)).toBe("—");
  });
  it("returns ms for sub-second values", () => {
    expect(formatLatency(0.123)).toBe("123ms");
  });
  it("returns s for values >= 1 second", () => {
    expect(formatLatency(2.34)).toBe("2.3s");
  });
});

describe("formatSpeed", () => {
  it("returns em-dash for all-zero input", () => {
    expect(formatSpeed(0, 0)).toBe("—");
  });
  it("combines prefill and gen with separator", () => {
    expect(formatSpeed(5000, 42)).toBe("5K prefill/s · 42 gen/s");
  });
});

describe("formatPercent", () => {
  it("returns 0.0% when total is zero", () => {
    expect(formatPercent(5, 0)).toBe("0.0%");
  });
  it("computes percentage of total", () => {
    expect(formatPercent(25, 100)).toBe("25.0%");
  });
});
