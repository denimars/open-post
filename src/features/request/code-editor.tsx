import CodeMirror from "@uiw/react-codemirror";
import { json } from "@codemirror/lang-json";
import { xml } from "@codemirror/lang-xml";
import { githubDark, githubLight } from "@uiw/codemirror-theme-github";
import { useMemo } from "react";
import type { Extension } from "@codemirror/state";
import { useTheme } from "@/app/theme-context";

interface CodeEditorProps {
  value: string;
  onChange?: (value: string) => void;
  language?: "json" | "xml" | "text";
  readOnly?: boolean;
  placeholder?: string;
  className?: string;
  ariaLabel: string;
}

export function CodeEditor({
  value,
  onChange,
  language = "text",
  readOnly = false,
  placeholder,
  className,
  ariaLabel,
}: CodeEditorProps) {
  const { resolved } = useTheme();

  const extensions = useMemo<Extension[]>(() => {
    if (language === "json") return [json()];
    if (language === "xml") return [xml()];
    return [];
  }, [language]);

  return (
    <div className={className} aria-label={ariaLabel}>
      <CodeMirror
        value={value}
        onChange={onChange}
        readOnly={readOnly}
        placeholder={placeholder}
        extensions={extensions}
        theme={resolved === "dark" ? githubDark : githubLight}
        height="100%"
        style={{ height: "100%", fontSize: "12px" }}
        basicSetup={{
          lineNumbers: true,
          foldGutter: true,
          highlightActiveLine: !readOnly,
          autocompletion: false,
        }}
      />
    </div>
  );
}
