import ReactMarkdown from "react-markdown";

import { HelpCircle } from "@/lib/icons";

interface ContextualHelpPanelProps {
  title: string;
  content: string;
  className?: string;
}

export function ContextualHelpPanel({
  title,
  content,
  className = "",
}: ContextualHelpPanelProps) {
  return (
    <div
      className={`rounded-lg border border-border bg-muted/30 p-4 ${className}`}
    >
      <div className="mb-3 flex items-center gap-2">
        <HelpCircle className="h-4 w-4 text-muted-foreground" />
        <h3 className="text-sm font-semibold">{title}</h3>
      </div>
      <div className="prose prose-sm dark:prose-invert max-w-none text-muted-foreground">
        <ReactMarkdown>{content}</ReactMarkdown>
      </div>
    </div>
  );
}
