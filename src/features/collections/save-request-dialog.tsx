import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import { createCollection, listCollections, saveRequest } from "@/lib/tauri";
import type { Collection, HttpRequest } from "@/types";

interface SaveRequestDialogProps {
  open: boolean;
  request: HttpRequest | null;
  defaultName: string;
  onClose: () => void;
  onSaved: (collectionId: string, name: string) => void;
}

export function SaveRequestDialog({
  open,
  request,
  defaultName,
  onClose,
  onSaved,
}: SaveRequestDialogProps) {
  const queryClient = useQueryClient();
  const [name, setName] = useState(defaultName);
  const [collectionId, setCollectionId] = useState<string>("");
  const [newCollection, setNewCollection] = useState("");

  const { data: collections = [] } = useQuery({
    queryKey: ["collections"],
    queryFn: listCollections,
    enabled: open,
  });

  useEffect(() => {
    if (open) setName(defaultName);
  }, [open, defaultName]);

  useEffect(() => {
    if (collectionId === "" && collections.length > 0) {
      setCollectionId(collections[0]?.id ?? "");
    }
  }, [collections, collectionId]);

  const save = useMutation({
    mutationFn: async () => {
      if (!request) throw new Error("no request to save");
      let targetId = collectionId;
      if (targetId === "" || targetId === "__new__") {
        const created = await createCollection(newCollection.trim() || "My collection");
        targetId = created.id;
      }
      await saveRequest(targetId, name.trim(), request);
      return { targetId, name: name.trim() };
    },
    onSuccess: ({ targetId, name: savedName }) => {
      queryClient.invalidateQueries({ queryKey: ["collections"] });
      onSaved(targetId, savedName);
      onClose();
    },
  });

  if (!open || !request) return null;

  const needsNewName = collectionId === "__new__" || collections.length === 0;

  return (
    <div
      role="dialog"
      aria-label="Save request"
      aria-modal="true"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-8"
      onClick={onClose}
    >
      <form
        className="w-96 space-y-3 rounded-lg border border-border bg-background p-4 shadow-xl"
        onClick={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault();
          if (name.trim()) save.mutate();
        }}
      >
        <h2 className="text-sm font-medium">Save request</h2>

        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">Request name</span>
          <Input
            autoFocus
            aria-label="Request name"
            value={name}
            onChange={(e) => setName(e.target.value)}
          />
        </label>

        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">Collection</span>
          <Select
            aria-label="Target collection"
            value={collectionId}
            onChange={(e) => setCollectionId(e.target.value)}
            className="w-full"
          >
            {collections.map((collection: Collection) => (
              <option key={collection.id} value={collection.id}>
                {collection.name}
              </option>
            ))}
            <option value="__new__">New collection…</option>
          </Select>
        </label>

        {needsNewName && (
          <label className="block space-y-1">
            <span className="text-xs text-muted-foreground">New collection name</span>
            <Input
              aria-label="New collection name"
              value={newCollection}
              placeholder="My collection"
              onChange={(e) => setNewCollection(e.target.value)}
            />
          </label>
        )}

        {save.isError && (
          <p className="text-xs text-red-500">Could not save the request. Please try again.</p>
        )}

        <div className="flex justify-end gap-2 pt-1">
          <Button type="button" variant="outline" onClick={onClose}>
            Cancel
          </Button>
          <Button type="submit" disabled={!name.trim() || save.isPending}>
            Save
          </Button>
        </div>
      </form>
    </div>
  );
}
