import { describe, expect, it } from "vitest";
import { mergeParams, paramsFromUrl, splitUrl, urlWithParams } from "@/lib/url";
import type { KeyValue } from "@/types";

const on = (key: string, value: string): KeyValue => ({ key, value, enabled: true });
const off = (key: string, value: string): KeyValue => ({ key, value, enabled: false });

describe("splitUrl", () => {
  it("splits on the first question mark", () => {
    expect(splitUrl("https://a.test/x?a=1?b=2")).toEqual({
      base: "https://a.test/x",
      query: "a=1?b=2",
    });
  });

  it("returns an empty query when there is none", () => {
    expect(splitUrl("https://a.test/x")).toEqual({ base: "https://a.test/x", query: "" });
  });
});

describe("paramsFromUrl", () => {
  it("returns nothing for a URL without a query", () => {
    expect(paramsFromUrl("https://a.test/users")).toEqual([]);
  });

  it("parses pairs into enabled rows", () => {
    expect(paramsFromUrl("https://a.test/users?page=2&limit=10")).toEqual([
      on("page", "2"),
      on("limit", "10"),
    ]);
  });

  it("handles a key with no value", () => {
    expect(paramsFromUrl("https://a.test?flag")).toEqual([on("flag", "")]);
  });

  it("decodes percent-encoded keys and values", () => {
    expect(paramsFromUrl("https://a.test?q=a%20b&t%26g=1")).toEqual([
      on("q", "a b"),
      on("t&g", "1"),
    ]);
  });

  it("decodes + as a space", () => {
    expect(paramsFromUrl("https://a.test?q=a+b")).toEqual([on("q", "a b")]);
  });

  it("leaves a malformed escape untouched instead of throwing", () => {
    expect(paramsFromUrl("https://a.test?q=%E0%A4%A")).toEqual([on("q", "%E0%A4%A")]);
  });

  it("ignores empty segments", () => {
    expect(paramsFromUrl("https://a.test?&a=1&")).toEqual([on("a", "1")]);
  });
});

describe("urlWithParams", () => {
  it("appends enabled params", () => {
    expect(urlWithParams("https://a.test/users", [on("page", "2")])).toBe(
      "https://a.test/users?page=2",
    );
  });

  it("replaces the existing query string", () => {
    expect(urlWithParams("https://a.test/users?old=1", [on("new", "2")])).toBe(
      "https://a.test/users?new=2",
    );
  });

  it("drops the query string when nothing is enabled", () => {
    expect(urlWithParams("https://a.test/users?a=1", [off("a", "1")])).toBe(
      "https://a.test/users",
    );
  });

  it("skips rows with a blank key", () => {
    expect(urlWithParams("https://a.test", [on("", "1"), on("a", "2")])).toBe(
      "https://a.test?a=2",
    );
  });

  it("percent-encodes keys and values", () => {
    expect(urlWithParams("https://a.test", [on("a b", "c&d")])).toBe("https://a.test?a%20b=c%26d");
  });

  it("keeps {{variables}} readable", () => {
    expect(urlWithParams("https://a.test", [on("token", "{{token}}")])).toBe(
      "https://a.test?token={{token}}",
    );
  });
});

describe("round trips", () => {
  it("url -> params -> url is stable", () => {
    const url = "https://a.test/users?page=2&q=a%20b";
    expect(urlWithParams(url, paramsFromUrl(url))).toBe(url);
  });
});

describe("mergeParams", () => {
  it("keeps disabled rows that are not in the URL", () => {
    const existing = [on("page", "1"), off("debug", "true")];
    expect(mergeParams(existing, paramsFromUrl("https://a.test?page=2"))).toEqual([
      on("page", "2"),
      off("debug", "true"),
    ]);
  });

  it("drops enabled rows the user removed from the URL", () => {
    const existing = [on("page", "1"), on("limit", "10")];
    expect(mergeParams(existing, paramsFromUrl("https://a.test?page=1"))).toEqual([
      on("page", "1"),
    ]);
  });
});
