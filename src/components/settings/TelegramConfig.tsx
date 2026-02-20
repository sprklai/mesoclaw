/**
 * TelegramConfig — configuration panel for the Telegram channel.
 *
 * Shows a step-by-step BotFather setup guide and input fields for:
 * - Bot token (password field)
 * - Allowed chat IDs (comma-separated)
 * - Polling timeout
 *
 * Phase 7.2 implementation.
 */

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  type TelegramChannelConfig,
  useChannelStore,
} from "@/stores/channelStore";

import { useChannelConfigForm } from "./channel-config/useChannelConfigForm";

// ─── TelegramConfig ───────────────────────────────────────────────────────────

interface TelegramConfigProps {
  /** Current Telegram config from the store. */
  config: TelegramChannelConfig;
}

export function TelegramConfig({ config }: TelegramConfigProps) {
  const { updateTelegramConfig } = useChannelStore();
  const { draft, isTesting, testResult, isSaving, handleChange, handleTest, handleSave } =
    useChannelConfigForm({
      config,
      channelType: "telegram",
      updateFn: updateTelegramConfig,
      fieldTransforms: { pollingTimeoutSecs: (v) => Number(v) },
    });

  return (
    <div className="space-y-6">
      {/* BotFather setup guide */}
      <section className="rounded-lg border border-border bg-muted/30 p-4">
        <h3 className="mb-3 text-sm font-semibold">
          How to create a Telegram bot
        </h3>
        <ol className="space-y-2 text-sm text-muted-foreground">
          <li>
            <span className="font-medium text-foreground">1.</span> Open
            Telegram and search for{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              @BotFather
            </span>
            .
          </li>
          <li>
            <span className="font-medium text-foreground">2.</span> Send{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              /newbot
            </span>{" "}
            and follow the prompts to set a name and username.
          </li>
          <li>
            <span className="font-medium text-foreground">3.</span> BotFather
            will send you a token — paste it below.
          </li>
          <li>
            <span className="font-medium text-foreground">4.</span> Find your
            chat ID by messaging{" "}
            <span className="rounded bg-muted px-1 font-mono text-xs">
              @userinfobot
            </span>
            , then add it to the allowed list.
          </li>
        </ol>
      </section>

      {/* Bot token */}
      <div className="space-y-2">
        <Label htmlFor="tg-token">Bot Token</Label>
        <Input
          id="tg-token"
          type="password"
          placeholder="110201543:AAHdqTcvCH1vGWJxfSeofSAs0K5PALDsaw"
          value={draft.token}
          onChange={handleChange("token")}
          autoComplete="off"
        />
        <p className="text-xs text-muted-foreground">
          Obtained from BotFather. Stored securely in the OS keyring.
        </p>
      </div>

      {/* Allowed chat IDs */}
      <div className="space-y-2">
        <Label htmlFor="tg-allowed-ids">Allowed Chat IDs</Label>
        <Input
          id="tg-allowed-ids"
          type="text"
          placeholder="123456789, -100987654321"
          value={draft.allowedChatIds}
          onChange={handleChange("allowedChatIds")}
        />
        <p className="text-xs text-muted-foreground">
          Comma-separated Telegram chat IDs. Unknown senders are silently
          ignored. Find your ID via{" "}
          <span className="font-mono">@userinfobot</span>.
        </p>
      </div>

      {/* Polling timeout */}
      <div className="space-y-2">
        <Label htmlFor="tg-timeout">Polling Timeout (seconds)</Label>
        <Input
          id="tg-timeout"
          type="number"
          min={5}
          max={60}
          value={draft.pollingTimeoutSecs}
          onChange={handleChange("pollingTimeoutSecs")}
          className="w-32"
        />
        <p className="text-xs text-muted-foreground">
          How long to wait for new messages per poll cycle (5–60 s).
        </p>
      </div>

      {/* Actions */}
      <div className="flex items-center gap-3 pt-2">
        <Button
          variant="outline"
          size="sm"
          onClick={handleTest}
          disabled={isTesting || !draft.token}
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
