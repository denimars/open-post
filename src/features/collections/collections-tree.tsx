import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, ChevronRight, Download, FolderPlus, Trash2, Upload } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { EmptyState } from "@/components/ui/empty-state";
import { TransferBanner, type TransferResult } from "@/components/ui/transfer-banner";
import {
  createCollection,
  deleteCollection,
  deleteRequest,
  exportCollectionToFile,
  importCollection,
  listCollections,
} from "@/lib/tauri";
import { pickJsonFile } from "@/lib/pick-file";
import { useTabsStore } from "@/stores/tabs";
import { toAppError, type Collection } from "@/types";

export function CollectionsTree() {
  const queryClient = useQueryClient();
  const openSavedRequest = useTabsStore((state) => state.openSavedRequest);
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [newName, setNewName] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [result, setResult] = useState<TransferResult | null>(null);

  const { data: collections = [], isLoading } = useQuery({
    queryKey: ["collections"],
    queryFn: listCollections,
  });

  const refresh = () => queryClient.invalidateQueries({ queryKey: ["collections"] });

  const create = useMutation({
    mutationFn: (name: string) => createCollection(name),
    onSuccess: () => {
      setNewName("");
      setIsCreating(false);
      refresh();
    },
  });
  const removeCollection = useMutation({ mutationFn: deleteCollection, onSuccess: refresh });
  const removeRequest = useMutation({ mutationFn: deleteRequest, onSuccess: refresh });

  // Reads a Postman collection the user picks and stores it as a new collection.
  const importFile = useMutation({
    mutationFn: async () => {
      const picked = await pickJsonFile();
      if (!picked) return null;
      return importCollection(picked.text);
    },
    onSuccess: (report) => {
      if (!report) return;
      refresh();
      if (report.createdEnvironment) {
        queryClient.invalidateQueries({ queryKey: ["environments"] });
      }
      setExpanded((previous) => ({ ...previous, [report.collection.id]: true }));
      setResult({
        tone: "success",
        title: `Imported ${report.importedRequests} request${
          report.importedRequests === 1 ? "" : "s"
        } into ${report.collection.name}`,
        warnings: report.warnings,
      });
    },
    onError: (error) =>
      setResult({
        tone: "error",
        title: "Import failed",
        detail: toAppError(error).message,
      }),
  });

  const exportFile = useMutation({
    mutationFn: (collectionId: string) => exportCollectionToFile(collectionId),
    onSuccess: (saved) =>
      setResult({ tone: "success", title: `Exported ${saved.fileName}`, detail: saved.path }),
    onError: (error) =>
      setResult({
        tone: "error",
        title: "Export failed",
        detail: toAppError(error).message,
      }),
  });

  if (isLoading) return <p className="p-3 text-xs text-muted-foreground">Loading…</p>;

  return (
    <div className="flex h-full flex-col">
      <div className="flex items-center justify-between border-b border-border px-2 py-1">
        <span className="px-1 text-xs font-medium text-muted-foreground">Collections</span>
        <div className="flex items-center">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Import a Postman collection"
            title="Import a Postman collection (v2.1)"
            disabled={importFile.isPending}
            onClick={() => importFile.mutate()}
          >
            <Upload className="h-3.5 w-3.5" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            aria-label="New collection"
            title="New collection"
            onClick={() => setIsCreating(true)}
          >
            <FolderPlus className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {result && <TransferBanner result={result} onDismiss={() => setResult(null)} />}

      {isCreating && (
        <form
          className="flex gap-1 p-2"
          onSubmit={(e) => {
            e.preventDefault();
            if (newName.trim()) create.mutate(newName.trim());
          }}
        >
          <Input
            autoFocus
            aria-label="Collection name"
            value={newName}
            placeholder="Collection name"
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => e.key === "Escape" && setIsCreating(false)}
            className="h-7 text-xs"
          />
          <Button type="submit" size="sm" disabled={!newName.trim()}>
            Add
          </Button>
        </form>
      )}

      {collections.length === 0 && !isCreating ? (
        <EmptyState
          title="No collections"
          hint="Group requests you want to keep into a collection, or import one from Postman."
        />
      ) : (
        <ul className="min-h-0 flex-1 overflow-auto">
          {collections.map((collection: Collection) => {
            const isOpen = expanded[collection.id] ?? true;
            return (
              <li key={collection.id}>
                <div className="group flex items-center gap-1 px-2 py-1 hover:bg-muted">
                  <button
                    aria-label={`${isOpen ? "Collapse" : "Expand"} ${collection.name}`}
                    onClick={() =>
                      setExpanded((prev) => ({ ...prev, [collection.id]: !isOpen }))
                    }
                    className="flex min-w-0 flex-1 items-center gap-1 text-left text-xs"
                  >
                    {isOpen ? (
                      <ChevronDown className="h-3.5 w-3.5 shrink-0" />
                    ) : (
                      <ChevronRight className="h-3.5 w-3.5 shrink-0" />
                    )}
                    <span className="truncate font-medium">{collection.name}</span>
                    <span className="shrink-0 text-muted-foreground">
                      {collection.requests.length}
                    </span>
                  </button>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`Export ${collection.name}`}
                    title="Export as a Postman collection"
                    disabled={exportFile.isPending}
                    onClick={() => exportFile.mutate(collection.id)}
                    className="opacity-0 group-hover:opacity-100"
                  >
                    <Download className="h-3.5 w-3.5" />
                  </Button>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={`Delete ${collection.name}`}
                    onClick={() => removeCollection.mutate(collection.id)}
                    className="opacity-0 group-hover:opacity-100"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>

                {isOpen && (
                  <ul>
                    {collection.requests.map((saved) => (
                      <li key={saved.id} className="group flex items-center hover:bg-muted">
                        <button
                          onClick={() =>
                            openSavedRequest(saved.request, {
                              collectionId: collection.id,
                              savedRequestId: saved.id,
                              name: saved.name,
                            })
                          }
                          className="flex min-w-0 flex-1 items-center gap-2 py-1 pl-7 pr-2 text-left text-xs"
                        >
                          <span className="w-10 shrink-0 font-semibold text-muted-foreground">
                            {saved.request.method}
                          </span>
                          <span className="truncate">{saved.name}</span>
                        </button>
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label={`Delete ${saved.name}`}
                          onClick={() => removeRequest.mutate(saved.id)}
                          className="opacity-0 group-hover:opacity-100"
                        >
                          <Trash2 className="h-3.5 w-3.5" />
                        </Button>
                      </li>
                    ))}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
