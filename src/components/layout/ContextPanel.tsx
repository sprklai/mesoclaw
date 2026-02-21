import { useState } from "react";
import { Brain, Cpu, Wifi, WifiOff } from "@/lib/icons";
import { useContextPanelStore } from "@/stores/contextPanelStore";
import { useGatewayStore } from "@/stores/gatewayStore";
import { useLLMStore } from "@/stores/llm";
import { AIModelQuickAccess } from "@/components/ai/AIModelQuickAccess";

export function ContextPanel() {
  const content = useContextPanelStore((s) => s.content);
  const { config, providersWithModels } = useLLMStore();
  const isConnected = useGatewayStore((s) => s.connected);

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
      {content ? (
        <div>{content}</div>
      ) : (
        <DefaultContextContent
          config={config}
          providersWithModels={providersWithModels}
          isConnected={isConnected}
        />
      )}
    </div>
  );
}

interface DefaultContentProps {
  config: { providerId?: string; modelId?: string } | null;
  providersWithModels: Array<{
    id: string;
    name: string;
    models: Array<{ modelId: string; displayName: string }>;
  }>;
  isConnected: boolean;
}

function DefaultContextContent({
  config,
  providersWithModels,
  isConnected,
}: DefaultContentProps) {
  const [modelSelectorOpen, setModelSelectorOpen] = useState(false);

  // Find the display name for the current model
  const modelInfo = (() => {
    if (!config?.providerId || !config?.modelId) return null;
    const provider = providersWithModels.find((p) => p.id === config.providerId);
    if (!provider) return null;
    const model = provider.models.find((m) => m.modelId === config.modelId);
    if (!model) return null;
    return { providerName: provider.name, displayName: model.displayName };
  })();

  return (
    <div className="space-y-4">
      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Active Model
        </h2>
        <button
          type="button"
          onClick={() => setModelSelectorOpen(true)}
          className="w-full rounded-lg border border-border bg-card p-3 text-left transition-colors hover:border-primary/50 hover:bg-accent/50"
        >
          <div className="flex items-center gap-2">
            <Cpu className="size-4 text-primary shrink-0" aria-hidden />
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium">
                {modelInfo?.displayName ?? config?.modelId ?? "No model selected"}
              </p>
              <p className="truncate text-xs text-muted-foreground">
                {modelInfo?.providerName ?? config?.providerId ?? "Configure in Settings"}
              </p>
            </div>
            <span className="text-xs text-muted-foreground shrink-0">Change</span>
          </div>
        </button>
        <AIModelQuickAccess
          open={modelSelectorOpen}
          onOpenChange={setModelSelectorOpen}
          mode="global-default"
          filterAvailable
        />
      </div>

      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Gateway
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            {isConnected ? (
              <Wifi className="size-4 text-green-600" aria-hidden />
            ) : (
              <WifiOff className="size-4 text-muted-foreground" aria-hidden />
            )}
            <span className="text-sm font-medium">
              {isConnected ? "Connected" : "Disconnected"}
            </span>
          </div>
        </div>
      </div>

      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Memory
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            <Brain className="size-4 text-primary" aria-hidden />
            <span className="text-sm text-muted-foreground">
              Available in Memory tab
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
