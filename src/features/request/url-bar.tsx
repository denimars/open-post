import { Send, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { HTTP_METHODS, type HttpMethod } from "@/types";
import { cn } from "@/lib/utils";

const METHOD_TONE: Record<HttpMethod, string> = {
  GET: "text-emerald-500",
  POST: "text-amber-500",
  PUT: "text-sky-500",
  PATCH: "text-violet-500",
  DELETE: "text-red-500",
  HEAD: "text-muted-foreground",
  OPTIONS: "text-muted-foreground",
};

interface UrlBarProps {
  method: HttpMethod;
  url: string;
  isSending: boolean;
  onMethodChange: (method: HttpMethod) => void;
  onUrlChange: (url: string) => void;
  onSend: () => void;
  onCancel: () => void;
  onSave: () => void;
}

export function UrlBar({
  method,
  url,
  isSending,
  onMethodChange,
  onUrlChange,
  onSend,
  onCancel,
  onSave,
}: UrlBarProps) {
  return (
    <div className="flex items-center gap-2 border-b border-border p-3">
      <Select
        aria-label="HTTP method"
        value={method}
        onChange={(e) => onMethodChange(e.target.value as HttpMethod)}
        className={cn("w-28 font-semibold", METHOD_TONE[method])}
      >
        {HTTP_METHODS.map((m) => (
          <option key={m} value={m}>
            {m}
          </option>
        ))}
      </Select>

      <Input
        aria-label="Request URL"
        value={url}
        placeholder="https://api.example.com/users  or  {{base_url}}/users"
        spellCheck={false}
        onChange={(e) => onUrlChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && !isSending) onSend();
        }}
        className="flex-1 font-mono text-xs"
      />

      {isSending ? (
        <Button variant="destructive" onClick={onCancel}>
          <X className="h-4 w-4" />
          Cancel
        </Button>
      ) : (
        <Button onClick={onSend} disabled={url.trim() === ""}>
          <Send className="h-4 w-4" />
          Send
        </Button>
      )}

      <Button variant="outline" onClick={onSave} title="Save to a collection (Ctrl/Cmd+S)">
        Save
      </Button>
    </div>
  );
}
