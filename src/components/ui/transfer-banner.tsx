import { AlertTriangle, Check, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export interface TransferResult {
  tone: "success" | "error";
  title: string;
  /** A file path, or the reason an import failed. */
  detail?: string;
  /** Things that could not be mapped exactly; the import still succeeded. */
  warnings?: string[];
}

interface TransferBannerProps {
  result: TransferResult;
  onDismiss: () => void;
}

/** Outcome of an import or export, shown until the user dismisses it. */
export function TransferBanner({ result, onDismiss }: TransferBannerProps) {
  const failed = result.tone === "error";
  const warnings = result.warnings ?? [];

  return (
    <div
      role="status"
      className={cn(
        "border-b px-2 py-1.5 text-xs",
        failed
          ? "border-red-500/30 bg-red-500/10 text-red-700 dark:text-red-300"
          : "border-border bg-muted/60",
      )}
    >
      <div className="flex items-start gap-1.5">
        {failed ? (
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
        ) : (
          <Check className="mt-0.5 h-3.5 w-3.5 shrink-0 text-emerald-600 dark:text-emerald-400" />
        )}
        <div className="min-w-0 flex-1">
          <p className="font-medium">{result.title}</p>
          {result.detail && (
            <p className="mt-0.5 break-all text-muted-foreground">{result.detail}</p>
          )}
          {warnings.length > 0 && (
            <ul className="mt-1 space-y-0.5 text-muted-foreground">
              {warnings.map((warning) => (
                <li key={warning} className="flex gap-1">
                  <span aria-hidden="true">•</span>
                  <span className="min-w-0">{warning}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <Button
          variant="ghost"
          size="icon"
          aria-label="Dismiss"
          onClick={onDismiss}
          className="h-5 w-5 shrink-0"
        >
          <X className="h-3 w-3" />
        </Button>
      </div>
    </div>
  );
}
