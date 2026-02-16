import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

import { SettingRow } from "@/components/setting-row";
import { SettingsSection } from "@/components/settings-section";
import { Button } from "@/components/ui/button";
import { Loader2, Trash2, CheckCircle2 } from "@/lib/icons";

interface CacheManagementProps {
  workspaceId: string | null;
}

export function CacheManagement({ workspaceId }: CacheManagementProps) {
  const [isClearingCache, setIsClearingCache] = useState(false);
  const [clearSuccess, setClearSuccess] = useState(false);

  const handleClearCache = async () => {
    if (!workspaceId) {
      return;
    }

    setIsClearingCache(true);
    setClearSuccess(false);

    try {
      await invoke("clear_cache", { workspaceId });
      setClearSuccess(true);
      setTimeout(() => setClearSuccess(false), 3000);
    } catch (error) {
      console.error("Failed to clear cache:", error);
    } finally {
      setIsClearingCache(false);
    }
  };

  return (
    <SettingsSection
      title="Cache Management"
      description="Manage cached explanations and schema data"
    >
      <SettingRow
        label="Clear Explanation Cache"
        description="Remove all cached AI-generated explanations. This will force regeneration of all explanations."
      >
        <div className="flex flex-col gap-2">
          <Button
            onClick={handleClearCache}
            disabled={isClearingCache || !workspaceId}
            variant="outline"
            className="w-full max-w-sm"
          >
            {isClearingCache ? (
              <>
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                Clearing...
              </>
            ) : (
              <>
                <Trash2 className="mr-2 h-4 w-4" />
                Clear Cache
              </>
            )}
          </Button>
          {clearSuccess && (
            <div className="flex items-center gap-2 text-sm text-green-600">
              <CheckCircle2 className="h-4 w-4" />
              <span>Cache cleared successfully</span>
            </div>
          )}
          {!workspaceId && (
            <p className="text-sm text-muted-foreground">
              Connect to a database to manage cache
            </p>
          )}
        </div>
      </SettingRow>

      <div className="rounded-lg border border-border bg-muted/50 p-4">
        <h3 className="mb-2 text-sm font-medium">About Caching</h3>
        <ul className="space-y-1 text-sm text-muted-foreground">
          <li>
            • Explanations are cached to improve performance and reduce API
            costs
          </li>
          <li>
            • Cache is automatically invalidated when schema changes are
            detected
          </li>
          <li>• Clearing cache will require regenerating all explanations</li>
          <li>• Cache is stored locally in your application database</li>
        </ul>
      </div>
    </SettingsSection>
  );
}
