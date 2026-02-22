/**
 * ModuleScaffold — dialog for creating a new module manifest.
 *
 * Collects: name, type, runtime, command, template, image, env vars, volumes.
 * Shows a live TOML preview of the manifest that will be written.
 */

import { useEffect, useState } from "react";

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
import { useModuleStore, type ModuleType, type RuntimeType } from "@/stores/moduleStore";

// ─── TOML preview generator ───────────────────────────────────────────────────

function generateToml(
  name: string,
  moduleType: ModuleType,
  runtimeType: RuntimeType,
  command: string,
  description: string,
  image: string,
  env: Record<string, string>,
  volumes: string[],
): string {
  const id = name.toLowerCase().replace(/\s+/g, "-").replace(/[^a-z0-9-]/g, "");

  let toml = `[module]
id = "${id || "my-module"}"
name = "${name || "My Module"}"
version = "0.1.0"
description = "${description || "A new module"}"
type = "${moduleType}"

[runtime]
type = "${runtimeType}"
command = "${command || "my-binary"}"
args = []
`;

  // Add image for container runtimes
  if ((runtimeType === "docker" || runtimeType === "podman") && image) {
    toml += `image = "${image}"\n`;
  }

  // Add environment variables
  const envKeys = Object.keys(env);
  if (envKeys.length > 0) {
    toml += `env = {\n`;
    toml += envKeys.map((k) => `  "${k}" = "${env[k]}"`).join(",\n");
    toml += `\n}\n`;
  }

  // Add volumes
  if (volumes.length > 0) {
    toml += `volumes = [${volumes.map((v) => `"${v}"`).join(", ")}]\n`;
  }

  toml += `timeout_secs = 30

[security]
allow_network = false
allow_filesystem = false
max_memory_mb = 256
`;

  return toml;
}

// ─── EnvVarEditor ─────────────────────────────────────────────────────────────

interface EnvVarEditorProps {
  env: Record<string, string>;
  onChange: (env: Record<string, string>) => void;
}

function EnvVarEditor({ env, onChange }: EnvVarEditorProps) {
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");

  const addEnvVar = () => {
    if (newKey.trim()) {
      onChange({ ...env, [newKey.trim()]: newValue });
      setNewKey("");
      setNewValue("");
    }
  };

  const removeEnvVar = (key: string) => {
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    const { [key]: _, ...rest } = env;
    onChange(rest);
  };

  const envEntries = Object.entries(env);

  return (
    <div className="flex flex-col gap-2">
      <label className="text-xs text-muted-foreground">Environment Variables</label>

      {envEntries.length > 0 && (
        <div className="flex flex-col gap-1">
          {envEntries.map(([key, value]) => (
            <div key={key} className="flex items-center gap-2">
              <span className="text-xs font-mono bg-muted px-2 py-1 rounded flex-1 truncate">
                {key}={value}
              </span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={() => removeEnvVar(key)}
              >
                ×
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className="flex gap-2">
        <Input
          value={newKey}
          onChange={(e) => setNewKey(e.target.value)}
          placeholder="KEY"
          className="flex-1 h-8 text-xs font-mono"
          onKeyDown={(e) => e.key === "Enter" && addEnvVar()}
        />
        <Input
          value={newValue}
          onChange={(e) => setNewValue(e.target.value)}
          placeholder="value"
          className="flex-1 h-8 text-xs font-mono"
          onKeyDown={(e) => e.key === "Enter" && addEnvVar()}
        />
        <Button
          variant="outline"
          size="sm"
          className="h-8 px-3 text-xs"
          onClick={addEnvVar}
          disabled={!newKey.trim()}
        >
          Add
        </Button>
      </div>
    </div>
  );
}

// ─── VolumeEditor ─────────────────────────────────────────────────────────────

interface VolumeEditorProps {
  volumes: string[];
  onChange: (volumes: string[]) => void;
}

function VolumeEditor({ volumes, onChange }: VolumeEditorProps) {
  const [newVolume, setNewVolume] = useState("");

  const addVolume = () => {
    if (newVolume.trim()) {
      onChange([...volumes, newVolume.trim()]);
      setNewVolume("");
    }
  };

  const removeVolume = (index: number) => {
    onChange(volumes.filter((_, i) => i !== index));
  };

  return (
    <div className="flex flex-col gap-2">
      <label className="text-xs text-muted-foreground">Volume Mounts</label>

      {volumes.length > 0 && (
        <div className="flex flex-col gap-1">
          {volumes.map((vol, index) => (
            <div key={index} className="flex items-center gap-2">
              <span className="text-xs font-mono bg-muted px-2 py-1 rounded flex-1 truncate">
                {vol}
              </span>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 px-2 text-xs"
                onClick={() => removeVolume(index)}
              >
                ×
              </Button>
            </div>
          ))}
        </div>
      )}

      <div className="flex gap-2">
        <Input
          value={newVolume}
          onChange={(e) => setNewVolume(e.target.value)}
          placeholder="/host/path:/container/path:ro"
          className="flex-1 h-8 text-xs font-mono"
          onKeyDown={(e) => e.key === "Enter" && addVolume()}
        />
        <Button
          variant="outline"
          size="sm"
          className="h-8 px-3 text-xs"
          onClick={addVolume}
          disabled={!newVolume.trim()}
        >
          Add
        </Button>
      </div>
    </div>
  );
}

// ─── ModuleScaffold ───────────────────────────────────────────────────────────

interface ModuleScaffoldProps {
  className?: string;
}

export function ModuleScaffold({ className }: ModuleScaffoldProps) {
  const scaffoldOpen = useModuleStore((s) => s.scaffoldOpen);
  const scaffoldForm = useModuleStore((s) => s.scaffoldForm);
  const scaffolding = useModuleStore((s) => s.scaffolding);
  const templates = useModuleStore((s) => s.templates);
  const error = useModuleStore((s) => s.error);
  const closeScaffold = useModuleStore((s) => s.closeScaffold);
  const updateScaffoldForm = useModuleStore((s) => s.updateScaffoldForm);
  const submitScaffold = useModuleStore((s) => s.submitScaffold);
  const loadTemplates = useModuleStore((s) => s.loadTemplates);

  // Load templates on mount
  useEffect(() => {
    if (scaffoldOpen && templates.length === 0) {
      loadTemplates();
    }
  }, [scaffoldOpen, templates.length, loadTemplates]);

  // Auto-set image when template changes
  useEffect(() => {
    const selectedTemplate = templates.find((t) => t.id === scaffoldForm.template);
    if (
      selectedTemplate?.defaultImage &&
      (scaffoldForm.runtimeType === "docker" || scaffoldForm.runtimeType === "podman") &&
      !scaffoldForm.image
    ) {
      updateScaffoldForm({ image: selectedTemplate.defaultImage });
    }
  }, [scaffoldForm.template, scaffoldForm.runtimeType, scaffoldForm.image, templates, updateScaffoldForm]);

  const toml = generateToml(
    scaffoldForm.name,
    scaffoldForm.moduleType,
    scaffoldForm.runtimeType,
    scaffoldForm.command,
    scaffoldForm.description,
    scaffoldForm.image,
    scaffoldForm.env,
    scaffoldForm.volumes,
  );

  const canSubmit = scaffoldForm.name.trim() !== "" && !scaffolding;

  const isContainer = scaffoldForm.runtimeType === "docker" || scaffoldForm.runtimeType === "podman";

  return (
    <Dialog open={scaffoldOpen} onOpenChange={(v) => !v && closeScaffold()}>
      <DialogContent className={cn("max-w-3xl max-h-[90vh] overflow-y-auto", className)}>
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

            {/* Template selector */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Template</label>
              <select
                value={scaffoldForm.template}
                onChange={(e) => updateScaffoldForm({ template: e.target.value })}
                className="rounded-md border bg-background px-2 py-1 text-xs focus:outline-none focus-visible:ring-2 focus-visible:ring-ring w-full"
              >
                <option value="empty">Empty — minimal manifest only</option>
                <option value="python_tool">Python Tool — stdin/stdout JSON-RPC</option>
                <option value="python_ml">Python ML — pandas/numpy analysis</option>
                <option value="python_service">Python HTTP Service</option>
                <option value="node_tool">Node.js Tool — stdin/stdout JSON-RPC</option>
              </select>
            </div>

            {/* Container image (only for docker/podman) */}
            {isContainer && (
              <div className="flex flex-col gap-1">
                <label className="text-xs text-muted-foreground">Container Image</label>
                <Input
                  value={scaffoldForm.image}
                  onChange={(e) => updateScaffoldForm({ image: e.target.value })}
                  placeholder="python:3.12-slim"
                  className="font-mono"
                />
              </div>
            )}

            {/* Command */}
            <div className="flex flex-col gap-1">
              <label className="text-xs text-muted-foreground">Command</label>
              <Input
                value={scaffoldForm.command}
                onChange={(e) => updateScaffoldForm({ command: e.target.value })}
                placeholder={isContainer ? "python3 (optional, uses template default)" : "my-binary or /path/to/binary"}
                className="font-mono"
              />
            </div>

            {/* Environment variables */}
            <EnvVarEditor
              env={scaffoldForm.env}
              onChange={(env) => updateScaffoldForm({ env })}
            />

            {/* Volume mounts (only for containers) */}
            {isContainer && (
              <VolumeEditor
                volumes={scaffoldForm.volumes}
                onChange={(volumes) => updateScaffoldForm({ volumes })}
              />
            )}

            {error && <p className="text-xs text-destructive">{error}</p>}
          </div>

          {/* ── Right: TOML preview ── */}
          <div className="flex flex-col gap-1">
            <div className="flex items-center gap-2">
              <label className="text-xs text-muted-foreground">
                manifest.toml preview
              </label>
              <Badge variant="outline" className="text-xs">live</Badge>
            </div>
            <pre className="flex-1 rounded-md border bg-muted/30 p-3 text-[11px] font-mono leading-relaxed overflow-auto whitespace-pre min-h-[400px]">
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
