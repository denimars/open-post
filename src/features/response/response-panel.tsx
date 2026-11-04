import { useMemo, useState } from "react";
import { AlertTriangle, Loader2 } from "lucide-react";
import { TabsStrip } from "@/components/ui/tabs-strip";
import { EmptyState } from "@/components/ui/empty-state";
import { CodeEditor } from "@/features/request/code-editor";
import { formatBytes, formatDuration, statusTone } from "@/lib/utils";
import type { AppError, HttpResponse } from "@/types";

type View = "pretty" | "raw" | "headers";

interface ResponsePanelProps {
  response: HttpResponse | null;
  error: AppError | null;
  isSending: boolean;
}

function contentTypeOf(response: HttpResponse): string {
  const header = response.headers.find(([name]) => name.toLowerCase() === "content-type");
  return header?.[1] ?? "";
}

function languageOf(response: HttpResponse): "json" | "xml" | "text" {
  const type = contentTypeOf(response);
  if (type.includes("json")) return "json";
  if (type.includes("xml")) return "xml";
  return "text";
}

/** Formats JSON for the Pretty view; falls back to the raw body. */
function prettify(body: string, language: string): string {
  if (language !== "json") return body;
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

export function ResponsePanel({ response, error, isSending }: ResponsePanelProps) {
  const [view, setView] = useState<View>("pretty");

  const language = response ? languageOf(response) : "text";
  const pretty = useMemo(
    () => (response && !response.isBinary ? prettify(response.body, language) : ""),
    [response, language],
  );

  if (isSending) {
    return (
      <EmptyState
        icon={<Loader2 className="h-5 w-5 animate-spin" />}
        title="Sending…"
        hint="Press Cancel to stop the request."
      />
    );
  }

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-2 p-8 text-center">
        <AlertTriangle className="h-5 w-5 text-red-500" />
        <p className="text-sm font-medium text-red-500">{errorTitle(error)}</p>
        <p className="max-w-md text-xs text-muted-foreground">{error.message}</p>
      </div>
    );
  }

  if (!response) {
    return (
      <EmptyState
        title="No response yet"
        hint="Send the request to see the status, body and headers here."
      />
    );
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-4 border-b border-border px-3 py-2 text-xs">
        <span className={statusTone(response.status)}>
          <span className="font-semibold">{response.status}</span> {response.statusText}
        </span>
        <span className="text-muted-foreground">{formatDuration(response.durationMs)}</span>
        <span className="text-muted-foreground">{formatBytes(response.sizeBytes)}</span>
        {response.unresolvedVariables.length > 0 && (
          <span className="flex items-center gap-1 text-amber-500">
            <AlertTriangle className="h-3.5 w-3.5" />
            Unresolved: {response.unresolvedVariables.join(", ")}
          </span>
        )}
      </div>

      {response.isTruncated && (
        <p className="border-b border-border bg-amber-500/10 px-3 py-1.5 text-xs text-amber-600 dark:text-amber-400">
          The response is larger than 10 MB and was truncated for display.
        </p>
      )}

      <TabsStrip
        className="px-3"
        activeId={view}
        onSelect={(id) => setView(id as View)}
        items={[
          { id: "pretty", label: "Pretty" },
          { id: "raw", label: "Raw" },
          { id: "headers", label: "Headers", badge: response.headers.length },
        ]}
      />

      <div className="min-h-0 flex-1 overflow-auto">
        {response.isBinary ? (
          <p className="p-3 text-xs text-muted-foreground">
            The response is binary ({formatBytes(response.sizeBytes)}) and is shown base64-encoded
            in the Raw tab.
          </p>
        ) : null}

        {view === "pretty" && !response.isBinary && (
          <CodeEditor
            ariaLabel="Response body"
            value={pretty}
            language={language}
            readOnly
            className="h-full"
          />
        )}

        {view === "raw" && (
          <CodeEditor
            ariaLabel="Raw response body"
            value={response.body}
            readOnly
            className="h-full"
          />
        )}

        {view === "headers" && (
          <table className="w-full text-xs">
            <tbody>
              {response.headers.map(([name, value], index) => (
                <tr key={`${name}-${index}`} className="border-b border-border/50">
                  <td className="w-1/3 px-3 py-1.5 font-medium text-muted-foreground">{name}</td>
                  <td className="break-all px-3 py-1.5 font-mono">{value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}

function errorTitle(error: AppError): string {
  switch (error.kind) {
    case "timeout":
      return "The request timed out";
    case "dns":
      return "Host not found";
    case "connect":
      return "Could not connect";
    case "tls":
      return "TLS error";
    case "cancelled":
      return "Request cancelled";
    case "invalidUrl":
      return "Invalid URL";
    default:
      return "Request failed";
  }
}
