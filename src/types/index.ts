/**
 * TypeScript mirrors of the structs in `src-tauri/src/models.rs`.
 * Change both sides in the same commit.
 */

export const HTTP_METHODS = [
  "GET",
  "POST",
  "PUT",
  "PATCH",
  "DELETE",
  "HEAD",
  "OPTIONS",
] as const;

export type HttpMethod = (typeof HTTP_METHODS)[number];

export interface KeyValue {
  key: string;
  value: string;
  enabled: boolean;
}

export type RawContentType = "json" | "text" | "xml";

export type RequestBody =
  | { type: "none" }
  | { type: "raw"; content: string; contentType: RawContentType }
  | { type: "urlEncoded"; fields: KeyValue[] }
  | { type: "formData"; fields: KeyValue[] };

export type Auth =
  | { type: "none" }
  | { type: "bearer"; token: string }
  | { type: "basic"; username: string; password: string };

export interface HttpRequest {
  id: string;
  method: HttpMethod;
  url: string;
  params: KeyValue[];
  headers: KeyValue[];
  body: RequestBody;
  auth: Auth;
  timeoutMs: number | null;
  name: string | null;
}

export interface HttpResponse {
  status: number;
  statusText: string;
  headers: [string, string][];
  body: string;
  isBinary: boolean;
  isTruncated: boolean;
  sizeBytes: number;
  durationMs: number;
  unresolvedVariables: string[];
}

export type AppErrorKind =
  | "invalidUrl"
  | "invalidRequest"
  | "dns"
  | "connect"
  | "timeout"
  | "tls"
  | "cancelled"
  | "request"
  | "storage"
  | "notFound"
  | "internal";

export interface AppError {
  kind: AppErrorKind;
  message: string;
}

export interface Environment {
  id: string;
  name: string;
  variables: KeyValue[];
}

export interface SavedRequest {
  id: string;
  collectionId: string;
  name: string;
  request: HttpRequest;
}

export interface Collection {
  id: string;
  name: string;
  requests: SavedRequest[];
}

export interface HistoryEntry {
  id: string;
  sentAt: number;
  method: HttpMethod;
  url: string;
  status: number | null;
  durationMs: number | null;
  request: HttpRequest;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "message" in value &&
    typeof (value as AppError).message === "string"
  );
}

/** Turns anything thrown by `invoke` into a displayable AppError. */
export function toAppError(value: unknown): AppError {
  if (isAppError(value)) return value;
  if (value instanceof Error) return { kind: "internal", message: value.message };
  return { kind: "internal", message: String(value) };
}

// ----------------------------------------------- Postman import / export

/** An export file's contents plus the file name Postman would use. */
export interface ExportFile {
  fileName: string;
  contents: string;
}

/** Where an export was written. */
export interface SavedExport {
  path: string;
  fileName: string;
  bytes: number;
}

export interface CollectionImportReport {
  collection: Collection;
  importedRequests: number;
  /** Everything that could not be mapped exactly, in plain language. */
  warnings: string[];
  /** Set when the file carried collection variables and they became an environment. */
  createdEnvironment: Environment | null;
}

export interface EnvironmentImportReport {
  environment: Environment;
  warnings: string[];
}
