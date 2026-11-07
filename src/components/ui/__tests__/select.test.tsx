import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Select } from "@/components/ui/select";

/**
 * A native select does not inherit `color`, so losing `text-foreground` makes
 * the control invisible in dark mode. These assertions guard the merge rules
 * that keep it there.
 */
describe("Select", () => {
  it("sets an explicit text colour, since a native select does not inherit one", () => {
    render(
      <Select aria-label="Body type">
        <option value="none">None</option>
      </Select>,
    );
    expect(screen.getByLabelText("Body type")).toHaveClass("text-foreground");
  });

  it("keeps the text colour when the caller only overrides the font size", () => {
    render(
      <Select aria-label="Active environment" className="h-7 text-xs">
        <option value="">No environment</option>
      </Select>,
    );
    const select = screen.getByLabelText("Active environment");

    expect(select).toHaveClass("text-foreground");
    expect(select).toHaveClass("text-xs");
    expect(select).not.toHaveClass("text-sm");
    expect(select).not.toHaveClass("h-9");
  });

  it("lets the caller override the text colour, as the method picker does", () => {
    render(
      <Select aria-label="HTTP method" className="w-28 font-semibold text-emerald-500">
        <option value="GET">GET</option>
      </Select>,
    );
    const select = screen.getByLabelText("HTTP method");

    expect(select).toHaveClass("text-emerald-500");
    expect(select).not.toHaveClass("text-foreground");
  });
});
