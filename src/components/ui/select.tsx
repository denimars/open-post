import { forwardRef, type SelectHTMLAttributes } from "react";
import { cn } from "@/lib/utils";

/**
 * A native `<select>`, themed for light and dark mode.
 *
 * `text-foreground` is not redundant: a native select does not inherit `color`
 * from its parent, it falls back to the user agent's own near-black, which is
 * invisible on a dark background. The dropdown list itself is drawn by the
 * platform, which picks light or dark from the `color-scheme` set in
 * `index.css`.
 *
 * Callers may override the text colour (the method picker colours GET, POST, …);
 * `cn` merges with the last class winning.
 */
export const Select = forwardRef<HTMLSelectElement, SelectHTMLAttributes<HTMLSelectElement>>(
  ({ className, ...props }, ref) => (
    <select
      ref={ref}
      className={cn(
        "h-9 cursor-pointer rounded-md border border-border bg-background px-2 text-sm text-foreground",
        "hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        "disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      {...props}
    />
  ),
);
Select.displayName = "Select";
