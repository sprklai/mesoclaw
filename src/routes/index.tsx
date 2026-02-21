import { Link, createFileRoute } from "@tanstack/react-router";
import { Brain, MessageSquare, Settings, Sparkles } from "lucide-react";
import { GatewayStatus } from "@/components/ui/gateway-status";
import { ActivityDashboard } from "@/components/dashboard";
import { APP_IDENTITY } from "@/config/app-identity";
import { useLLMStore } from "@/stores/llm";
import { cn } from "@/lib/utils";
import { PageHeader } from "@/components/layout/PageHeader";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { config } = useLLMStore();

  const hour = new Date().getHours();
  const greeting =
    hour < 12 ? "Good morning" : hour < 17 ? "Good afternoon" : "Good evening";

  const quickActions = [
    {
      icon: Sparkles,
      label: "New Chat",
      href: "/chat" as const,
      description: "Start a conversation",
    },
    {
      icon: Brain,
      label: "Memory",
      href: "/memory" as const,
      description: "Search agent memory",
    },
    {
      icon: MessageSquare,
      label: "Channels",
      href: "/channels" as const,
      description: "View Telegram inbox",
    },
    {
      icon: Settings,
      label: "Settings",
      href: "/settings" as const,
      description: "Configure providers",
    },
  ];

  return (
    <div className="flex h-full flex-col">
      <div className="shrink-0 p-6">
        <PageHeader
          title={`${greeting}, ${APP_IDENTITY.productName}`}
          description={new Date().toLocaleDateString("en-US", {
            weekday: "long",
            month: "long",
            day: "numeric",
          })}
        >
          <GatewayStatus />
        </PageHeader>
      </div>
      <div className="flex-1 overflow-auto px-6 pb-6">
        <div className="mx-auto w-full max-w-4xl space-y-8">
          {/* ── Quick actions grid ── */}
          <section aria-labelledby="quick-actions-heading">
            <h2
              id="quick-actions-heading"
              className="mb-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Quick Actions
            </h2>
            <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              {quickActions.map(({ icon: Icon, label, href, description }) => (
                <Link
                  key={href}
                  to={href}
                  className={cn(
                    "group flex flex-col gap-3 rounded-xl border border-border bg-card p-4",
                    "transition-all hover:border-primary/40 hover:shadow-sm",
                    "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  )}
                >
                  <div className="flex size-9 items-center justify-center rounded-lg bg-primary/10">
                    <Icon className="size-4 text-primary" aria-hidden />
                  </div>
                  <div>
                    <p className="text-sm font-medium leading-none">{label}</p>
                    <p className="mt-1 text-xs text-muted-foreground">
                      {description}
                    </p>
                  </div>
                </Link>
              ))}
            </div>
          </section>

          {/* ── System status ── */}
          <section aria-labelledby="status-heading">
            <h2
              id="status-heading"
              className="mb-4 text-xs font-semibold uppercase tracking-wider text-muted-foreground"
            >
              System Status
            </h2>
            <div className="rounded-xl border border-border bg-card p-4">
              <div className="flex flex-wrap gap-6">
                <StatusRow
                  label="AI Provider"
                  value={config?.providerId ?? "Not configured"}
                  ok={!!config?.providerId}
                />
                <StatusRow
                  label="Model"
                  value={config?.modelId ?? "Not selected"}
                  ok={!!config?.modelId}
                />
              </div>
            </div>
          </section>

          {/* ── Activity dashboard ── */}
          <ActivityDashboard />
        </div>
      </div>
    </div>
  );
}

function StatusRow({
  label,
  value,
  ok,
}: {
  label: string;
  value: string;
  ok: boolean;
}) {
  return (
    <div className="flex items-center gap-2">
      <div
        className={cn(
          "size-2 rounded-full",
          ok ? "bg-green-500" : "bg-muted-foreground"
        )}
        aria-hidden
      />
      <span className="text-sm text-muted-foreground">{label}:</span>
      <span className="text-sm font-medium">{value}</span>
    </div>
  );
}
