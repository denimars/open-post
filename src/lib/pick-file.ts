/**
 * Opening a file from the webview.
 *
 * A hidden `<input type="file">` is enough: the webview shows the platform's
 * own file dialog and hands us the contents, so the app needs no filesystem
 * capability for an import. Export goes the other way, through Rust.
 */

export interface PickedFile {
  name: string;
  text: string;
}

/** Resolves with the chosen file's text, or `null` if the dialog was cancelled. */
export function pickJsonFile(): Promise<PickedFile | null> {
  return new Promise((resolve, reject) => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".json,application/json";
    input.hidden = true;
    document.body.appendChild(input);

    const finish = (value: PickedFile | null) => {
      input.remove();
      resolve(value);
    };

    input.addEventListener("cancel", () => finish(null));
    input.addEventListener("change", () => {
      const file = input.files?.[0];
      if (!file) {
        finish(null);
        return;
      }
      // `FileReader` rather than `file.text()`: it is available in every
      // webview version the app may run in, and in jsdom under test.
      const reader = new FileReader();
      reader.onload = () => finish({ name: file.name, text: String(reader.result ?? "") });
      reader.onerror = () => {
        input.remove();
        reject(reader.error ?? new Error(`could not read ${file.name}`));
      };
      reader.readAsText(file);
    });

    input.click();
  });
}
