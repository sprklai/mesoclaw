/**
 * DiscordConfig — configuration panel for the Discord channel.
 *
 * Shows a step-by-step Discord Developer Portal setup guide and input fields for:
 * - Bot token (password field)
 * - Allowed guild (server) IDs (comma-separated)
 * - Allowed channel IDs (comma-separated)
 *
 * Phase 7.2 implementation.
 */

import { useState } from "react";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  type DiscordChannelConfig,
  useChannelStore,
} from "@/stores/channelStore";

// ─── DiscordConfig ────────────────────────────────────────────────────────────

interface DiscordConfigProps {
  /** Current Discord config from the store. */
  config: DiscordChannelConfig;
}

export function DiscordConfig({ config }: DiscordConfigProps) {
  const { updateDiscordConfig, testConnection } = useChannelStore();
  const [draft, setDraft] = useState<DiscordChannelConfig>(config);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<"ok" | "fail" | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const handleChange =
    (field: keyof DiscordChannelConfig) =>
    (e: React.ChangeEvent<HTMLInputElement>) => {
      setDraft((prev) => ({ ...prev, [field]: e.target.value }));
      setTestResult(null);
    };

  const handleTest = async () => {
    setIsTesting(true);
    setTestResult(null);
    const ok = await testConnection("discord");
    setTestResult(ok ? "ok" : "fail");
    setIsTesting(false);
  };

  const handleSave = async () => {
    setIsSaving(true);
    await updateDiscordConfig(draft);
    setIsSaving(false);
  };

  return (
    <div className="space-y-6">
      {/* Developer Portal setup guide */}
      <section className="rounded-lg border border-border bg-muted/30 p-4">
        <h3 className="mb-3 text-sm font-semibold">
          How to create a Discord bot
        </h3>
        <ol className="space-y-2 text-sm text-muted-foreground">
          <li>
            <span className="font-medium text-foreground">1.</span> Go to{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              discord.com/developers/applications
            </span>{" "}
            → New Application.
          </li>
          <li>
            <span className="font-medium text-foreground">2.</span> Under{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              Bot
            </span>{" "}
            → copy the Token. Enable{" "}
            <span className="font-medium">Message Content Intent</span> under
            Privileged Gateway Intents.
          </li>
          <li>
            <span className="font-medium text-foreground">3.</span> Use OAuth2
            URL Generator with{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              bot
            </span>{" "}
            scope and{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              Send Messages
            </span>{" "}
            permission to invite the bot to your server.
          </li>
          <li>
            <span className="font-medium text-foreground">4.</span> Right-click
            a server or channel in Discord (Developer Mode must be on) to copy
            its ID for the allow-lists below.
          </li>
        </ol>
      </section>

      {/* Bot token */}
      <div className="space-y-2">
        <Label htmlFor="dc-token">Bot Token</Label>
        <Input
          id="dc-token"
          type="password"
          placeholder="MTExMjM0NTY3ODkwMTIzNDU2.Gh1234.abc…"
          value={draft.botToken}
          onChange={handleChange("botToken")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          Obtained from the Discord Developer Portal. Stored securely in the OS
          keyring.
        </p>
      </div>

      {/* Allowed guild IDs */}
      <div className="space-y-2">
        <Label htmlFor="dc-guilds">Allowed Server (Guild) IDs</Label>
        <Input
          id="dc-guilds"
          type="text"
          placeholder="123456789012345678, 987654321098765432"
          value={draft.allowedGuildIds}
          onChange={handleChange("allowedGuildIds")}
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated Discord server IDs. Leave empty to accept messages
          from all servers. Enable Developer Mode in Discord to copy IDs.
        </p>
      </div>

      {/* Allowed channel IDs */}
      <div className="space-y-2">
        <Label htmlFor="dc-channels">Allowed Channel IDs</Label>
        <Input
          id="dc-channels"
          type="text"
          placeholder="111222333444555666, 666555444333222111"
          value={draft.allowedChannelIds}
          onChange={handleChange("allowedChannelIds")}
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated Discord channel IDs. Leave empty to accept messages
          from all channels.
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
            ✗ Connection failed — check your token
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
