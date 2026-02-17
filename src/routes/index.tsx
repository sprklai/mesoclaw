import { createFileRoute } from "@tanstack/react-router";
import { Link } from "@tanstack/react-router";
import { Sparkles, Zap } from "lucide-react";
import { useTranslation } from "react-i18next";
import { APP_IDENTITY } from "@/config/app-identity";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const { t } = useTranslation("common");

  return (
    /*
     * Mobile-friendly home page layout:
     * - Vertically centered on desktop, top-aligned with padding on mobile
     * - Sticky bottom input area placeholder for future chat input
     * - pb-safe ensures content is not obscured by iOS home indicator
     */
    <div className="flex h-full flex-col">
      {/* Main scrollable content */}
      <div className="flex flex-1 items-center justify-center overflow-auto">
        <div className="w-full max-w-3xl px-4 py-6 text-center sm:px-6 md:px-8">
          {/* ── Hero heading ──────────────────────────────────────── */}
          <h1 className="text-2xl font-bold sm:text-3xl md:text-4xl">
            {t("navigation.home")} — {APP_IDENTITY.productName}
          </h1>
          <p className="mt-3 text-base text-muted-foreground sm:mt-4 sm:text-lg">
            A production-ready foundation for building AI-powered desktop
            applications with Tauri 2, React, and TypeScript.
          </p>

          {/* ── Feature cards ─────────────────────────────────────── */}
          <div className="mt-8 grid gap-4 sm:mt-12 sm:gap-6 md:grid-cols-2">
            <Link
              to="/settings"
              search={{ tab: "ai" }}
              className="group rounded-lg border border-border bg-card p-5 text-left transition-colors hover:bg-accent sm:p-6"
            >
              <div className="flex items-center gap-3">
                <div className="rounded-md bg-primary/10 p-2">
                  <Sparkles className="h-5 w-5 text-primary" aria-hidden />
                </div>
                <h2 className="text-lg font-semibold sm:text-xl">
                  AI Integration
                </h2>
              </div>
              <p className="mt-3 text-sm text-muted-foreground">
                Configure LLM providers, manage API keys securely, and customize
                AI behavior for your application.
              </p>
              <div className="mt-4 text-sm font-medium text-primary group-hover:underline">
                Configure AI Settings →
              </div>
            </Link>

            <Link
              to="/settings"
              search={{ tab: "skills" }}
              className="group rounded-lg border border-border bg-card p-5 text-left transition-colors hover:bg-accent sm:p-6"
            >
              <div className="flex items-center gap-3">
                <div className="rounded-md bg-primary/10 p-2">
                  <Zap className="h-5 w-5 text-primary" aria-hidden />
                </div>
                <h2 className="text-lg font-semibold sm:text-xl">
                  Skills System
                </h2>
              </div>
              <p className="mt-3 text-sm text-muted-foreground">
                Create custom AI skills with prompts and parameters. Build
                reusable capabilities for your domain.
              </p>
              <div className="mt-4 text-sm font-medium text-primary group-hover:underline">
                Manage Skills →
              </div>
            </Link>
          </div>

          {/* ── Quick start guide ─────────────────────────────────── */}
          <div className="mt-8 space-y-4 sm:mt-12">
            <h3 className="text-base font-semibold sm:text-lg">Quick Start</h3>
            <div className="rounded-lg border border-border bg-muted/50 p-4 text-left text-sm">
              <ol className="list-inside list-decimal space-y-2 text-muted-foreground">
                <li>
                  Configure your AI provider in Settings → AI Integration
                </li>
                <li>Create a skill with a custom prompt and parameters</li>
                <li>
                  Use the skill system to build AI-powered features in your app
                </li>
                <li>Extend with custom Tauri commands and React components</li>
              </ol>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
