import { useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface AddCustomModelDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  providerId: string;
  providerName: string;
  onAdd: (modelId: string, displayName: string) => void;
}

export function AddCustomModelDialog({
  open,
  onOpenChange,
  providerId: _providerId,
  providerName,
  onAdd,
}: AddCustomModelDialogProps) {
  const [modelId, setModelId] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!modelId.trim()) {
      return;
    }

    setIsLoading(true);
    try {
      await onAdd(modelId.trim(), displayName.trim() || modelId.trim());
      // Reset form on success
      setModelId("");
      setDisplayName("");
      onOpenChange(false);
    } finally {
      setIsLoading(false);
    }
  };

  const handleCancel = () => {
    // Reset form
    setModelId("");
    setDisplayName("");
    onOpenChange(false);
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Add Custom Model</DialogTitle>
          <DialogDescription>
            Add a custom AI model to {providerName} by entering its model ID.
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit}>
          <div className="flex flex-col gap-4 py-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="model-id">Model ID</Label>
              <Input
                id="model-id"
                placeholder="e.g., anthropic/claude-3.5-sonnet"
                value={modelId}
                onChange={(e) => setModelId(e.target.value)}
                disabled={isLoading}
                required
              />
              <p className="text-xs text-muted-foreground">
                Enter the model ID as required by {providerName}. For example:
                <br />
                <span className="font-mono text-xs">
                  anthropic/claude-3.5-sonnet
                </span>
                <br />
                <span className="font-mono text-xs">openai/gpt-4o</span>
              </p>
            </div>

            <div className="flex flex-col gap-2">
              <Label htmlFor="display-name">Display Name (optional)</Label>
              <Input
                id="display-name"
                placeholder="Defaults to model ID if empty"
                value={displayName}
                onChange={(e) => setDisplayName(e.target.value)}
                disabled={isLoading}
              />
              <p className="text-xs text-muted-foreground">
                A friendly name for display in the UI. If left empty, the model
                ID will be used.
              </p>
            </div>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={handleCancel}
              disabled={isLoading}
            >
              Cancel
            </Button>
            <Button type="submit" disabled={isLoading || !modelId.trim()}>
              {isLoading ? "Adding..." : "Add Model"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
