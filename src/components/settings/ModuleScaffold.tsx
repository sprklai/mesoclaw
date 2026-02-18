/**
 * ModuleScaffold — dialog for creating a new module manifest.
 *
 * Collects: name, type, runtime, command.
 * Shows a live TOML preview of the manifest that will be written.
 */

import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { useModuleStore } from "@/stores/moduleStore";
import type { ModuleType, RuntimeType } from "@/stores/moduleStore";

// ─── TOML preview generator ───────────────────────────────────────────────────

function generateToml(
  name: string,
  moduleType: ModuleType,
  runtimeType: RuntimeType,
  command: string,
  description: string,
): string {
  const id = name.toLowerCase().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "");
  return `[module]
id = "${id || "my-module"}"
name = "${name || "My Module"}"
version = "0.1.0"
description = "${description || "A new module"}"
type = "${moduleType}"

[runtime]
type = "${runtimeType}"
command = "${command || "my-binary"}"
args = []
timeout_secs = 30

[security]
allow_network = false
allow_filesystem = false
max_memory_mb = 256
`;
}

// ─── ModuleScaffold ───────────────────────────────────────────────────────────

interface ModuleScaffoldProps {
  className?: string;
}

export function ModuleScaffold({ className }: ModuleScaffoldProps) {
  const scaffoldOpen = useModuleStore((s) => s.scaffoldOpen);
  const scaffoldForm = useModuleStore((s) => s.scaffoldForm);
  const scaffolding = useModuleStore((s) => s.scaffolding);
  const error = useModuleStore((s) => s.error);
  const closeScaffold = useModuleStore((s) => s.closeScaffold);
  const updateScaffoldForm = useModuleStore((s) => s.updateScaffoldForm);
  const submitScaffold = useModuleStore((s) => s.submitScaffold);

  const toml = generateToml(
    scaffoldForm.name,
    scaffoldForm.moduleType,
    scaffoldForm.runtimeType,
    scaffoldForm.command,
    scaffoldForm.description,
  );

  const canSubmit = scaffoldForm.name.trim() !== "" && scaffoldForm.command.trim() !== "" && !scaffolding;

  return (
    <Dialog open={scaffoldOpen} onOpenChange={(v) => !v && closeScaffold()}>
      <DialogContent className={cn("max-w-2xl", className)}>
        <DialogHeader>
          <DialogTitle>New Module</DialogTitle>
        </DialogHeader>

        <div className="grid grid-cols-2 gap-4">
          {/* ── Left: form ── */}
          <div className="flex flex-col gap-3">
            {/* Name */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Name</label>
              <Input
                value={scaffoldForm.name}
                onChange={(e) => updateScaffoldForm({ name: e.target.value })}
                placeholder="My Awesome Tool"
              />
            </div>

            {/* Description */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Description</label>
              <Input
                value={scaffoldForm.description}
                onChange={(e) => updateScaffoldForm({ description: e.target.value })}
                placeholder="What this module does"
              />
            </div>

            {/* Module type */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Type</label>
              <select
                value={scaffoldForm.moduleType}
                onChange={(e) =>
                  updateScaffoldForm({ moduleType: e.target.value as ModuleType })
                }
                className="rounded-md border bg-background px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-ring w-full"
              >
                <option value="tool">Tool — spawned on demand</option>
                <option value="service">Service — long-running process</option>
                <option value="mcp">MCP — Model Context Protocol server</option>
              </select>
            </div>

            {/* Runtime type */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Runtime</label>
              <select
                value={scaffoldForm.runtimeType}
                onChange={(e) =>
                  updateScaffoldForm({ runtimeType: e.target.value as RuntimeType })
                }
                className="rounded-md border bg-background px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-ring w-full"
              >
                <option value="native">Native (OS process)</option>
                <option value="docker">Docker container</option>
                <option value="podman">Podman container</option>
              </select>
            </div>

            {/* Command */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Command</label>
              <Input
                value={scaffoldForm.command}
                onChange={(e) => updateScaffoldForm({ command: e.target.value })}
                placeholder="my-binary or /path/to/binary"
                className="font-mono"
              />
            </div>

            {error && <p className="text-xs text-destructive">{error}</p>}
          </div>

          {/* ── Right: TOML preview ── */}
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <label className="text-xs text-muted-foreground">
                manifest.toml preview
              </label>
              <Badge variant="outline" className="text-[10px]">live</Badge>
            </div>
            <pre className="flex-1 rounded-md border bg-muted/30 p-3 text-[11px] font-mono leading-relaxed overflow-auto whitespace-pre">
              {toml}
            </pre>
          </div>
        </div>

        <DialogFooter className="mt-2">
          <Button variant="ghost" size="sm" onClick={closeScaffold}>
            Cancel
          </Button>
          <Button
            variant="default"
            size="sm"
            disabled={!canSubmit}
            onClick={submitScaffold}
          >
            {scaffolding ? "Creating…" : "Create Module"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
