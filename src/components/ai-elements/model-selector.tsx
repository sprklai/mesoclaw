import type { ComponentProps, ReactNode } from "react";

import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from "@/components/ui/command";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import {
  Anthropic,
  DeepSeek,
  Gemini,
  Google,
  Groq,
  Mistral,
  OpenAI,
  Together,
  VertexAI,
  XAI,
  OllamaIcon,
  Claude,
  OpenRouter,
  Vercel,
  Cpu as LucideCpu,
  Sparkles,
} from "@/lib/icons";
import { cn } from "@/lib/utils";

export type ModelSelectorProps = ComponentProps<typeof Command>;

export const ModelSelector = (props: ModelSelectorProps) => (
  <Command {...props} />
);

export type ModelSelectorTriggerProps = ComponentProps<typeof DialogTrigger>;

export const ModelSelectorTrigger = (props: ModelSelectorTriggerProps) => (
  <DialogTrigger {...props} />
);

export type ModelSelectorContentProps = ComponentProps<typeof DialogContent> & {
  title?: ReactNode;
};

export const ModelSelectorContent = ({
  className,
  children,
  title = "Model Selector",
  ...props
}: ModelSelectorContentProps) => (
  <DialogContent
    className={cn(
      "overflow-hidden p-0 shadow-xl",
      // Fix blur and improve rendering
      "will-change-transform",
      // Dynamic sizing
      "w-full min-w-[320px] max-w-[600px]",
      // Better dark mode visibility
      "bg-popover text-popover-foreground",
      className
    )}
    {...props}
  >
    <DialogTitle className="sr-only">{title}</DialogTitle>
    {children}
  </DialogContent>
);

export type ModelSelectorDialogProps = ComponentProps<typeof Dialog>;

export const ModelSelectorDialog = (props: ModelSelectorDialogProps) => (
  <Dialog {...props} />
);

export type ModelSelectorInputProps = ComponentProps<typeof CommandInput>;

export const ModelSelectorInput = ({
  className,
  ...props
}: ModelSelectorInputProps) => (
  <CommandInput className={cn("border-0 focus:ring-0", className)} {...props} />
);

export type ModelSelectorListProps = ComponentProps<typeof CommandList>;

export const ModelSelectorList = (props: ModelSelectorListProps) => (
  <CommandList {...props} />
);

export type ModelSelectorEmptyProps = ComponentProps<typeof CommandEmpty>;

export const ModelSelectorEmpty = (props: ModelSelectorEmptyProps) => (
  <CommandEmpty {...props} />
);

export type ModelSelectorGroupProps = ComponentProps<typeof CommandGroup>;

export const ModelSelectorGroup = ({
  className,
  ...props
}: ModelSelectorGroupProps) => (
  <CommandGroup
    className={cn(
      "**:[[cmdk-group-heading]]:px-3 **:[[cmdk-group-heading]]:py-2 **:[[cmdk-group-heading]]:text-sm **:[[cmdk-group-heading]]:font-semibold **:[[cmdk-group-heading]]:text-foreground",
      className
    )}
    {...props}
  />
);

export type ModelSelectorItemProps = ComponentProps<typeof CommandItem>;

export const ModelSelectorItem = ({
  className,
  ...props
}: ModelSelectorItemProps) => (
  <CommandItem
    className={cn("px-3 py-2.5", "focus:bg-accent", className)}
    {...props}
  />
);

export type ModelSelectorShortcutProps = ComponentProps<typeof CommandShortcut>;

export const ModelSelectorShortcut = (props: ModelSelectorShortcutProps) => (
  <CommandShortcut {...props} />
);

export type ModelSelectorSeparatorProps = ComponentProps<
  typeof CommandSeparator
>;

export const ModelSelectorSeparator = (props: ModelSelectorSeparatorProps) => (
  <CommandSeparator {...props} />
);

export type ModelSelectorLogoProps = Omit<
  ComponentProps<"img">,
  "src" | "alt"
> & {
  provider?:
    | "moonshotai-cn"
    | "lucidquery"
    | "moonshotai"
    | "zai-coding-plan"
    | "alibaba"
    | "xai"
    | "vultr"
    | "nvidia"
    | "upstage"
    | "groq"
    | "github-copilot"
    | "mistral"
    | "vercel"
    | "nebius"
    | "deepseek"
    | "alibaba-cn"
    | "google-vertex-anthropic"
    | "venice"
    | "chutes"
    | "cortecs"
    | "github-models"
    | "togetherai"
    | "azure"
    | "baseten"
    | "huggingface"
    | "opencode"
    | "fastrouter"
    | "google"
    | "google-vertex"
    | "cloudflare-workers-ai"
    | "inception"
    | "wandb"
    | "openai"
    | "zhipuai-coding-plan"
    | "perplexity"
    | "openrouter"
    | "zenmux"
    | "v0"
    | "iflowcn"
    | "synthetic"
    | "deepinfra"
    | "zhipuai"
    | "submodel"
    | "zai"
    | "inference"
    | "requesty"
    | "morph"
    | "lmstudio"
    | "anthropic"
    | "aihubmix"
    | "fireworks-ai"
    | "modelscope"
    | "llama"
    | "scaleway"
    | "amazon-bedrock"
    | "cerebras"
    | (string & {});
};

export const ModelSelectorLogo = ({
  provider,
  className,
  style,
  ...props
}: ModelSelectorLogoProps) => {
  // Handle undefined or empty provider
  if (!provider) {
    return (
      <div
        className={cn(
          "flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-muted/50",
          className
        )}
        style={style}
      >
        <Sparkles className="h-4 w-4 text-muted-foreground" />
      </div>
    );
  }

  // Map provider IDs to Lobe icon components
  const iconMap: Record<
    string,
    React.ComponentType<{ className?: string; style?: React.CSSProperties }>
  > = {
    // AI Gateway providers
    "vercel-ai-gateway": Vercel,
    openrouter: OpenRouter, // fallback

    // Direct providers
    vercel: Vercel,
    ollama: OllamaIcon,
    openai: OpenAI,
    anthropic: Claude.Color,
    gemini: Gemini.Color,
    google: Google,
    googlevertex: VertexAI,
    vertex: VertexAI,
    xai: XAI,
    groq: Groq,
    deepseek: DeepSeek,
    together: Together,
    mistral: Mistral,
    cohere: Anthropic, // fallback

    // Local providers
    lmstudio: LucideCpu,
  };

  // Normalize provider ID for matching
  const normalizedProvider = provider.toLowerCase().replace(/[_-]/g, "");

  // For local providers like ollama, lmstudio - use special styling
  if (provider === "ollama" || provider === "lmstudio") {
    const IconComponent = provider === "ollama" ? OllamaIcon : LucideCpu;
    return (
      <div
        className={cn(
          "flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-muted/50 p-1.5",
          className
        )}
        style={style}
      >
        <IconComponent className="h-full w-full text-foreground" />
      </div>
    );
  }

  // Try exact match first
  let IconComponent:
    | React.ComponentType<{ className?: string; style?: React.CSSProperties }>
    | undefined = iconMap[provider.toLowerCase()];

  // Try normalized match
  if (!IconComponent) {
    const entry = Object.entries(iconMap).find(
      ([key]) => key.toLowerCase().replace(/[_-]/g, "") === normalizedProvider
    );
    IconComponent = entry?.[1];
  }

  if (IconComponent) {
    return (
      <div
        className={cn(
          "flex h-5 w-5 shrink-0 items-center justify-center",
          className
        )}
        style={style}
      >
        <IconComponent className="h-full w-full" />
      </div>
    );
  }

  // Check if this is likely a user-defined provider (not in our known providers list)
  // User-defined providers won't have logos on models.dev
  const knownProviders = [
    "moonshotai-cn",
    "lucidquery",
    "moonshotai",
    "zai-coding-plan",
    "alibaba",
    "xai",
    "vultr",
    "nvidia",
    "upstage",
    "groq",
    "github-copilot",
    "mistral",
    "vercel",
    "nebius",
    "deepseek",
    "alibaba-cn",
    "google-vertex-anthropic",
    "venice",
    "chutes",
    "cortecs",
    "github-models",
    "togetherai",
    "azure",
    "baseten",
    "huggingface",
    "opencode",
    "fastrouter",
    "google",
    "google-vertex",
    "cloudflare-workers-ai",
    "inception",
    "wandb",
    "openai",
    "zhipuai-coding-plan",
    "perplexity",
    "openrouter",
    "zenmux",
    "v0",
    "iflowcn",
    "synthetic",
    "deepinfra",
    "zhipuai",
    "submodel",
    "zai",
    "inference",
    "requesty",
    "morph",
    "lmstudio",
    "anthropic",
    "aihubmix",
    "fireworks-ai",
    "modelscope",
    "llama",
    "scaleway",
    "amazon-bedrock",
    "cerebras",
    "vercel-ai-gateway",
    "ollama",
    "gemini",
  ];

  const isKnownProvider = knownProviders.some(
    (known) => known.toLowerCase().replace(/[_-]/g, "") === normalizedProvider
  );

  // For unknown (user-defined) providers, use a generic custom icon
  if (!isKnownProvider) {
    return (
      <div
        className={cn(
          "flex h-6 w-6 shrink-0 items-center justify-center rounded-md bg-primary/10 p-1.5",
          className
        )}
        style={style}
      >
        <Sparkles className="h-full w-full text-primary" />
      </div>
    );
  }

  // Fallback to external image for known providers
  return (
    <img
      src={`https://models.dev/logos/${provider}.svg`}
      alt={provider}
      className={cn("h-5 w-5 shrink-0", className)}
      style={style}
      loading="lazy"
      {...props}
    />
  );
};

export type ModelSelectorLogoGroupProps = ComponentProps<"div">;

export const ModelSelectorLogoGroup = ({
  className,
  ...props
}: ModelSelectorLogoGroupProps) => (
  <div
    className={cn(
      "flex gap-1 [&>img]:rounded-full [&>img]:bg-background [&>img]:p-px [&>img]:ring-1 dark:[&>img]:bg-muted",
      className
    )}
    {...props}
  />
);

export type ModelSelectorNameProps = ComponentProps<"span">;

export const ModelSelectorName = ({
  className,
  ...props
}: ModelSelectorNameProps) => (
  <span
    className={cn("text-sm font-medium text-foreground", className)}
    {...props}
  />
);
