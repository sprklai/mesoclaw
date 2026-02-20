/**
 * SlackConfig — configuration panel for the Slack channel.
 *
 * Shows a step-by-step Slack App setup guide and input fields for:
 * - Bot token (xoxb-…, password field)
 * - App token for Socket Mode (xapp-…, password field)
 * - Allowed channel IDs (comma-separated)
 *
 * Uses Socket Mode (WebSocket) so no public HTTPS endpoint is required.
 *
 * Phase 7.4 implementation.
 */

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  type SlackChannelConfig,
  useChannelStore,
} from "@/stores/channelStore";

// ─── SlackConfig ──────────────────────────────────────────────────────────────

interface SlackConfigProps {
  /** Current Slack config from the store. */
  config: SlackChannelConfig;
}

export function SlackConfig({ config }: SlackConfigProps) {
  const { updateSlackConfig, testConnection } = useChannelStore();
  const [draft, setDraft] = useState<SlackChannelConfig>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleChange =
    (field: keyof SlackChannelConfig) =>
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setDraft((prev) => ({ ...prev, [field]: e.target.value }));
      setTestResult(null);
    };

  const handleTest = async () => {
    setIsTesting(true);
    setTestResult(null);
    const ok = await testConnection("slack");
    setTestResult(ok ? "ok" : "fail");
    setIsTesting(false);
  };

  const handleSave = async () => {
    setIsSaving(true);
    await updateSlackConfig(draft);
    setIsSaving(false);
  };

  return (
    <div className="space-y-6">
      {/* Setup guide */}
      <section className="rounded-lg border border-border bg-muted/30 p-4">
        <h3 className="mb-3 text-sm font-semibold">
          How to create a Slack app with Socket Mode
        </h3>
        <ol className="space-y-2 text-sm text-muted-foreground">
          <li>
            <span className="font-medium text-foreground">1.</span> Go to{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              api.slack.com/apps
            </span>{" "}
            → Create New App → From scratch.
          </li>
          <li>
            <span className="font-medium text-foreground">2.</span> Under{" "}
            <span className="font-medium">Socket Mode</span> → Enable Socket
            Mode → Generate an App-Level Token with{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              connections:write
            </span>{" "}
            scope → copy the{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              xapp-…
            </span>{" "}
            token.
          </li>
          <li>
            <span className="font-medium text-foreground">3.</span> Under{" "}
            <span className="font-medium">OAuth &amp; Permissions</span> → add
            Bot Token Scopes:{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              channels:history
            </span>
            ,{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              chat:write
            </span>
            ,{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              im:history
            </span>
            .
          </li>
          <li>
            <span className="font-medium text-foreground">4.</span> Install to
            Workspace → copy the Bot User OAuth Token (
            <span className="rounded bg-muted px-1 font-mono text-xs">
              xoxb-…
            </span>
            ).
          </li>
          <li>
            <span className="font-medium text-foreground">5.</span> Under{" "}
            <span className="font-medium">Event Subscriptions</span> → Subscribe
            to Bot Events:{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              message.channels
            </span>
            ,{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              message.im
            </span>
            .
          </li>
        </ol>
      </section>

      {/* Bot token */}
      <div className="space-y-2">
        <Label htmlFor="sl-bot-token">Bot Token</Label>
        <Input
          id="sl-bot-token"
          type="password"
          placeholder="xoxb-xxxxxxxxxxxx-xxxxxxxxxxxx-xxxxxxxxxxxx"
          value={draft.botToken}
          onChange={handleChange("botToken")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          Bot User OAuth Token from OAuth &amp; Permissions. Stored securely in
          the OS keyring.
        </p>
      </div>

      {/* App token */}
      <div className="space-y-2">
        <Label htmlFor="sl-app-token">App Token (Socket Mode)</Label>
        <Input
          id="sl-app-token"
          type="password"
          placeholder="xapp-1-XXXXXXXXX-0000000000000-abc…"
          value={draft.appToken}
          onChange={handleChange("appToken")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          App-Level Token for Socket Mode. Starts with{" "}
          <span className="font-mono">xapp-</span>. Stored securely in the OS
          keyring.
        </p>
      </div>

      {/* Allowed channel IDs */}
      <div className="space-y-2">
        <Label htmlFor="sl-channels">Allowed Channel IDs</Label>
        <Input
          id="sl-channels"
          type="text"
          placeholder="C01234567AB, C09876543ZZ"
          value={draft.allowedChannelIds}
          onChange={handleChange("allowedChannelIds")}
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated Slack channel IDs. Leave empty to receive messages
          from all channels. Right-click a channel in Slack → View channel
          details → Copy channel ID.
        </p>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-2">
        <Button
          variant="outline"
          size="sm"
          onClick={handleTest}
          disabled={isTesting || !draft.botToken}
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
            ✗ Connection failed — check your bot token
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
