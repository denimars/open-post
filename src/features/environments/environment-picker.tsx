import { useEffect } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Settings2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Select } from "@/components/ui/select";
import { getActiveEnvironment, listEnvironments, setActiveEnvironment } from "@/lib/tauri";
import { useTabsStore } from "@/stores/tabs";
import type { Environment } from "@/types";

interface EnvironmentPickerProps {
  onManage: () => void;
}

export function EnvironmentPicker({ onManage }: EnvironmentPickerProps) {
  const queryClient = useQueryClient();
  const { activeEnvironmentId, setActiveEnvironment: setLocalActive } = useTabsStore();

  const { data: environments = [] } = useQuery({
    queryKey: ["environments"],
    queryFn: listEnvironments,
  });
  const { data: storedActive } = useQuery({
    queryKey: ["activeEnvironment"],
    queryFn: getActiveEnvironment,
  });

  // Adopt the persisted selection once, on first load.
  useEffect(() => {
    if (storedActive !== undefined && activeEnvironmentId === null && storedActive !== null) {
      setLocalActive(storedActive);
    }
  }, [storedActive, activeEnvironmentId, setLocalActive]);

  const select = useMutation({
    mutationFn: (id: string | null) => setActiveEnvironment(id),
    onSuccess: (_data, id) => {
      setLocalActive(id);
      queryClient.invalidateQueries({ queryKey: ["activeEnvironment"] });
    },
  });

  return (
    <div className="flex items-center gap-1">
      <Select
        aria-label="Active environment"
        value={activeEnvironmentId ?? ""}
        onChange={(e) => select.mutate(e.target.value === "" ? null : e.target.value)}
        className="h-7 text-xs"
      >
        <option value="">No environment</option>
        {environments.map((env: Environment) => (
          <option key={env.id} value={env.id}>
            {env.name}
          </option>
        ))}
      </Select>
      <Button variant="ghost" size="icon" aria-label="Manage environments" onClick={onManage}>
        <Settings2 className="h-4 w-4" />
      </Button>
    </div>
  );
}
