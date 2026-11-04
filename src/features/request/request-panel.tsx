import { useState } from "react";
import { TabsStrip } from "@/components/ui/tabs-strip";
import { KeyValueTable } from "./key-value-table";
import { AuthPanel } from "./auth-panel";
import { BodyEditor } from "./body-editor";
import { UrlBar } from "./url-bar";
import { useTabsStore, type Tab } from "@/stores/tabs";
import type { HttpMethod } from "@/types";

type Section = "params" | "headers" | "body" | "auth";

interface RequestPanelProps {
  tab: Tab;
  onSend: () => void;
  onCancel: () => void;
  onSave: () => void;
}

function bodyCount(tab: Tab): number {
  const body = tab.request.body;
  if (body.type === "urlEncoded" || body.type === "formData") {
    return body.fields.filter((f) => f.enabled && f.key.trim() !== "").length;
  }
  return body.type === "raw" && body.content.trim() !== "" ? 1 : 0;
}

export function RequestPanel({ tab, onSend, onCancel, onSave }: RequestPanelProps) {
  const [section, setSection] = useState<Section>("params");
  const { updateRequest, setUrl, setParams } = useTabsStore();

  const enabledParams = tab.request.params.filter((p) => p.enabled && p.key.trim() !== "").length;
  const enabledHeaders = tab.request.headers.filter(
    (h) => h.enabled && h.key.trim() !== "",
  ).length;

  return (
    <div className="flex h-full flex-col">
      <UrlBar
        method={tab.request.method}
        url={tab.request.url}
        isSending={tab.isSending}
        onMethodChange={(method: HttpMethod) => updateRequest(tab.id, { method })}
        onUrlChange={(url) => setUrl(tab.id, url)}
        onSend={onSend}
        onCancel={onCancel}
        onSave={onSave}
      />

      <TabsStrip
        className="px-3"
        activeId={section}
        onSelect={(id) => setSection(id as Section)}
        items={[
          { id: "params", label: "Params", badge: enabledParams },
          { id: "headers", label: "Headers", badge: enabledHeaders },
          { id: "body", label: "Body", badge: bodyCount(tab) },
          { id: "auth", label: "Auth", badge: tab.request.auth.type === "none" ? 0 : 1 },
        ]}
      />

      <div className="min-h-0 flex-1 overflow-auto">
        {section === "params" && (
          <div className="p-3">
            <KeyValueTable
              label="Query params"
              rows={tab.request.params}
              onChange={(params) => setParams(tab.id, params)}
            />
          </div>
        )}

        {section === "headers" && (
          <div className="p-3">
            <KeyValueTable
              label="Headers"
              rows={tab.request.headers}
              keyPlaceholder="Content-Type"
              valuePlaceholder="application/json"
              onChange={(headers) => updateRequest(tab.id, { headers })}
            />
          </div>
        )}

        {section === "body" && (
          <BodyEditor
            body={tab.request.body}
            onChange={(body) => updateRequest(tab.id, { body })}
          />
        )}

        {section === "auth" && (
          <AuthPanel
            auth={tab.request.auth}
            onChange={(auth) => updateRequest(tab.id, { auth })}
          />
        )}
      </div>
    </div>
  );
}
