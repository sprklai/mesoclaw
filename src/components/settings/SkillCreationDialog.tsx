/**
 * Dialog for creating new AI skills/prompt templates.
 */

import { Plus, Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select } from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { useSkillStore } from "@/stores/skillStore";

interface SkillCreationDialogProps {
  onCreated?: () => void;
}

const CATEGORY_OPTIONS: Array<{ value: string; label: string }> = [
  { value: "general", label: "General" },
  { value: "performance", label: "Performance" },
  { value: "understanding", label: "Understanding" },
  { value: "security", label: "Security" },
  { value: "documentation", label: "Documentation" },
];

const DEFAULT_TEMPLATE = `You are an AI assistant specialized in a specific task.

User request: {{ request }}

Provide a helpful and accurate response.`;

export function SkillCreationDialog({ onCreated }: SkillCreationDialogProps) {
  const [open, setOpen] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [id, setId] = useState("");
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [category, setCategory] = useState("general");
  const [template, setTemplate] = useState(DEFAULT_TEMPLATE);

  const createSkillAction = useSkillStore((state) => state.createSkill);

  const resetForm = () => {
    setId("");
    setName("");
    setDescription("");
    setCategory("general");
    setTemplate(DEFAULT_TEMPLATE);
  };

  const handleCreate = async () => {
    // Validation
    if (!id.trim()) {
      toast.error("Please enter a skill ID");
      return;
    }
    if (!name.trim()) {
      toast.error("Please enter a skill name");
      return;
    }
    if (!template.trim()) {
      toast.error("Please enter a template");
      return;
    }

    // Validate ID format (lowercase, alphanumeric, hyphens only)
    if (!/^[a-z0-9-]+$/.test(id)) {
      toast.error("Skill ID must be lowercase letters, numbers, and hyphens only");
      return;
    }

    setIsCreating(true);
    try {
      const result = await createSkillAction({
        id: id.trim(),
        name: name.trim(),
        description: description.trim(),
        category,
        template: template.trim(),
      });

      if (result) {
        toast.success(`Skill "${name}" created successfully`);
        resetForm();
        setOpen(false);
        onCreated?.();
      }
    } finally {
      setIsCreating(false);
    }
  };

  const handleOpenChange = (newOpen: boolean) => {
    setOpen(newOpen);
    if (!newOpen) {
      resetForm();
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogTrigger asChild>
        <Button size="sm">
          <Plus className="h-4 w-4 mr-2" />
          Create Skill
        </Button>
      </DialogTrigger>
      <DialogContent className="sm:max-w-[600px]">
        <DialogHeader>
          <DialogTitle>Create New Skill</DialogTitle>
          <DialogDescription>
            Create a reusable prompt template for AI interactions. Use{" "}
            <code className="text-xs bg-muted px-1 rounded">
              {"{{ variable }}"}
            </code>{" "}
            syntax for dynamic content.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-4 py-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="skill-id">Skill ID</Label>
              <Input
                id="skill-id"
                placeholder="my-skill"
                value={id}
                onChange={(e) => setId(e.target.value.toLowerCase())}
                disabled={isCreating}
              />
              <p className="text-xs text-muted-foreground">
                Lowercase letters, numbers, hyphens only
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="skill-name">Display Name</Label>
              <Input
                id="skill-name"
                placeholder="My Skill"
                value={name}
                onChange={(e) => setName(e.target.value)}
                disabled={isCreating}
              />
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="skill-description">Description</Label>
            <Input
              id="skill-description"
              placeholder="What does this skill do?"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              disabled={isCreating}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="skill-category">Category</Label>
            <Select
              value={category}
              onValueChange={setCategory}
              options={CATEGORY_OPTIONS}
              disabled={isCreating}
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="skill-template">Template</Label>
            <Textarea
              id="skill-template"
              placeholder="Enter the prompt template..."
              value={template}
              onChange={(e) => setTemplate(e.target.value)}
              disabled={isCreating}
              rows={10}
              className="font-mono text-sm"
            />
            <p className="text-xs text-muted-foreground">
              Use <code className="text-xs">{"{{ variable }}"}</code> for
              parameters. Example: <code className="text-xs">{"{{ request }}"}</code>
            </p>
          </div>
        </div>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => handleOpenChange(false)}
            disabled={isCreating}
          >
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={isCreating}>
            {isCreating ? (
              <>
                <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                Creating...
              </>
            ) : (
              "Create Skill"
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
