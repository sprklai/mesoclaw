import { createFileRoute } from "@tanstack/react-router";
import { Link } from "@tanstack/react-router";
import { Sparkles, Zap } from "lucide-react";
import { APP_IDENTITY } from "@/config/app-identity";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  return (
    <div className="flex h-full items-center justify-center">
      <div className="max-w-3xl text-center">
        <h1 className="text-4xl font-bold">
          Welcome to {APP_IDENTITY.productName}
        </h1>
        <p className="mt-4 text-lg text-muted-foreground">
          A production-ready foundation for building AI-powered desktop
          applications with Tauri 2, React, and TypeScript.
        </p>

        <div className="mt-12 grid gap-6 md:grid-cols-2">
          <Link
            to="/settings"
            search={{ tab: "ai" }}
            className="group rounded-lg border border-border bg-card p-6 text-left transition-colors hover:bg-accent"
          >
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-primary/10 p-2">
                <Sparkles className="h-5 w-5 text-primary" />
              </div>
              <h2 className="text-xl font-semibold">AI Integration</h2>
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
            className="group rounded-lg border border-border bg-card p-6 text-left transition-colors hover:bg-accent"
          >
            <div className="flex items-center gap-3">
              <div className="rounded-md bg-primary/10 p-2">
                <Zap className="h-5 w-5 text-primary" />
              </div>
              <h2 className="text-xl font-semibold">Skills System</h2>
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

        <div className="mt-12 space-y-4">
          <h3 className="text-lg font-semibold">Quick Start</h3>
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
  );
}
