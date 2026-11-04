import { Select } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { KeyValueTable } from "./key-value-table";
import { CodeEditor } from "./code-editor";
import type { RawContentType, RequestBody } from "@/types";

interface BodyEditorProps {
  body: RequestBody;
  onChange: (body: RequestBody) => void;
}

function prettyJson(content: string): string | null {
  try {
    return JSON.stringify(JSON.parse(content), null, 2);
  } catch {
    return null;
  }
}

export function BodyEditor({ body, onChange }: BodyEditorProps) {
  function setType(type: RequestBody["type"]) {
    if (type === body.type) return;
    switch (type) {
      case "none":
        return onChange({ type: "none" });
      case "raw":
        return onChange({ type: "raw", content: "", contentType: "json" });
      case "urlEncoded":
        return onChange({ type: "urlEncoded", fields: [] });
      case "formData":
        return onChange({ type: "formData", fields: [] });
    }
  }

  const formatted = body.type === "raw" && body.contentType === "json"
    ? prettyJson(body.content)
    : null;

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center gap-2 border-b border-border px-3 py-2">
        <Select
          aria-label="Body type"
          value={body.type}
          onChange={(e) => setType(e.target.value as RequestBody["type"])}
          className="h-7 text-xs"
        >
          <option value="none">None</option>
          <option value="raw">Raw</option>
          <option value="urlEncoded">x-www-form-urlencoded</option>
          <option value="formData">form-data</option>
        </Select>

        {body.type === "raw" && (
          <>
            <Select
              aria-label="Raw content type"
              value={body.contentType}
              onChange={(e) =>
                onChange({ ...body, contentType: e.target.value as RawContentType })
              }
              className="h-7 text-xs"
            >
              <option value="json">JSON</option>
              <option value="text">Text</option>
              <option value="xml">XML</option>
            </Select>
            {body.contentType === "json" && (
              <Button
                variant="ghost"
                size="sm"
                disabled={formatted === null || formatted === body.content}
                onClick={() => formatted && onChange({ ...body, content: formatted })}
              >
                Beautify
              </Button>
            )}
          </>
        )}

        {body.type === "formData" && (
          <span className="text-xs text-muted-foreground">
            Text fields only — file upload is not supported yet.
          </span>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-auto">
        {body.type === "none" && (
          <p className="p-3 text-xs text-muted-foreground">This request has no body.</p>
        )}

        {body.type === "raw" && (
          <CodeEditor
            ariaLabel="Request body"
            value={body.content}
            language={body.contentType}
            placeholder={body.contentType === "json" ? '{\n  "key": "value"\n}' : ""}
            onChange={(content) => onChange({ ...body, content })}
            className="h-full"
          />
        )}

        {(body.type === "urlEncoded" || body.type === "formData") && (
          <div className="p-3">
            <KeyValueTable
              label="Body fields"
              rows={body.fields}
              onChange={(fields) => onChange({ ...body, fields })}
            />
          </div>
        )}
      </div>
    </div>
  );
}
