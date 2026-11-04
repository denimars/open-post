/**
 * Display-side helpers for `{{name}}` placeholders.
 * The real substitution happens in Rust before the request is sent.
 */
import type { Environment, KeyValue } from "@/types";

const PLACEHOLDER = /\{\{\s*([^{}]+?)\s*\}\}/g;

/** Every variable name referenced in `text`, in order, without duplicates. */
export function referencedVariables(text: string): string[] {
  const found = new Set<string>();
  for (const match of text.matchAll(PLACEHOLDER)) {
    const name = match[1]?.trim();
    if (name) found.add(name);
  }
  return [...found];
}

export function activeVariables(environment: Environment | null): KeyValue[] {
  if (!environment) return [];
  return environment.variables.filter((v) => v.enabled && v.key.trim() !== "");
}

/**
 * Preview substitution for tooltips and the URL bar hint — never used to
 * build the request that is actually sent.
 */
export function previewResolve(text: string, variables: KeyValue[]): string {
  const lookup = new Map(variables.map((v) => [v.key.trim(), v.value]));
  return text.replace(PLACEHOLDER, (whole, rawName: string) => {
    const value = lookup.get(rawName.trim());
    return value === undefined ? whole : value;
  });
}

/** Names used in `text` that the active environment does not define. */
export function missingVariables(text: string, variables: KeyValue[]): string[] {
  const known = new Set(variables.map((v) => v.key.trim()));
  return referencedVariables(text).filter((name) => !known.has(name));
}
