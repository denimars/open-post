import { Trash2 } from "lucide-react";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import type { KeyValue } from "@/types";

interface KeyValueTableProps {
  rows: KeyValue[];
  onChange: (rows: KeyValue[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
  /** Label used by screen readers and tests. */
  label: string;
}

/**
 * Editable key/value grid with an always-present blank row at the bottom:
 * typing into it appends a new row, the way Postman behaves.
 */
export function KeyValueTable({
  rows,
  onChange,
  keyPlaceholder = "Key",
  valuePlaceholder = "Value",
  label,
}: KeyValueTableProps) {
  const display = [...rows, { key: "", value: "", enabled: true }];

  function update(index: number, patch: Partial<KeyValue>) {
    const next = display.map((row, i) => (i === index ? { ...row, ...patch } : row));
    // Drop the trailing blank row unless the user typed into it.
    const last = next[next.length - 1];
    if (last && last.key === "" && last.value === "") next.pop();
    onChange(next);
  }

  function remove(index: number) {
    onChange(rows.filter((_, i) => i !== index));
  }

  return (
    <table className="w-full text-sm" aria-label={label}>
      <thead>
        <tr className="border-b border-border text-left text-xs text-muted-foreground">
          <th className="w-9 py-1.5 font-medium" />
          <th className="py-1.5 font-medium">Key</th>
          <th className="py-1.5 font-medium">Value</th>
          <th className="w-9 py-1.5 font-medium" />
        </tr>
      </thead>
      <tbody>
        {display.map((row, index) => {
          const isBlankRow = index === display.length - 1;
          return (
            <tr key={index} className="border-b border-border/50">
              <td className="py-1 pr-2">
                <input
                  type="checkbox"
                  aria-label={`Enable ${row.key || "row"}`}
                  checked={row.enabled}
                  disabled={isBlankRow}
                  onChange={(e) => update(index, { enabled: e.target.checked })}
                  className="h-3.5 w-3.5 accent-primary disabled:opacity-30"
                />
              </td>
              <td className="py-1 pr-2">
                <Input
                  aria-label={`${label} key ${index + 1}`}
                  value={row.key}
                  placeholder={keyPlaceholder}
                  onChange={(e) => update(index, { key: e.target.value })}
                  className="h-7 border-transparent bg-transparent px-1.5 hover:border-border"
                />
              </td>
              <td className="py-1 pr-2">
                <Input
                  aria-label={`${label} value ${index + 1}`}
                  value={row.value}
                  placeholder={valuePlaceholder}
                  onChange={(e) => update(index, { value: e.target.value })}
                  className="h-7 border-transparent bg-transparent px-1.5 font-mono text-xs hover:border-border"
                />
              </td>
              <td className="py-1">
                {!isBlankRow && (
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`Remove ${row.key || `row ${index + 1}`}`}
                    onClick={() => remove(index)}
                  >
                    <Trash2 className="h-3.5 w-3.5 text-muted-foreground" />
                  </Button>
                )}
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
