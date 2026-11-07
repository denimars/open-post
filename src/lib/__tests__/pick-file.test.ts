import { describe, expect, it, vi } from "vitest";
import { pickJsonFile } from "@/lib/pick-file";

/**
 * Hands back the `<input type="file">` that `pickJsonFile` creates, so the test
 * can drive it the way the platform dialog would.
 */
function captureInput(): Promise<HTMLInputElement> {
  return new Promise((resolve) => {
    const create = document.createElement.bind(document);
    vi.spyOn(document, "createElement").mockImplementation((tag: string) => {
      const element = create(tag);
      if (tag === "input") {
        // `click()` would try to open a real dialog; resolve instead.
        element.click = () => resolve(element as HTMLInputElement);
      }
      return element;
    });
  });
}

/** jsdom has no file dialog, so the selection is assigned directly. */
function select(input: HTMLInputElement, files: File[]) {
  Object.defineProperty(input, "files", { value: files, configurable: true });
  input.dispatchEvent(new Event("change"));
}

describe("pickJsonFile", () => {
  it("resolves with the name and text of the chosen file", async () => {
    const captured = captureInput();
    const picked = pickJsonFile();

    const input = await captured;
    expect(input.accept).toContain(".json");
    select(input, [new File(['{"info":{}}'], "api.postman_collection.json")]);

    await expect(picked).resolves.toEqual({
      name: "api.postman_collection.json",
      text: '{"info":{}}',
    });
  });

  it("resolves with null when the dialog is cancelled", async () => {
    const captured = captureInput();
    const picked = pickJsonFile();

    (await captured).dispatchEvent(new Event("cancel"));

    await expect(picked).resolves.toBeNull();
  });

  it("resolves with null when the change event carries no file", async () => {
    const captured = captureInput();
    const picked = pickJsonFile();

    select(await captured, []);

    await expect(picked).resolves.toBeNull();
  });

  it("removes the input from the document afterwards", async () => {
    const captured = captureInput();
    const picked = pickJsonFile();

    const input = await captured;
    expect(input.isConnected).toBe(true);
    select(input, [new File(["{}"], "a.json")]);
    await picked;

    expect(input.isConnected).toBe(false);
  });
});
