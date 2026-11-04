import { useState } from "react";
import { TabsStrip } from "@/components/ui/tabs-strip";
import { CollectionsTree } from "@/features/collections/collections-tree";
import { HistoryList } from "@/features/history/history-list";

export function Sidebar() {
  const [view, setView] = useState<"collections" | "history">("collections");

  return (
    <div className="flex h-full flex-col border-r border-border">
      <TabsStrip
        className="px-2"
        activeId={view}
        onSelect={(id) => setView(id as "collections" | "history")}
        items={[
          { id: "collections", label: "Collections" },
          { id: "history", label: "History" },
        ]}
      />
      <div className="min-h-0 flex-1 overflow-hidden">
        {view === "collections" ? <CollectionsTree /> : <HistoryList />}
      </div>
    </div>
  );
}
