import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { EmptyState } from "@/components/ui/empty-state";
import { clearHistory, listHistory } from "@/lib/tauri";
import { statusTone } from "@/lib/utils";
import { useTabsStore } from "@/stores/tabs";
import type { HistoryEntry } from "@/types";

function timeLabel(sentAt: number): string {
  return new Date(sentAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function HistoryList() {
  const queryClient = useQueryClient();
  const openTab = useTabsStore((state) => state.openTab);

  const { data: entries = [], isLoading } = useQuery({
    queryKey: ["history"],
    queryFn: () => listHistory(100),
  });

  const clear = useMutation({
    mutationFn: clearHistory,
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["history"] }),
  });

  if (isLoading) return <p className="p-3 text-xs text-muted-foreground">Loading…</p>;

  if (entries.length === 0) {
    return <EmptyState title="No history yet" hint="Requests you send show up here." />;
  }

  return (
    <div className="flex h-full flex-col">
      <div className="flex justify-end border-b border-border px-2 py-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => clear.mutate()}
          disabled={clear.isPending}
        >
          <Trash2 className="h-3.5 w-3.5" />
          Clear
        </Button>
      </div>
      <ul className="min-h-0 flex-1 overflow-auto">
        {entries.map((entry: HistoryEntry) => (
          <li key={entry.id}>
            <button
              onClick={() => openTab({ ...entry.request, id: crypto.randomUUID() })}
              className="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-muted"
            >
              <span className="w-12 shrink-0 font-semibold text-muted-foreground">
                {entry.method}
              </span>
              <span className="min-w-0 flex-1 truncate font-mono">{entry.url}</span>
              {entry.status !== null && (
                <span className={statusTone(entry.status)}>{entry.status}</span>
              )}
              <span className="shrink-0 text-muted-foreground">{timeLabel(entry.sentAt)}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
