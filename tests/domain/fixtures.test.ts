import { describe, it, expect } from "vitest";
import { validateContextSource, validateContextDocument } from "@/domain/context/validators";
import * as fs from "node:fs";
import * as path from "node:path";

const FIXTURES_DIR = path.resolve(__dirname, "../fixtures/context");

function loadJson(name: string) {
  return JSON.parse(fs.readFileSync(path.join(FIXTURES_DIR, name), "utf-8"));
}

describe("fixtures — domain contracts", () => {
  it("valid-source.json passes ContextSource validation", () => {
    const data = loadJson("valid-source.json");
    const result = validateContextSource(data);
    expect(result.valid).toBe(true);
  });

  it("valid-document.json passes ContextDocument validation", () => {
    const data = loadJson("valid-document.json");
    const result = validateContextDocument(data);
    expect(result.valid).toBe(true);
  });

  it("invalid-source.json fails validation with structured errors", () => {
    const data = loadJson("invalid-source.json");
    const result = validateContextSource(data);
    expect(result.valid).toBe(false);
    expect(result.errors.length).toBeGreaterThan(0);
  });

  it("invalid-document.json fails validation with structured errors", () => {
    const data = loadJson("invalid-document.json");
    const result = validateContextDocument(data);
    expect(result.valid).toBe(false);
    expect(result.errors.some((e) => e.path === "freshness.state")).toBe(true);
  });
});
