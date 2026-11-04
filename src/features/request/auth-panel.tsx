import { Input } from "@/components/ui/input";
import { Select } from "@/components/ui/select";
import type { Auth } from "@/types";

interface AuthPanelProps {
  auth: Auth;
  onChange: (auth: Auth) => void;
}

export function AuthPanel({ auth, onChange }: AuthPanelProps) {
  function setType(type: Auth["type"]) {
    if (type === auth.type) return;
    if (type === "none") onChange({ type: "none" });
    else if (type === "bearer") onChange({ type: "bearer", token: "" });
    else onChange({ type: "basic", username: "", password: "" });
  }

  return (
    <div className="space-y-3 p-3">
      <label className="flex items-center gap-2 text-xs text-muted-foreground">
        Type
        <Select
          aria-label="Auth type"
          value={auth.type}
          onChange={(e) => setType(e.target.value as Auth["type"])}
        >
          <option value="none">No Auth</option>
          <option value="bearer">Bearer Token</option>
          <option value="basic">Basic Auth</option>
        </Select>
      </label>

      {auth.type === "bearer" && (
        <label className="block space-y-1">
          <span className="text-xs text-muted-foreground">Token</span>
          <Input
            aria-label="Bearer token"
            value={auth.token}
            placeholder="{{token}}"
            onChange={(e) => onChange({ type: "bearer", token: e.target.value })}
            className="font-mono text-xs"
          />
        </label>
      )}

      {auth.type === "basic" && (
        <div className="grid grid-cols-2 gap-3">
          <label className="space-y-1">
            <span className="text-xs text-muted-foreground">Username</span>
            <Input
              aria-label="Basic auth username"
              value={auth.username}
              onChange={(e) => onChange({ ...auth, username: e.target.value })}
            />
          </label>
          <label className="space-y-1">
            <span className="text-xs text-muted-foreground">Password</span>
            <Input
              aria-label="Basic auth password"
              type="password"
              value={auth.password}
              onChange={(e) => onChange({ ...auth, password: e.target.value })}
            />
          </label>
        </div>
      )}

      {auth.type === "none" && (
        <p className="text-xs text-muted-foreground">
          This request does not use authentication.
        </p>
      )}
    </div>
  );
}
