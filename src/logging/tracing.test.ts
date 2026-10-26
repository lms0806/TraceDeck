import { describe, expect, it } from "vitest";

import { parseTracing } from "./tracing";

describe("parseTracing", () => {
  it("parses timestamp, level, target, and message", () => {
    expect(
      parseTracing(
        "2026-10-25T14:09:36.097647Z  INFO backend::kms::logic::character::hexa_core_cost: use hexa_core_cost request",
      ),
    ).toEqual([
      {
        timestamp: "2026-10-25T14:09:36.097647Z",
        level: "INFO",
        target: "backend::kms::logic::character::hexa_core_cost",
        message: "use hexa_core_cost request",
      },
    ]);
  });

  it("removes ANSI styling before parsing", () => {
    const [record] = parseTracing(
      "\u001b[2m2026-10-25T14:09:36.097647Z\u001b[0m \u001b[32m INFO\u001b[0m backend::api: request completed",
    );
    expect(record?.target).toBe("backend::api");
  });

  it("ignores non-tracing lines", () => {
    expect(parseTracing("connection established\nplain output")).toEqual([]);
  });
});
