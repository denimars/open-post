import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Plus, Trash2, Upload, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { TransferBanner, type TransferResult } from "@/components/ui/transfer-banner";
import { KeyValueTable } from "@/features/request/key-value-table";
import {
  deleteEnvironment,
  exportEnvironmentToFile,
  importEnvironment,
  listEnvironments,
  saveEnvironment,
} from "@/lib/tauri";
import { pickJsonFile } from "@/lib/pick-file";
import { cn } from "@/lib/utils";
import { toAppError, type Environment } from "@/types";

interface EnvironmentManagerProps {
  open: boolean;
  onClose: () => void;
}

export function EnvironmentManager({ open, onClose }: EnvironmentManagerProps) {
  const queryClient = useQueryClient();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [draft, setDraft] = useState<Environment | null>(null);
  const [result, setResult] = useState<TransferResult | null>(null);

  const { data: environments = [] } = useQuery({
    queryKey: ["environments"],
    queryFn: listEnvironments,
    enabled: open,
  });

  // Keep the editor pointed at a real environment as the list changes.
  useEffect(() => {
    if (!open) return;
    const current = environments.find((env: Environment) => env.id === selectedId);
    if (current) {
      setDraft((previous) => (previous?.id === current.id ? previous : { ...current }));
    } else {
      const first = environments[0] ?? null;
      setSelectedId(first?.id ?? null);
      setDraft(first ? { ...first } : null);
    }
  }, [environments, selectedId, open]);

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["environments"] });

  const save = useMutation({
    mutationFn: (env: Environment) => saveEnvironment(env),
    onSuccess: (saved) => {
      setSelectedId(saved.id);
      setDraft(saved);
      refresh();
    },
  });

  const remove = useMutation({
    mutationFn: deleteEnvironment,
    onSuccess: () => {
      setSelectedId(null);
      setDraft(null);
      refresh();
      queryClient.invalidateQueries({ queryKey: ["activeEnvironment"] });
    },
  });

  const importFile = useMutation({
    mutationFn: async () => {
      const picked = await pickJsonFile();
      if (!picked) return null;
      return importEnvironment(picked.text);
    },
    onSuccess: (report) => {
      if (!report) return;
      refresh();
      setSelectedId(report.environment.id);
      setDraft({ ...report.environment });
      setResult({
        tone: "success",
        title: `Imported ${report.environment.variables.length} variable(s) into ${report.environment.name}`,
        warnings: report.warnings,
      });
    },
    onError: (error) =>
      setResult({ tone: "error", title: "Import failed", detail: toAppError(error).message }),
  });

  const exportFile = useMutation({
    mutationFn: (environmentId: string) => exportEnvironmentToFile(environmentId),
    onSuccess: (saved) =>
      setResult({ tone: "success", title: `Exported ${saved.fileName}`, detail: saved.path }),
    onError: (error) =>
      setResult({ tone: "error", title: "Export failed", detail: toAppError(error).message }),
  });

  if (!open) return null;

  return (
    <div
      role="dialog"
      aria-label="Environments"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-8"
      onClick={onClose}
    >
      <div
        className="flex h-[32rem] w-[52rem] overflow-hidden rounded-lg border border-border bg-background shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <aside className="flex w-56 shrink-0 flex-col border-r border-border">
          <div className="flex items-center justify-between border-b border-border px-2 py-1.5">
            <span className="text-xs font-medium">Environments</span>
            <div className="flex items-center">
              <Button
                variant="ghost"
                size="icon"
                aria-label="Import a Postman environment"
                title="Import a Postman environment"
                disabled={importFile.isPending}
                onClick={() => importFile.mutate()}
              >
                <Upload className="h-3.5 w-3.5" />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                aria-label="New environment"
                title="New environment"
                onClick={() =>
                  save.mutate({ id: "", name: "New environment", variables: [] })
                }
              >
                <Plus className="h-3.5 w-3.5" />
              </Button>
            </div>
          </div>
          <ul className="min-h-0 flex-1 overflow-auto">
            {environments.map((env: Environment) => (
              <li key={env.id}>
                <button
                  onClick={() => {
                    setSelectedId(env.id);
                    setDraft({ ...env });
                  }}
                  className={cn(
                    "w-full truncate px-3 py-1.5 text-left text-xs hover:bg-muted",
                    env.id === selectedId && "bg-muted font-medium",
                  )}
                >
                  {env.name}
                </button>
              </li>
            ))}
          </ul>
        </aside>

        <section className="flex min-w-0 flex-1 flex-col">
          <header className="flex items-center justify-between border-b border-border px-3 py-2">
            <span className="text-sm font-medium">
              {draft ? draft.name : "No environment selected"}
            </span>
            <Button variant="ghost" size="icon" aria-label="Close" onClick={onClose}>
              <X className="h-4 w-4" />
            </Button>
          </header>

          {result && <TransferBanner result={result} onDismiss={() => setResult(null)} />}

          {draft ? (
            <>
              <div className="flex items-end gap-2 border-b border-border p-3">
                <label className="flex-1 space-y-1">
                  <span className="text-xs text-muted-foreground">Name</span>
                  <Input
                    aria-label="Environment name"
                    value={draft.name}
                    onChange={(e) => setDraft({ ...draft, name: e.target.value })}
                  />
                </label>
                <Button onClick={() => save.mutate(draft)} disabled={!draft.name.trim()}>
                  Save
                </Button>
                <Button
                  variant="outline"
                  aria-label="Export environment"
                  title="Export as a Postman environment"
                  disabled={exportFile.isPending}
                  onClick={() => exportFile.mutate(draft.id)}
                >
                  <Download className="h-3.5 w-3.5" />
                </Button>
                <Button
                  variant="outline"
                  aria-label="Delete environment"
                  onClick={() => remove.mutate(draft.id)}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </Button>
              </div>

              <div className="min-h-0 flex-1 overflow-auto p-3">
                <KeyValueTable
                  label="Variables"
                  rows={draft.variables}
                  keyPlaceholder="base_url"
                  valuePlaceholder="https://api.example.com"
                  onChange={(variables) => setDraft({ ...draft, variables })}
                />
                <p className="mt-3 text-xs text-muted-foreground">
                  Use a variable anywhere in a request as{" "}
                  <code className="rounded bg-muted px-1">{"{{name}}"}</code>.
                </p>
              </div>
            </>
          ) : (
            <p className="p-4 text-xs text-muted-foreground">
              Create an environment to define <code>{"{{variables}}"}</code>.
            </p>
          )}
        </section>
      </div>
    </div>
  );
}
