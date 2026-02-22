/**
 * ProfileSettings — user name and bot display name configuration.
 *
 * Allows users to set their name (for personalized responses) and
 * the bot's display name (how the assistant refers to itself).
 */

import { useEffect, useState } from "react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { APP_IDENTITY } from "@/config/app-identity";
import { useAppSettingsStore, useAppIdentity } from "@/stores/appSettingsStore";

export function ProfileSettings() {
  const { userName, appDisplayName, setUserIdentity, loadUserIdentity } =
    useAppSettingsStore();
  const identity = useAppIdentity();

  const [nameInput, setNameInput] = useState("");
  const [displayNameInput, setDisplayNameInput] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  // Load existing identity on mount
  useEffect(() => {
    loadUserIdentity();
  }, [loadUserIdentity]);

  // Update form when identity loads
  useEffect(() => {
    setNameInput(userName ?? "");
    setDisplayNameInput(appDisplayName ?? APP_IDENTITY.productName);
    setHasChanges(false);
  }, [userName, appDisplayName]);

  // Track changes
  useEffect(() => {
    const nameChanged = nameInput !== (userName ?? "");
    const displayNameChanged =
      displayNameInput !== (appDisplayName ?? APP_IDENTITY.productName);
    setHasChanges(nameChanged || displayNameChanged);
  }, [nameInput, displayNameInput, userName, appDisplayName]);

  async function handleSave() {
    setIsSaving(true);
    try {
      await setUserIdentity(
        nameInput.trim() || null,
        displayNameInput.trim() || null,
      );
      toast.success("Profile updated");
      setHasChanges(false);
    } catch (error) {
      toast.error("Failed to save profile");
      console.error("Failed to save identity:", error);
    } finally {
      setIsSaving(false);
    }
  }

  function handleReset() {
    setNameInput(userName ?? "");
    setDisplayNameInput(appDisplayName ?? APP_IDENTITY.productName);
    setHasChanges(false);
  }

  return (
    <div className="space-y-6">
      {/* Preview card */}
      <div className="rounded-lg border border-border bg-card p-4">
        <p className="mb-2 text-xs font-medium uppercase tracking-wider text-muted-foreground">
          Preview
        </p>
        <p className="text-sm text-foreground">
          {getGreeting(nameInput, identity.productName)}
        </p>
      </div>

      {/* Form fields */}
      <div className="space-y-4">
        <div className="space-y-1.5">
          <Label htmlFor="user-name">Your Name</Label>
          <Input
            id="user-name"
            type="text"
            placeholder="Enter your name for personalization"
            value={nameInput}
            onChange={(e) => setNameInput(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            Optional. Used to personalize AI responses.
          </p>
        </div>

        <div className="space-y-1.5">
          <Label htmlFor="app-display-name">Bot Display Name</Label>
          <Input
            id="app-display-name"
            type="text"
            placeholder="What would you like to call the assistant?"
            value={displayNameInput}
            onChange={(e) => setDisplayNameInput(e.target.value)}
          />
          <p className="text-xs text-muted-foreground">
            This is how the assistant will refer to itself. Defaults to{" "}
            {APP_IDENTITY.productName}.
          </p>
        </div>
      </div>

      {/* Actions */}
      {hasChanges && (
        <div className="flex items-center gap-2">
          <Button onClick={handleSave} disabled={isSaving}>
            {isSaving ? "Saving..." : "Save Changes"}
          </Button>
          <Button variant="ghost" onClick={handleReset} disabled={isSaving}>
            Reset
          </Button>
        </div>
      )}
    </div>
  );
}

function getGreeting(userName: string, botName: string): string {
  const hour = new Date().getHours();
  const timeGreeting =
    hour < 12 ? "Good morning" : hour < 17 ? "Good afternoon" : "Good evening";

  if (userName.trim()) {
    return `${timeGreeting}, ${userName.trim()}! I'm ${botName}. How can I help you today?`;
  }
  return `${timeGreeting}! I'm ${botName}. How can I help you today?`;
}
