/**
 * MatrixConfig — configuration panel for the Matrix channel.
 *
 * Shows a setup guide for Matrix/Element and input fields for:
 * - Homeserver URL
 * - Username (MXID)
 * - Access token (password field)
 * - Allowed room IDs (comma-separated)
 *
 * Matrix is a strategic integration because it can bridge to WhatsApp, Slack,
 * IRC, Signal, and more via protocol bridges — giving MesoClaw reach across
 * many platforms through a single Matrix account.
 *
 * Phase 7.3 implementation.
 */

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  type MatrixChannelConfig,
  useChannelStore,
} from "@/stores/channelStore";

// ─── MatrixConfig ─────────────────────────────────────────────────────────────

interface MatrixConfigProps {
  /** Current Matrix config from the store. */
  config: MatrixChannelConfig;
}

export function MatrixConfig({ config }: MatrixConfigProps) {
  const { updateMatrixConfig, testConnection } = useChannelStore();
  const [draft, setDraft] = useState<MatrixChannelConfig>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleChange =
    (field: keyof MatrixChannelConfig) =>
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setDraft((prev) => ({ ...prev, [field]: e.target.value }));
      setTestResult(null);
    };

  const handleTest = async () => {
    setIsTesting(true);
    setTestResult(null);
    const ok = await testConnection("matrix");
    setTestResult(ok ? "ok" : "fail");
    setIsTesting(false);
  };

  const handleSave = async () => {
    setIsSaving(true);
    await updateMatrixConfig(draft);
    setIsSaving(false);
  };

  return (
    <div className="space-y-6">
      {/* Setup guide */}
      <section className="rounded-lg border border-border bg-muted/30 p-4">
        <h3 className="mb-3 text-sm font-semibold">
          How to connect a Matrix bot account
        </h3>
        <ol className="space-y-2 text-sm text-muted-foreground">
          <li>
            <span className="font-medium text-foreground">1.</span> Create a
            bot account on any homeserver (e.g.{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              matrix.org
            </span>
            ).
          </li>
          <li>
            <span className="font-medium text-foreground">2.</span> In Element
            → Settings → Help &amp; About → scroll to{" "}
            <span className="font-medium">Access Token</span> and copy it.
            Alternatively use the Matrix login API to obtain a token
            programmatically.
          </li>
          <li>
            <span className="font-medium text-foreground">3.</span> Invite the
            bot account to the rooms you want it to monitor, then paste the
            room IDs below.
          </li>
          <li>
            <span className="font-medium text-foreground">4.</span>{" "}
            <span className="font-medium">Bridge tip:</span> Install bridge bots
            (e.g. mautrix-whatsapp) on your homeserver to relay WhatsApp,
            Signal, or Slack messages through Matrix to MesoClaw.
          </li>
        </ol>
      </section>

      {/* Homeserver URL */}
      <div className="space-y-2">
        <Label htmlFor="mx-homeserver">Homeserver URL</Label>
        <Input
          id="mx-homeserver"
          type="url"
          placeholder="https://matrix.org"
          value={draft.homeserverUrl}
          onChange={handleChange("homeserverUrl")}
        />
        <p className="text-xs text-muted-foreground">
          Full URL of your Matrix homeserver including{" "}
          <span className="font-mono">https://</span>.
        </p>
      </div>

      {/* Username */}
      <div className="space-y-2">
        <Label htmlFor="mx-username">Username (MXID)</Label>
        <Input
          id="mx-username"
          type="text"
          placeholder="@mybot:matrix.org"
          value={draft.username}
          onChange={handleChange("username")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          Full Matrix ID including the server part (e.g.{" "}
          <span className="font-mono">@user:matrix.org</span>).
        </p>
      </div>

      {/* Access token */}
      <div className="space-y-2">
        <Label htmlFor="mx-token">Access Token</Label>
        <Input
          id="mx-token"
          type="password"
          placeholder="syt_dXNlcm5hbWU_abc123…"
          value={draft.accessToken}
          onChange={handleChange("accessToken")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          Obtained from Element → Settings → Help &amp; About. Stored securely
          in the OS keyring.
        </p>
      </div>

      {/* Allowed room IDs */}
      <div className="space-y-2">
        <Label htmlFor="mx-rooms">Allowed Room IDs</Label>
        <Input
          id="mx-rooms"
          type="text"
          placeholder="!abc123:matrix.org, !xyz789:matrix.org"
          value={draft.allowedRoomIds}
          onChange={handleChange("allowedRoomIds")}
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated room IDs. Leave empty to receive messages from all
          joined rooms. Find room IDs in Element → Room Settings → Advanced.
        </p>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-2">
        <Button
          variant="outline"
          size="sm"
          onClick={handleTest}
          disabled={isTesting || !draft.accessToken}
        >
          {isTesting ? "Testing…" : "Test Connection"}
        </Button>

        {testResult === "ok" && (
          <span className="text-sm font-medium text-green-600">
            ✓ Connected successfully
          </span>
        )}
        {testResult === "fail" && (
          <span className="text-sm font-medium text-destructive">
            ✗ Connection failed — check your homeserver URL and access token
          </span>
        )}

        <div className="flex-1" />

        <Button size="sm" onClick={handleSave} disabled={isSaving}>
          {isSaving ? "Saving…" : "Save"}
        </Button>
      </div>
    </div>
  );
}
