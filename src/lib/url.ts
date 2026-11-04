/**
 * Two-way sync between the URL bar and the Params table.
 *
 * The URL is the source of truth for what gets sent; the table is a view of
 * its query string plus any disabled rows the user kept around.
 */
import type { KeyValue } from "@/types";

/** Splits a raw URL into everything before `?` and the query string. */
export function splitUrl(url: string): { base: string; query: string } {
  const index = url.indexOf("?");
  if (index === -1) return { base: url, query: "" };
  return { base: url.slice(0, index), query: url.slice(index + 1) };
}

/**
 * Parses the query string of `url` into rows. Every parsed row is enabled;
 * a row the user disabled is not present in the URL by definition.
 */
export function paramsFromUrl(url: string): KeyValue[] {
  const { query } = splitUrl(url);
  if (!query) return [];

  return query
    .split("&")
    .filter((pair) => pair.length > 0)
    .map((pair) => {
      const eq = pair.indexOf("=");
      const rawKey = eq === -1 ? pair : pair.slice(0, eq);
      const rawValue = eq === -1 ? "" : pair.slice(eq + 1);
      return {
        key: safeDecode(rawKey),
        value: safeDecode(rawValue),
        enabled: true,
      };
    });
}

/**
 * Rebuilds `url` so its query string contains exactly the enabled `params`.
 * Rows with an empty key are skipped; disabled rows are dropped from the URL
 * but the caller keeps them in the table.
 */
export function urlWithParams(url: string, params: KeyValue[]): string {
  const { base } = splitUrl(url);
  const query = params
    .filter((p) => p.enabled && p.key.trim() !== "")
    .map((p) => `${encodeComponent(p.key)}=${encodeComponent(p.value)}`)
    .join("&");
  return query ? `${base}?${query}` : base;
}

/**
 * Merges the params parsed out of a newly typed URL with the existing table,
 * preserving disabled rows and their order where possible.
 */
export function mergeParams(existing: KeyValue[], fromUrl: KeyValue[]): KeyValue[] {
  const disabled = existing.filter((p) => !p.enabled);
  return [...fromUrl, ...disabled];
}

/** `{{var}}` placeholders must survive percent-encoding to stay readable. */
function encodeComponent(value: string): string {
  return encodeURIComponent(value).replace(/%7B%7B(.*?)%7D%7D/g, "{{$1}}");
}

function safeDecode(value: string): string {
  try {
    return decodeURIComponent(value.replace(/\+/g, " "));
  } catch {
    return value;
  }
}
