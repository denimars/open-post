import { describe, expect, it } from "vitest";
import {
  activeVariables,
  missingVariables,
  previewResolve,
  referencedVariables,
} from "@/lib/variables";
import type { KeyValue } from "@/types";

const vars: KeyValue[] = [
  { key: "base_url", value: "https://api.test", enabled: true },
  { key: "token", value: "abc", enabled: true },
  { key: "off", value: "x", enabled: false },
];

describe("referencedVariables", () => {
  it("finds every name once, in order", () => {
    expect(referencedVariables("{{a}}/{{b}}/{{a}}")).toEqual(["a", "b"]);
  });

  it("trims whitespace inside the braces", () => {
    expect(referencedVariables("{{  spaced  }}")).toEqual(["spaced"]);
  });

  it("returns nothing when there are no placeholders", () => {
    expect(referencedVariables("https://a.test")).toEqual([]);
  });
});

describe("previewResolve", () => {
  it("substitutes known variables", () => {
    expect(previewResolve("{{base_url}}/users", vars)).toBe("https://api.test/users");
  });

  it("leaves unknown variables in place", () => {
    expect(previewResolve("{{nope}}/users", vars)).toBe("{{nope}}/users");
  });
});

describe("activeVariables", () => {
  it("drops disabled and blank rows", () => {
    const env = {
      id: "e1",
      name: "Test",
      variables: [...vars, { key: "  ", value: "y", enabled: true }],
    };
    expect(activeVariables(env).map((v) => v.key)).toEqual(["base_url", "token"]);
  });

  it("returns nothing without an environment", () => {
    expect(activeVariables(null)).toEqual([]);
  });
});

describe("missingVariables", () => {
  it("lists only the undefined names", () => {
    expect(missingVariables("{{base_url}}/{{missing}}", vars)).toEqual(["missing"]);
  });
});
