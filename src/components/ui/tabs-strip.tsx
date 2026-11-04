import { cn } from "@/lib/utils";

export interface StripItem {
  id: string;
  label: string;
  badge?: string | number;
}

interface TabsStripProps {
  items: StripItem[];
  activeId: string;
  onSelect: (id: string) => void;
  className?: string;
}

/** A simple underlined tab strip used inside the request and response panes. */
export function TabsStrip({ items, activeId, onSelect, className }: TabsStripProps) {
  return (
    <div className={cn("flex items-center gap-1 border-b border-border", className)} role="tablist">
      {items.map((item) => (
        <button
          key={item.id}
          role="tab"
          aria-selected={item.id === activeId}
          onClick={() => onSelect(item.id)}
          className={cn(
            "-mb-px border-b-2 px-3 py-2 text-xs font-medium transition-colors",
            item.id === activeId
              ? "border-primary text-foreground"
              : "border-transparent text-muted-foreground hover:text-foreground",
          )}
        >
          {item.label}
          {item.badge !== undefined && item.badge !== 0 && (
            <span className="ml-1.5 rounded bg-muted px-1.5 py-0.5 text-[10px] text-muted-foreground">
              {item.badge}
            </span>
          )}
        </button>
      ))}
    </div>
  );
}
