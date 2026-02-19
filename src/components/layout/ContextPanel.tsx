import { Brain, Cpu, Wifi, WifiOff } from "@/lib/icons";
import { useContextPanelStore } from "@/stores/contextPanelStore";
import { useGatewayStore } from "@/stores/gatewayStore";
import { useLLMStore } from "@/stores/llm";

export function ContextPanel() {
  const content = useContextPanelStore((s) => s.content);
  const { config } = useLLMStore();
  const isConnected = useGatewayStore((s) => s.connected);

  return (
    <div className="flex h-full flex-col gap-4 overflow-y-auto p-4">
      {content ? (
        <div>{content}</div>
      ) : (
        <DefaultContextContent config={config} isConnected={isConnected} />
      )}
    </div>
  );
}

interface DefaultContentProps {
  config: { providerId?: string; modelId?: string } | null;
  isConnected: boolean;
}

function DefaultContextContent({ config, isConnected }: DefaultContentProps) {
  return (
    <div className="space-y-4">
      <div>
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Active Model
        </h2>
        <div className="rounded-lg border border-border bg-card p-3">
          <div className="flex items-center gap-2">
            <Cpu className="size-4 text-primary" aria-hidden />
            <div className="min-w-0">
              <p className="truncate text-sm font-medium">
                {config?.modelId ?? "No model selected"}
              </p>
              <p className="truncate text-xs text-muted-foreground">
                {config?.providerId ?? "Configure in Settings"}
              </p>
            </div>
          </div>
        </div>
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
